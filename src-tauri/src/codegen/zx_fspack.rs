//! ZX FSPK binary export adapter
//!
//! Exports character data to the FSPK binary format for use with ZX runtime.

use crate::commands::CharacterData;
use crate::schema::{FrameHitbox, GuardType, Rect, State};
use std::collections::HashMap;

use super::zx_fspack_format::{
    to_q12_4, to_q12_4_unsigned, write_u16_le, write_u32_le, write_u8, FLAGS_RESERVED, HEADER_SIZE,
    HIT_WINDOW24_SIZE, HURT_WINDOW12_SIZE, KEY_NONE, MAGIC, SECTION_CANCELS_U16,
    SECTION_CANCEL_DENIES, SECTION_CANCEL_TAG_RULES, SECTION_EVENT_ARGS, SECTION_EVENT_EMITS,
    SECTION_HEADER_SIZE, SECTION_HIT_WINDOWS, SECTION_HURT_WINDOWS, SECTION_KEYFRAMES_KEYS,
    SECTION_MESH_KEYS, SECTION_MOVE_NOTIFIES, SECTION_MOVE_RESOURCE_COSTS,
    SECTION_MOVE_RESOURCE_DELTAS, SECTION_MOVE_RESOURCE_PRECONDITIONS, SECTION_RESOURCE_DEFS,
    SECTION_SHAPES, SECTION_STATES, SECTION_STATE_EXTRAS, SECTION_STATE_TAGS,
    SECTION_STATE_TAG_RANGES, SECTION_STRING_TABLE, SHAPE12_SIZE, SHAPE_KIND_AABB,
    STATE_EXTRAS72_SIZE, STATE_RECORD_SIZE, STRREF_SIZE,
};

fn checked_u16(value: usize, what: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{} overflows u16: {}", what, value))
}

fn checked_u32(value: usize, what: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{} overflows u32: {}", what, value))
}

fn align_up(value: usize, align: u32) -> Result<usize, String> {
    if align == 0 {
        return Err("alignment must be non-zero".to_string());
    }
    if !align.is_power_of_two() {
        return Err(format!("alignment must be power of two, got {}", align));
    }

    let align = align as usize;
    if align == 1 {
        return Ok(value);
    }

    let mask = align - 1;
    let v = value
        .checked_add(mask)
        .ok_or_else(|| "align_up overflow".to_string())?;
    Ok(v & !mask)
}

// =============================================================================
// String Table
// =============================================================================

/// Interned string table for deduplication.
///
/// Strings are stored as raw UTF-8 bytes. The `index` map stores (offset, length)
/// pairs for each unique string that has been interned.
pub struct StringTable {
    data: Vec<u8>,
    /// Map from string to (offset, length) in data
    index: HashMap<String, (u32, u16)>,
}

impl StringTable {
    /// Create a new empty string table.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Intern a string, returning its (offset, length) in the table.
    ///
    /// If the string was already interned, returns the existing location.
    /// Otherwise, appends the string to the data and records its location.
    pub fn intern(&mut self, s: &str) -> Result<(u32, u16), String> {
        if let Some(&loc) = self.index.get(s) {
            return Ok(loc);
        }

        let offset = checked_u32(self.data.len(), "string table offset")?;
        let len = checked_u16(s.len(), "string table string length")?;
        self.data.extend_from_slice(s.as_bytes());
        self.index.insert(s.to_string(), (offset, len));
        Ok((offset, len))
    }

    /// Consume the string table and return the raw byte data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Get the current byte length of the string table data.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the string table is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Move Packing
// =============================================================================

/// Pack a Rect into a Shape12 (AABB) structure.
///
/// Shape12 layout:
/// - kind (u8): shape type (0 = AABB)
/// - flags (u8): reserved
/// - a (i16): x position (Q12.4)
/// - b (i16): y position (Q12.4)
/// - c (u16): width (Q12.4 unsigned)
/// - d (u16): height (Q12.4 unsigned)
/// - e (i16): unused for AABB
fn pack_shape(rect: &Rect) -> [u8; SHAPE12_SIZE] {
    let mut buf = [0u8; SHAPE12_SIZE];
    buf[0] = SHAPE_KIND_AABB; // kind
    buf[1] = 0; // flags

    let x = to_q12_4(rect.x as f32);
    let y = to_q12_4(rect.y as f32);
    let w = to_q12_4_unsigned(rect.w as f32);
    let h = to_q12_4_unsigned(rect.h as f32);

    buf[2..4].copy_from_slice(&x.to_le_bytes()); // a = x
    buf[4..6].copy_from_slice(&y.to_le_bytes()); // b = y
    buf[6..8].copy_from_slice(&w.to_le_bytes()); // c = w
    buf[8..10].copy_from_slice(&h.to_le_bytes()); // d = h
    buf[10..12].copy_from_slice(&0i16.to_le_bytes()); // e = 0

    buf
}

/// Convert GuardType to u8 for binary encoding.
fn guard_type_to_u8(guard: &GuardType) -> u8 {
    match guard {
        GuardType::High => 0,
        GuardType::Mid => 1,
        GuardType::Low => 2,
        GuardType::Unblockable => 3,
    }
}

/// Pack a FrameHitbox into a HitWindow24 structure.
///
/// HitWindow24 layout (24 bytes) - must match view.rs HitWindowView:
/// - 0: start_frame (u8)
/// - 1: end_frame (u8)
/// - 2: guard (u8)
/// - 3: reserved (u8)
/// - 4-5: damage (u16 LE)
/// - 6-7: chip_damage (u16 LE)
/// - 8: hitstun (u8)
/// - 9: blockstun (u8)
/// - 10: hitstop (u8)
/// - 11: reserved (u8)
/// - 12-15: shapes_off (u32 LE)
/// - 16-17: shapes_len (u16 LE)
/// - 18-21: cancels_off (u32 LE)
/// - 22-23: cancels_len (u16 LE)
fn pack_hit_window(
    hb: &FrameHitbox,
    shapes_off: u32,
    damage: u16,
    hitstun: u8,
    blockstun: u8,
    hitstop: u8,
    guard: u8,
) -> [u8; HIT_WINDOW24_SIZE] {
    let mut buf = [0u8; HIT_WINDOW24_SIZE];

    buf[0] = hb.frames.0; // start_frame
    buf[1] = hb.frames.1; // end_frame
    buf[2] = guard; // guard
    buf[3] = 0; // reserved
    buf[4..6].copy_from_slice(&damage.to_le_bytes()); // damage
    buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // chip_damage (TODO: add to schema)
    buf[8] = hitstun; // hitstun
    buf[9] = blockstun; // blockstun
    buf[10] = hitstop; // hitstop
    buf[11] = 0; // reserved
    buf[12..16].copy_from_slice(&shapes_off.to_le_bytes()); // shapes_off
    buf[16..18].copy_from_slice(&1u16.to_le_bytes()); // shapes_len = 1
    // bytes 18-27 are cancels/pushback (already zeroed, not used in v1)

    buf
}

/// Pack a FrameHitbox into a HurtWindow12 structure.
///
/// HurtWindow12 layout (12 bytes):
/// - frame_start (u8): first active frame
/// - frame_end (u8): last active frame
/// - shape_off (u32): offset into SHAPES section
/// - shape_count (u16): number of shapes (always 1 for v1)
/// - flags (u16): hurtbox flags (invuln, armor, etc.)
/// - reserved (2 bytes): padding
fn pack_hurt_window(hb: &FrameHitbox, shapes_off: u32) -> [u8; HURT_WINDOW12_SIZE] {
    let mut buf = [0u8; HURT_WINDOW12_SIZE];

    buf[0] = hb.frames.0; // frame_start
    buf[1] = hb.frames.1; // frame_end
    buf[2..6].copy_from_slice(&shapes_off.to_le_bytes()); // shape_off
    buf[6..8].copy_from_slice(&1u16.to_le_bytes()); // shape_count = 1
    buf[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags = 0 for v1
                                                     // bytes 10-11 are reserved/padding (already zeroed)

    buf
}

/// Convert move type string to u8 for binary encoding.
/// Maps common type strings to fixed IDs for runtime compatibility.
fn move_type_to_u8(move_type: Option<&String>) -> u8 {
    match move_type.map(|s| s.as_str()) {
        Some("normal") => 0,
        Some("command_normal") => 1,
        Some("special") => 2,
        Some("super") => 3,
        Some("movement") => 4,
        Some("throw") => 5,
        Some("ex") => 6,
        Some("rekka") => 7,
        Some(_) => 255, // unknown custom type
        None => 0,      // default to normal
    }
}

/// Convert TriggerType to u8 for binary encoding.
fn trigger_type_to_u8(trigger: Option<&crate::schema::TriggerType>) -> u8 {
    use crate::schema::TriggerType;
    match trigger {
        Some(TriggerType::Press) => 0,
        Some(TriggerType::Release) => 1,
        Some(TriggerType::Hold) => 2,
        None => 0, // default to Press
    }
}

/// Pack a Move into a MoveRecord structure.
///
/// MoveRecord layout (32 bytes):
/// - 0-1: move_id (u16)
/// - 2-3: mesh_key (u16)
/// - 4-5: keyframes_key (u16)
/// - 6: move_type (u8)
/// - 7: trigger (u8)
/// - 8: guard (u8)
/// - 9: flags (u8)
/// - 10: startup (u8)
/// - 11: active (u8)
/// - 12: recovery (u8)
/// - 13: reserved (u8)
/// - 14-15: total (u16)
/// - 16-17: damage (u16)
/// - 18: hitstun (u8)
/// - 19: blockstun (u8)
/// - 20: hitstop (u8)
/// - 21: reserved (u8)
/// - 22-25: hit_windows_off (u32)
/// - 26-27: hit_windows_len (u16)
/// - 28-29: hurt_windows_off (u16)
/// - 30-31: hurt_windows_len (u16)
#[allow(clippy::too_many_arguments)] // Binary record packing requires all fields
fn pack_move_record(
    move_id: u16,
    mesh_key: u16,
    keyframes_key: u16,
    mv: &State,
    hit_windows_off: u32,
    hit_windows_len: u16,
    hurt_windows_off: u16,
    hurt_windows_len: u16,
    flags: u8,
) -> [u8; STATE_RECORD_SIZE] {
    let mut buf = [0u8; STATE_RECORD_SIZE];

    buf[0..2].copy_from_slice(&move_id.to_le_bytes()); // move_id
    buf[2..4].copy_from_slice(&mesh_key.to_le_bytes()); // mesh_key
    buf[4..6].copy_from_slice(&keyframes_key.to_le_bytes()); // keyframes_key
    buf[6] = move_type_to_u8(mv.move_type.as_ref()); // move_type
    buf[7] = trigger_type_to_u8(mv.trigger.as_ref()); // trigger
    buf[8] = guard_type_to_u8(&mv.guard); // guard
    buf[9] = flags; // cancel flags
    buf[10] = mv.startup; // startup
    buf[11] = mv.active; // active
    buf[12] = mv.recovery; // recovery
    buf[13] = 0; // reserved
    let total = mv
        .total
        .map(|t| t as u16)
        .unwrap_or_else(|| (mv.startup as u16) + (mv.active as u16) + (mv.recovery as u16));
    buf[14..16].copy_from_slice(&total.to_le_bytes()); // total
    buf[16..18].copy_from_slice(&mv.damage.to_le_bytes()); // damage
    buf[18] = mv.hitstun; // hitstun
    buf[19] = mv.blockstun; // blockstun
    buf[20] = mv.hitstop; // hitstop
    buf[21] = 0; // reserved
    buf[22..26].copy_from_slice(&hit_windows_off.to_le_bytes()); // hit_windows_off
    buf[26..28].copy_from_slice(&hit_windows_len.to_le_bytes()); // hit_windows_len
    buf[28..30].copy_from_slice(&hurt_windows_off.to_le_bytes()); // hurt_windows_off (u16)
    buf[30..32].copy_from_slice(&hurt_windows_len.to_le_bytes()); // hurt_windows_len

    buf
}

/// Cancel lookup data for export.
///
/// Contains HashSets for each cancel type, keyed by move input notation.
pub struct CancelLookup<'a> {
    /// Moves that have chain cancel routes (keys in chains HashMap)
    pub chains: std::collections::HashSet<&'a str>,
    /// Chain routes: maps source move input -> list of target move inputs
    pub chain_routes: &'a std::collections::HashMap<String, Vec<String>>,
    /// Moves that can cancel into specials
    pub specials: std::collections::HashSet<&'a str>,
    /// Moves that can cancel into supers
    pub supers: std::collections::HashSet<&'a str>,
    /// Moves that can cancel into jump
    pub jumps: std::collections::HashSet<&'a str>,
    /// Maps move input notation -> move index (for resolving chain targets)
    pub input_to_index: HashMap<&'a str, u16>,
}

/// Packed move data with backing arrays.
pub struct PackedMoveData {
    /// MOVES section: array of MoveRecord (32 bytes each)
    pub moves: Vec<u8>,
    /// SHAPES section: array of Shape12 (12 bytes each)
    pub shapes: Vec<u8>,
    /// HIT_WINDOWS section: array of HitWindow24 (24 bytes each)
    pub hit_windows: Vec<u8>,
    /// HURT_WINDOWS section: array of HurtWindow12 (12 bytes each)
    pub hurt_windows: Vec<u8>,
    /// CANCELS_U16 section: array of u16 move IDs
    pub cancels: Vec<u8>,
    /// Per-move cancel info: (byte_offset, count) into CANCELS_U16 section
    pub cancel_info: Vec<(u32, u16)>,
}

/// Pack all moves into binary sections.
///
/// Returns packed move data with all backing arrays.
///
/// The `anim_to_index` map provides indices into the MESH_KEYS/KEYFRAMES_KEYS arrays
/// for each animation name. If None, all moves use KEY_NONE for asset references.
///
/// The `cancel_lookup` provides cancel information for setting MoveRecord.flags.
/// If None, all flags are 0.
pub fn pack_moves(
    moves: &[State],
    anim_to_index: Option<&HashMap<String, u16>>,
    cancel_lookup: Option<&CancelLookup>,
) -> Result<PackedMoveData, String> {
    let mut packed = PackedMoveData {
        moves: Vec::new(),
        shapes: Vec::new(),
        hit_windows: Vec::new(),
        hurt_windows: Vec::new(),
        cancels: Vec::new(),
        cancel_info: Vec::with_capacity(moves.len()),
    };

    for (idx, mv) in moves.iter().enumerate() {
        let move_id = checked_u16(idx, "move_id")?;

        // Look up animation index if map is provided
        let anim_index = anim_to_index
            .and_then(|map| {
                if mv.animation.is_empty() {
                    None
                } else {
                    map.get(&mv.animation).copied()
                }
            })
            .unwrap_or(KEY_NONE);

        // Track offsets before adding this move's data
        let hit_windows_off = checked_u32(packed.hit_windows.len(), "hit_windows_off")?;
        let hurt_windows_off = checked_u16(packed.hurt_windows.len(), "hurt_windows_off")?; // u16 for compact layout

        // Pack hitboxes -> shapes + hit_windows
        for hb in &mv.hitboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed.hit_windows.extend_from_slice(&pack_hit_window(
                hb,
                shape_off,
                mv.damage,
                mv.hitstun,
                mv.blockstun,
                mv.hitstop,
                guard_type_to_u8(&mv.guard),
            ));
        }

        // Pack hurtboxes -> shapes + hurt_windows
        for hb in &mv.hurtboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed
                .hurt_windows
                .extend_from_slice(&pack_hurt_window(hb, shape_off));
        }

        // Calculate lengths
        let hit_windows_len = checked_u16(mv.hitboxes.len(), "hit_windows_len")?;
        let hurt_windows_len = checked_u16(mv.hurtboxes.len(), "hurt_windows_len")?;

        // Compute cancel flags and pack chain cancel routes
        let mut flags: u8 = 0;
        let cancels_off = checked_u32(packed.cancels.len(), "cancels_off")?;
        let mut cancels_len: u16 = 0;

        if let Some(lookup) = cancel_lookup {
            let input = mv.input.as_str();
            if lookup.chains.contains(input) {
                flags |= super::zx_fspack_format::CANCEL_FLAG_CHAIN;
            }
            if lookup.specials.contains(input) {
                flags |= super::zx_fspack_format::CANCEL_FLAG_SPECIAL;
            }
            if lookup.supers.contains(input) {
                flags |= super::zx_fspack_format::CANCEL_FLAG_SUPER;
            }
            if lookup.jumps.contains(input) {
                flags |= super::zx_fspack_format::CANCEL_FLAG_JUMP;
            }

            // Pack chain cancel routes into CANCELS_U16 section
            if let Some(targets) = lookup.chain_routes.get(input) {
                for target_input in targets {
                    if let Some(&target_idx) = lookup.input_to_index.get(target_input.as_str()) {
                        packed.cancels.extend_from_slice(&target_idx.to_le_bytes());
                        cancels_len = cancels_len
                            .checked_add(1)
                            .ok_or_else(|| "cancel count overflow".to_string())?;
                    }
                }
            }
        }

        // Track cancel offset/length for this move
        packed.cancel_info.push((cancels_off, cancels_len));

        // Pack move record - mesh_key and keyframes_key both use the same animation index
        packed.moves.extend_from_slice(&pack_move_record(
            move_id,
            anim_index, // mesh_key
            anim_index, // keyframes_key
            mv,
            hit_windows_off,
            hit_windows_len,
            hurt_windows_off,
            hurt_windows_len,
            flags,
        ));
    }

    Ok(packed)
}

// =============================================================================
// Asset Key Building
// =============================================================================

/// A string reference as (offset, length) pair into the string table.
pub type StrRef = (u32, u16);

/// Build asset key arrays from character data.
///
/// Returns two vectors of string references:
/// - `mesh_keys`: Keys for mesh assets, format: "{character_id}.{animation}"
/// - `keyframes_keys`: Keys for keyframes assets, format: "{animation}"
///
/// Keys are sorted deterministically by their string value to ensure
/// reproducible output. Duplicate animations are deduplicated.
pub fn build_asset_keys(
    char_data: &CharacterData,
    strings: &mut StringTable,
) -> Result<(Vec<StrRef>, Vec<StrRef>), String> {
    // Collect unique animation names
    let mut animations: Vec<&str> = char_data
        .moves
        .iter()
        .filter(|m| !m.animation.is_empty())
        .map(|m| m.animation.as_str())
        .collect();

    // Deduplicate and sort for determinism
    animations.sort();
    animations.dedup();

    let character_id = &char_data.character.id;

    // Build mesh keys: "{character_id}.{animation}"
    let mesh_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| {
            let mesh_key = format!("{}.{}", character_id, anim);
            strings.intern(&mesh_key)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Build keyframes keys: just the animation name
    let keyframes_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| strings.intern(anim))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((mesh_keys, keyframes_keys))
}

/// Write a string reference (StrRef) to the buffer.
///
/// StrRef layout: offset(u32) + length(u16) + padding(u16)
fn write_strref(buf: &mut Vec<u8>, strref: StrRef) {
    write_u32_le(buf, strref.0); // offset
    write_u16_le(buf, strref.1); // length
    write_u16_le(buf, 0); // padding
}

fn write_range(buf: &mut Vec<u8>, off: u32, len: u16) {
    write_u32_le(buf, off);
    write_u16_le(buf, len);
    write_u16_le(buf, 0);
}

fn write_i32_le(buf: &mut Vec<u8>, value: i32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_i64_le(buf: &mut Vec<u8>, value: i64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_u64_le(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write a section header to the buffer.
///
/// Section header layout: kind(u32) + offset(u32) + length(u32) + alignment(u32)
fn write_section_header(buf: &mut Vec<u8>, kind: u32, offset: u32, length: u32, alignment: u32) {
    write_u32_le(buf, kind);
    write_u32_le(buf, offset);
    write_u32_le(buf, length);
    write_u32_le(buf, alignment);
}

/// Export character data to FSPK binary format.
///
/// Returns the packed binary data as a Vec<u8>.
#[allow(clippy::vec_init_then_push)] // Intentional: base sections first, optional sections conditionally added
pub fn export_zx_fspack(char_data: &CharacterData) -> Result<Vec<u8>, String> {
    // Canonicalize move ordering so move indices are deterministic.
    // (Do this here as a backstop even if callers already sorted.)
    let mut char_data = char_data.clone();
    char_data.moves.sort_by(|a, b| a.input.cmp(&b.input));

    // Step 1: Build string table and asset keys
    let mut strings = StringTable::new();
    let (mesh_keys, keyframes_keys) = build_asset_keys(&char_data, &mut strings)?;

    // Build animation-to-index map for pack_moves
    // The keys are sorted alphabetically, so we can create the map from the sorted animations
    let mut animations: Vec<&str> = char_data
        .moves
        .iter()
        .filter(|m| !m.animation.is_empty())
        .map(|m| m.animation.as_str())
        .collect();
    animations.sort();
    animations.dedup();

    let mut anim_to_index: HashMap<String, u16> = HashMap::new();
    for (i, anim) in animations.iter().enumerate() {
        let idx = checked_u16(i, "animation index")?;
        anim_to_index.insert((*anim).to_string(), idx);
    }

    // Build input-to-index map for resolving chain targets
    let input_to_index: HashMap<&str, u16> = char_data
        .moves
        .iter()
        .enumerate()
        .map(|(i, m)| (m.input.as_str(), i as u16))
        .collect();

    // Build cancel lookup from cancel_table
    let cancel_lookup = CancelLookup {
        chains: char_data
            .cancel_table
            .chains
            .keys()
            .map(|s| s.as_str())
            .collect(),
        chain_routes: &char_data.cancel_table.chains,
        specials: char_data
            .cancel_table
            .special_cancels
            .iter()
            .map(|s| s.as_str())
            .collect(),
        supers: char_data
            .cancel_table
            .super_cancels
            .iter()
            .map(|s| s.as_str())
            .collect(),
        jumps: char_data
            .cancel_table
            .jump_cancels
            .iter()
            .map(|s| s.as_str())
            .collect(),
        input_to_index,
    };

    // Step 2: Pack moves with animation indices and cancel lookup
    let packed = pack_moves(&char_data.moves, Some(&anim_to_index), Some(&cancel_lookup))?;

    // Step 3: Pack optional sections (resources, events, notifies)
    const OPT_U16_NONE: u16 = u16::MAX;
    const EVENT_ARG_TAG_BOOL: u8 = 0;
    const EVENT_ARG_TAG_I64: u8 = 1;
    const EVENT_ARG_TAG_F32: u8 = 2;
    const EVENT_ARG_TAG_STRING: u8 = 3;
    const RESOURCE_DELTA_TRIGGER_ON_USE: u8 = 0;
    const RESOURCE_DELTA_TRIGGER_ON_HIT: u8 = 1;
    const RESOURCE_DELTA_TRIGGER_ON_BLOCK: u8 = 2;

    let mut resource_defs_data: Vec<u8> = Vec::new();
    if !char_data.character.resources.is_empty() {
        for res in &char_data.character.resources {
            let name = strings.intern(&res.name)?;
            write_strref(&mut resource_defs_data, name);
            write_u16_le(&mut resource_defs_data, res.start);
            write_u16_le(&mut resource_defs_data, res.max);
        }
    }

    let mut event_emits_data: Vec<u8> = Vec::new();
    let mut event_args_data: Vec<u8> = Vec::new();
    let mut move_notifies_data: Vec<u8> = Vec::new();
    let mut move_resource_costs_data: Vec<u8> = Vec::new();
    let mut move_resource_preconditions_data: Vec<u8> = Vec::new();
    let mut move_resource_deltas_data: Vec<u8> = Vec::new();

    // MOVE_EXTRAS is always parallel to MOVES when present.
    // Now 9 fields: 8 original + cancels
    let mut move_extras_records: Vec<[(u32, u16); 9]> = Vec::with_capacity(char_data.moves.len());

    let mut any_move_extras = false;

    for (move_idx, mv) in char_data.moves.iter().enumerate() {
        let on_use_events = mv
            .on_use
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);
        let on_hit_events = mv
            .on_hit
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);
        let on_block_events = mv
            .on_block
            .as_ref()
            .map(|x| x.events.as_slice())
            .unwrap_or(&[]);

        let on_use_emits_off = checked_u32(event_emits_data.len(), "on_use_emits_off")?;
        let on_use_emits_len = checked_u16(on_use_events.len(), "on_use_emits_len")?;
        for emit in on_use_events {
            let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
            let args_len = checked_u16(emit.args.len(), "event_args_len")?;

            for (k, v) in &emit.args {
                let key = strings.intern(k)?;
                write_strref(&mut event_args_data, key);

                match v {
                    crate::schema::EventArgValue::Bool(b) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_BOOL);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, if *b { 1 } else { 0 });
                    }
                    crate::schema::EventArgValue::I64(i) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_I64);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_i64_le(&mut event_args_data, *i);
                    }
                    crate::schema::EventArgValue::F32(f) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_F32);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, f.to_bits() as u64);
                    }
                    crate::schema::EventArgValue::String(s) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_STRING);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        let vref = strings.intern(s)?;
                        write_u32_le(&mut event_args_data, vref.0);
                        write_u16_le(&mut event_args_data, vref.1);
                        write_u16_le(&mut event_args_data, 0);
                    }
                }
            }

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        let on_hit_emits_off = checked_u32(event_emits_data.len(), "on_hit_emits_off")?;
        let on_hit_emits_len = checked_u16(on_hit_events.len(), "on_hit_emits_len")?;
        for emit in on_hit_events {
            let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
            let args_len = checked_u16(emit.args.len(), "event_args_len")?;

            for (k, v) in &emit.args {
                let key = strings.intern(k)?;
                write_strref(&mut event_args_data, key);

                match v {
                    crate::schema::EventArgValue::Bool(b) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_BOOL);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, if *b { 1 } else { 0 });
                    }
                    crate::schema::EventArgValue::I64(i) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_I64);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_i64_le(&mut event_args_data, *i);
                    }
                    crate::schema::EventArgValue::F32(f) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_F32);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, f.to_bits() as u64);
                    }
                    crate::schema::EventArgValue::String(s) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_STRING);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        let vref = strings.intern(s)?;
                        write_u32_le(&mut event_args_data, vref.0);
                        write_u16_le(&mut event_args_data, vref.1);
                        write_u16_le(&mut event_args_data, 0);
                    }
                }
            }

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        let on_block_emits_off = checked_u32(event_emits_data.len(), "on_block_emits_off")?;
        let on_block_emits_len = checked_u16(on_block_events.len(), "on_block_emits_len")?;
        for emit in on_block_events {
            let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
            let args_len = checked_u16(emit.args.len(), "event_args_len")?;

            for (k, v) in &emit.args {
                let key = strings.intern(k)?;
                write_strref(&mut event_args_data, key);

                match v {
                    crate::schema::EventArgValue::Bool(b) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_BOOL);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, if *b { 1 } else { 0 });
                    }
                    crate::schema::EventArgValue::I64(i) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_I64);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_i64_le(&mut event_args_data, *i);
                    }
                    crate::schema::EventArgValue::F32(f) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_F32);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        write_u64_le(&mut event_args_data, f.to_bits() as u64);
                    }
                    crate::schema::EventArgValue::String(s) => {
                        write_u8(&mut event_args_data, EVENT_ARG_TAG_STRING);
                        write_u8(&mut event_args_data, 0);
                        write_u16_le(&mut event_args_data, 0);
                        let vref = strings.intern(s)?;
                        write_u32_le(&mut event_args_data, vref.0);
                        write_u16_le(&mut event_args_data, vref.1);
                        write_u16_le(&mut event_args_data, 0);
                    }
                }
            }

            let id = strings.intern(&emit.id)?;
            write_strref(&mut event_emits_data, id);
            write_range(&mut event_emits_data, args_off, args_len);
        }

        // Move notifies
        let notifies_off = checked_u32(move_notifies_data.len(), "notifies_off")?;
        let notifies_len = checked_u16(mv.notifies.len(), "notifies_len")?;
        for notify in &mv.notifies {
            let notify_emits_off = checked_u32(event_emits_data.len(), "notify_emits_off")?;
            let notify_emits_len = checked_u16(notify.events.len(), "notify_emits_len")?;

            for emit in &notify.events {
                let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
                let args_len = checked_u16(emit.args.len(), "event_args_len")?;

                for (k, v) in &emit.args {
                    let key = strings.intern(k)?;
                    write_strref(&mut event_args_data, key);

                    match v {
                        crate::schema::EventArgValue::Bool(b) => {
                            write_u8(&mut event_args_data, EVENT_ARG_TAG_BOOL);
                            write_u8(&mut event_args_data, 0);
                            write_u16_le(&mut event_args_data, 0);
                            write_u64_le(&mut event_args_data, if *b { 1 } else { 0 });
                        }
                        crate::schema::EventArgValue::I64(i) => {
                            write_u8(&mut event_args_data, EVENT_ARG_TAG_I64);
                            write_u8(&mut event_args_data, 0);
                            write_u16_le(&mut event_args_data, 0);
                            write_i64_le(&mut event_args_data, *i);
                        }
                        crate::schema::EventArgValue::F32(f) => {
                            write_u8(&mut event_args_data, EVENT_ARG_TAG_F32);
                            write_u8(&mut event_args_data, 0);
                            write_u16_le(&mut event_args_data, 0);
                            write_u64_le(&mut event_args_data, f.to_bits() as u64);
                        }
                        crate::schema::EventArgValue::String(s) => {
                            write_u8(&mut event_args_data, EVENT_ARG_TAG_STRING);
                            write_u8(&mut event_args_data, 0);
                            write_u16_le(&mut event_args_data, 0);
                            let vref = strings.intern(s)?;
                            write_u32_le(&mut event_args_data, vref.0);
                            write_u16_le(&mut event_args_data, vref.1);
                            write_u16_le(&mut event_args_data, 0);
                        }
                    }
                }

                let id = strings.intern(&emit.id)?;
                write_strref(&mut event_emits_data, id);
                write_range(&mut event_emits_data, args_off, args_len);
            }

            // MoveNotify12: frame(u16) + pad(u16) + emits_off(u32) + emits_len(u16) + pad(u16)
            write_u16_le(&mut move_notifies_data, notify.frame);
            write_u16_le(&mut move_notifies_data, 0);
            write_range(&mut move_notifies_data, notify_emits_off, notify_emits_len);
        }

        // Move resource costs (Cost::Resource only)
        let costs_off = checked_u32(move_resource_costs_data.len(), "costs_off")?;
        let mut costs_len: u16 = 0;
        if let Some(costs) = &mv.costs {
            for cost in costs {
                if let crate::schema::Cost::Resource { name, amount } = cost {
                    let rname = strings.intern(name)?;
                    write_strref(&mut move_resource_costs_data, rname);
                    write_u16_le(&mut move_resource_costs_data, *amount);
                    write_u16_le(&mut move_resource_costs_data, 0);
                    costs_len = costs_len
                        .checked_add(1)
                        .ok_or_else(|| "move resource costs count overflows u16".to_string())?;
                }
            }
        }

        // Move resource preconditions (Precondition::Resource only)
        let pre_off = checked_u32(move_resource_preconditions_data.len(), "pre_off")?;
        let mut pre_len: u16 = 0;
        if let Some(preconditions) = &mv.preconditions {
            for pre in preconditions {
                if let crate::schema::Precondition::Resource { name, min, max } = pre {
                    let rname = strings.intern(name)?;
                    write_strref(&mut move_resource_preconditions_data, rname);
                    write_u16_le(
                        &mut move_resource_preconditions_data,
                        min.unwrap_or(OPT_U16_NONE),
                    );
                    write_u16_le(
                        &mut move_resource_preconditions_data,
                        max.unwrap_or(OPT_U16_NONE),
                    );
                    pre_len = pre_len.checked_add(1).ok_or_else(|| {
                        "move resource preconditions count overflows u16".to_string()
                    })?;
                }
            }
        }

        // Move resource deltas (on_use/on_hit/on_block)
        let deltas_off = checked_u32(move_resource_deltas_data.len(), "deltas_off")?;
        let mut deltas_len: u16 = 0;
        if let Some(on_use) = &mv.on_use {
            for d in &on_use.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_USE,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }
        if let Some(on_hit) = &mv.on_hit {
            for d in &on_hit.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_HIT,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }
        if let Some(on_block) = &mv.on_block {
            for d in &on_block.resource_deltas {
                let rname = strings.intern(&d.name)?;
                write_strref(&mut move_resource_deltas_data, rname);
                write_i32_le(&mut move_resource_deltas_data, d.delta);
                write_u8(
                    &mut move_resource_deltas_data,
                    RESOURCE_DELTA_TRIGGER_ON_BLOCK,
                );
                move_resource_deltas_data.extend_from_slice(&[0, 0, 0]);
                deltas_len = deltas_len
                    .checked_add(1)
                    .ok_or_else(|| "move resource deltas count overflows u16".to_string())?;
            }
        }

        // Intern the move input notation string.
        let input_ref = strings.intern(&mv.input)?;

        // Get cancel info from packed data
        let (cancel_off, cancel_len) = packed.cancel_info[move_idx];

        let record = [
            (on_use_emits_off, on_use_emits_len),
            (on_hit_emits_off, on_hit_emits_len),
            (on_block_emits_off, on_block_emits_len),
            (notifies_off, notifies_len),
            (costs_off, costs_len),
            (pre_off, pre_len),
            (deltas_off, deltas_len),
            (input_ref.0, input_ref.1),
            (cancel_off, cancel_len),
        ];

        // Always emit MOVE_EXTRAS when there are moves, since every move has an input.
        any_move_extras = true;
        move_extras_records.push(record);
    }

    // Step 4: Build section data
    // Mesh keys section: array of StrRef
    let mut mesh_keys_data = Vec::with_capacity(mesh_keys.len() * STRREF_SIZE);
    for strref in &mesh_keys {
        write_strref(&mut mesh_keys_data, *strref);
    }

    // Keyframes keys section: array of StrRef
    let mut keyframes_keys_data = Vec::with_capacity(keyframes_keys.len() * STRREF_SIZE);
    for strref in &keyframes_keys {
        write_strref(&mut keyframes_keys_data, *strref);
    }

    let mut move_extras_data: Vec<u8> = Vec::new();
    if any_move_extras {
        move_extras_data.reserve(move_extras_records.len() * STATE_EXTRAS72_SIZE);
        for rec in &move_extras_records {
            for (off, len) in rec {
                write_range(&mut move_extras_data, *off, *len);
            }
        }
    }

    // Omit backing sections if they have no data.
    if event_emits_data.is_empty() {
        // If no emits, args are unreachable; omit for cleanliness.
        event_args_data.clear();
    }
    if event_args_data.is_empty() {
        // Keep empty args data.
    }
    if move_notifies_data.is_empty() {
        // no-op
    }

    // Build state tag sections (one range entry per move, tags are StrRefs)
    // Note: move_type (the "type" field) is also included as a tag so that
    // tag-based cancel rules can match on it (e.g., "system" -> "any")
    let mut state_tag_ranges_data: Vec<u8> = Vec::new();
    let mut state_tags_data: Vec<u8> = Vec::new();
    let any_tags = char_data
        .moves
        .iter()
        .any(|m| !m.tags.is_empty() || m.move_type.is_some());

    if any_tags {
        for mv in &char_data.moves {
            let tag_offset = checked_u32(state_tags_data.len(), "state_tags_offset")?;
            // Count includes move_type if present
            let type_count = if mv.move_type.is_some() { 1 } else { 0 };
            let tag_count = checked_u16(mv.tags.len() + type_count, "state_tags_count")?;

            // Write range entry: offset(4) + count(2) + padding(2)
            write_u32_le(&mut state_tag_ranges_data, tag_offset);
            write_u16_le(&mut state_tag_ranges_data, tag_count);
            write_u16_le(&mut state_tag_ranges_data, 0); // padding

            // Write move_type as first tag if present (so cancel rules can match on type)
            if let Some(ref move_type) = mv.move_type {
                let (str_off, str_len) = strings.intern(move_type.as_str())?;
                write_strref(&mut state_tags_data, (str_off, str_len));
            }

            // Write explicit tag StrRefs
            for tag in &mv.tags {
                let (str_off, str_len) = strings.intern(tag.as_str())?;
                write_strref(&mut state_tags_data, (str_off, str_len));
            }
        }
    }

    // Encode cancel tag rules
    // CancelTagRule24: from_tag StrRef (8) + to_tag StrRef (8) + condition (1) + min_frame (1) + max_frame (1) + flags (1) + padding (4) = 24
    let mut cancel_tag_rules_data: Vec<u8> = Vec::new();
    for rule in &char_data.cancel_table.tag_rules {
        // from_tag StrRef (8 bytes) - use 0xFFFFFFFF sentinel for "any"
        if rule.from == "any" {
            write_u32_le(&mut cancel_tag_rules_data, 0xFFFFFFFF);
            write_u16_le(&mut cancel_tag_rules_data, 0);
            write_u16_le(&mut cancel_tag_rules_data, 0);
        } else {
            let (off, len) = strings.intern(&rule.from)?;
            write_strref(&mut cancel_tag_rules_data, (off, len));
        }

        // to_tag StrRef (8 bytes) - use 0xFFFFFFFF sentinel for "any"
        if rule.to == "any" {
            write_u32_le(&mut cancel_tag_rules_data, 0xFFFFFFFF);
            write_u16_le(&mut cancel_tag_rules_data, 0);
            write_u16_le(&mut cancel_tag_rules_data, 0);
        } else {
            let (off, len) = strings.intern(&rule.to)?;
            write_strref(&mut cancel_tag_rules_data, (off, len));
        }

        // condition (1 byte)
        let condition: u8 = match rule.on {
            crate::schema::CancelCondition::Always => 0,
            crate::schema::CancelCondition::Hit => 1,
            crate::schema::CancelCondition::Block => 2,
            crate::schema::CancelCondition::Whiff => 3,
        };
        write_u8(&mut cancel_tag_rules_data, condition);
        // min_frame (1 byte)
        write_u8(&mut cancel_tag_rules_data, rule.after_frame);
        // max_frame (1 byte)
        write_u8(&mut cancel_tag_rules_data, rule.before_frame);
        // flags (1 byte) - reserved
        write_u8(&mut cancel_tag_rules_data, 0);
        // padding (4 bytes)
        write_u32_le(&mut cancel_tag_rules_data, 0);
    }

    // Encode cancel denies
    // CancelDeny4: from_idx (u16) + to_idx (u16) = 4 bytes
    let mut cancel_denies_data: Vec<u8> = Vec::new();
    for (from_input, deny_list) in &char_data.cancel_table.deny {
        if let Some(&from_idx) = cancel_lookup.input_to_index.get(from_input.as_str()) {
            for to_input in deny_list {
                if let Some(&to_idx) = cancel_lookup.input_to_index.get(to_input.as_str()) {
                    write_u16_le(&mut cancel_denies_data, from_idx);
                    write_u16_le(&mut cancel_denies_data, to_idx);
                }
            }
        }
    }

    let string_table_data = strings.into_bytes();

    struct SectionData {
        kind: u32,
        align: u32,
        bytes: Vec<u8>,
    }

    let mut sections: Vec<SectionData> = Vec::new();

    // Base v1 sections (always present, same order)
    sections.push(SectionData {
        kind: SECTION_STRING_TABLE,
        align: 1,
        bytes: string_table_data,
    });
    sections.push(SectionData {
        kind: SECTION_MESH_KEYS,
        align: 4,
        bytes: mesh_keys_data,
    });
    sections.push(SectionData {
        kind: SECTION_KEYFRAMES_KEYS,
        align: 4,
        bytes: keyframes_keys_data,
    });
    sections.push(SectionData {
        kind: SECTION_STATES,
        align: 4,
        bytes: packed.moves,
    });
    sections.push(SectionData {
        kind: SECTION_HIT_WINDOWS,
        align: 4,
        bytes: packed.hit_windows,
    });
    sections.push(SectionData {
        kind: SECTION_HURT_WINDOWS,
        align: 4,
        bytes: packed.hurt_windows,
    });
    sections.push(SectionData {
        kind: SECTION_SHAPES,
        align: 4,
        bytes: packed.shapes,
    });
    sections.push(SectionData {
        kind: SECTION_CANCELS_U16,
        align: 2,
        bytes: packed.cancels,
    });

    // Optional sections (only present if data)
    if !resource_defs_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_RESOURCE_DEFS,
            align: 4,
            bytes: resource_defs_data,
        });
    }
    if any_move_extras {
        sections.push(SectionData {
            kind: SECTION_STATE_EXTRAS,
            align: 4,
            bytes: move_extras_data,
        });
    }
    if !event_emits_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_EVENT_EMITS,
            align: 4,
            bytes: event_emits_data,
        });
    }
    if !event_args_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_EVENT_ARGS,
            align: 4,
            bytes: event_args_data,
        });
    }
    if !move_notifies_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_NOTIFIES,
            align: 4,
            bytes: move_notifies_data,
        });
    }
    if !move_resource_costs_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_COSTS,
            align: 4,
            bytes: move_resource_costs_data,
        });
    }
    if !move_resource_preconditions_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_PRECONDITIONS,
            align: 4,
            bytes: move_resource_preconditions_data,
        });
    }
    if !move_resource_deltas_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_MOVE_RESOURCE_DELTAS,
            align: 4,
            bytes: move_resource_deltas_data,
        });
    }
    if !state_tag_ranges_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_STATE_TAG_RANGES,
            align: 4,
            bytes: state_tag_ranges_data,
        });
        sections.push(SectionData {
            kind: SECTION_STATE_TAGS,
            align: 4,
            bytes: state_tags_data,
        });
    }
    if !cancel_tag_rules_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CANCEL_TAG_RULES,
            align: 4,
            bytes: cancel_tag_rules_data,
        });
    }
    if !cancel_denies_data.is_empty() {
        sections.push(SectionData {
            kind: SECTION_CANCEL_DENIES,
            align: 4,
            bytes: cancel_denies_data,
        });
    }

    if sections.len() > 24 {
        return Err(format!(
            "Too many sections ({}), MAX_SECTIONS is 24",
            sections.len()
        ));
    }

    // Step 5: Calculate section offsets (honor per-section alignment)
    let section_count = checked_u32(sections.len(), "section_count")?;
    let header_and_sections_size = HEADER_SIZE + (sections.len() * SECTION_HEADER_SIZE);
    let mut current_offset: usize = header_and_sections_size;

    #[derive(Clone, Copy)]
    struct SectionHeader {
        kind: u32,
        off: u32,
        len: u32,
        align: u32,
    }

    let mut section_headers: Vec<SectionHeader> = Vec::with_capacity(sections.len());
    for s in &sections {
        current_offset = align_up(current_offset, s.align)?;
        let off = checked_u32(current_offset, "section offset")?;
        let len = checked_u32(s.bytes.len(), "section length")?;
        section_headers.push(SectionHeader {
            kind: s.kind,
            off,
            len,
            align: s.align,
        });
        current_offset = current_offset
            .checked_add(s.bytes.len())
            .ok_or_else(|| "section offset overflow".to_string())?;
    }

    let total_len = checked_u32(current_offset, "total_len")?;

    // Step 6: Build the final binary
    let mut output = Vec::with_capacity(current_offset);
    output.extend_from_slice(&MAGIC);
    write_u32_le(&mut output, FLAGS_RESERVED);
    write_u32_le(&mut output, total_len);
    write_u32_le(&mut output, section_count);

    for h in &section_headers {
        write_section_header(&mut output, h.kind, h.off, h.len, h.align);
    }

    for (i, s) in sections.into_iter().enumerate() {
        let h = section_headers[i];
        let target = h.off as usize;
        if output.len() > target {
            return Err(format!(
                "section {} starts before current output (off={} len={})",
                i, h.off, h.len
            ));
        }
        output.resize(target, 0);
        output.extend_from_slice(&s.bytes);
    }

    debug_assert_eq!(
        output.len(),
        total_len as usize,
        "Output size mismatch: expected {}, got {}",
        total_len,
        output.len()
    );

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        CancelTable, Character, CharacterResource, FrameHitbox, GuardType, MeterGain, Pushback,
        Rect, State,
    };
    use std::collections::HashMap;

    fn read_u32_le(bytes: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]])
    }

    /// Create a minimal test character.
    fn make_test_character(id: &str) -> Character {
        use crate::schema::PropertyValue;
        use std::collections::BTreeMap;

        let mut properties = BTreeMap::new();
        properties.insert("archetype".to_string(), PropertyValue::String("rushdown".to_string()));
        properties.insert("health".to_string(), PropertyValue::Number(1000.0));
        properties.insert("walk_speed".to_string(), PropertyValue::Number(3.5));
        properties.insert("back_walk_speed".to_string(), PropertyValue::Number(2.5));
        properties.insert("jump_height".to_string(), PropertyValue::Number(120.0));
        properties.insert("jump_duration".to_string(), PropertyValue::Number(40.0));
        properties.insert("dash_distance".to_string(), PropertyValue::Number(80.0));
        properties.insert("dash_duration".to_string(), PropertyValue::Number(20.0));

        Character {
            id: id.to_string(),
            name: "Test Character".to_string(),
            properties,
            resources: vec![],
        }
    }

    /// Create a minimal test state with the given input and animation.
    fn make_test_move(input: &str, animation: &str) -> State {
        State {
            input: input.to_string(),
            name: format!("{} attack", input),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 50,
            hitstun: 15,
            blockstun: 10,
            hitstop: 5,
            guard: GuardType::Mid,
            hitboxes: vec![],
            hurtboxes: vec![],
            pushback: Pushback { hit: 10, block: 5 },
            meter_gain: MeterGain { hit: 10, whiff: 5 },
            animation: animation.to_string(),
            move_type: None,
            trigger: None,
            parent: None,
            total: None,
            hits: None,
            preconditions: None,
            costs: None,
            movement: None,
            super_freeze: None,
            on_use: None,
            on_hit: None,
            on_block: None,
            notifies: vec![],
            advanced_hurtboxes: None,
            base: None,
            id: None,
        }
    }

    /// Create an empty cancel table.
    fn make_empty_cancel_table() -> CancelTable {
        CancelTable::default()
    }

    #[test]
    fn test_string_table_intern_returns_same_location_for_same_string() {
        let mut table = StringTable::new();

        let loc1 = table.intern("hello").unwrap();
        let loc2 = table.intern("hello").unwrap();

        assert_eq!(loc1, loc2, "Same string should return same location");
    }

    #[test]
    fn test_string_table_intern_different_strings() {
        let mut table = StringTable::new();

        let loc1 = table.intern("hello").unwrap();
        let loc2 = table.intern("world").unwrap();

        assert_ne!(
            loc1.0, loc2.0,
            "Different strings should have different offsets"
        );
        assert_eq!(loc1.0, 0, "First string should start at offset 0");
        assert_eq!(loc1.1, 5, "\"hello\" has length 5");
        assert_eq!(loc2.0, 5, "Second string should start after first");
        assert_eq!(loc2.1, 5, "\"world\" has length 5");
    }

    #[test]
    fn test_string_table_into_bytes() {
        let mut table = StringTable::new();
        table.intern("abc").unwrap();
        table.intern("def").unwrap();

        let bytes = table.into_bytes();
        assert_eq!(bytes, b"abcdef");
    }

    #[test]
    fn test_build_asset_keys_deterministic() {
        // Create character data with moves in non-alphabetical order
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5H", "stand_heavy"),
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings1 = StringTable::new();
        let (mesh_keys1, kf_keys1) = build_asset_keys(&char_data, &mut strings1).unwrap();

        // Create the same data but with moves in different order
        let char_data2 = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5M", "stand_medium"),
                make_test_move("5L", "stand_light"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings2 = StringTable::new();
        let (mesh_keys2, kf_keys2) = build_asset_keys(&char_data2, &mut strings2).unwrap();

        // Keys should be identical regardless of input order
        assert_eq!(mesh_keys1.len(), mesh_keys2.len());
        assert_eq!(kf_keys1.len(), kf_keys2.len());

        // String table content should be identical
        assert_eq!(strings1.into_bytes(), strings2.into_bytes());
    }

    #[test]
    fn test_build_asset_keys_deduplication() {
        // Create character data where two moves share the same animation
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation as 5L
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();

        // Should have only 2 unique animations, not 3
        assert_eq!(
            mesh_keys.len(),
            2,
            "Duplicate animations should be deduplicated"
        );
        assert_eq!(
            kf_keys.len(),
            2,
            "Duplicate animations should be deduplicated"
        );
    }

    #[test]
    fn test_build_asset_keys_sorted_order() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("236P", "zebra_anim"),
                make_test_move("5L", "alpha_anim"),
                make_test_move("5M", "beta_anim"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, _kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();
        let bytes = strings.into_bytes();

        // Verify that strings appear in sorted order: alpha_anim, beta_anim, zebra_anim
        // First mesh key should be "test.alpha_anim" starting at offset 0
        assert_eq!(mesh_keys[0].0, 0);

        // Extract the first mesh key string
        let first_key_start = mesh_keys[0].0 as usize;
        let first_key_len = mesh_keys[0].1 as usize;
        let first_key =
            std::str::from_utf8(&bytes[first_key_start..first_key_start + first_key_len]).unwrap();
        assert_eq!(first_key, "test.alpha_anim");
    }

    #[test]
    fn test_build_asset_keys_mesh_format() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, _kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();
        let bytes = strings.into_bytes();

        // Extract mesh key string
        let mesh_key_start = mesh_keys[0].0 as usize;
        let mesh_key_len = mesh_keys[0].1 as usize;
        let mesh_key =
            std::str::from_utf8(&bytes[mesh_key_start..mesh_key_start + mesh_key_len]).unwrap();

        assert_eq!(mesh_key, "test_char.stand_light");
    }

    #[test]
    fn test_build_asset_keys_keyframes_format() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (_mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();
        let bytes = strings.into_bytes();

        // Extract keyframes key string
        let kf_key_start = kf_keys[0].0 as usize;
        let kf_key_len = kf_keys[0].1 as usize;
        let kf_key = std::str::from_utf8(&bytes[kf_key_start..kf_key_start + kf_key_len]).unwrap();

        // Keyframes key is just the animation name
        assert_eq!(kf_key, "stand_light");
    }

    #[test]
    fn test_build_asset_keys_empty_moves() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();

        assert!(mesh_keys.is_empty());
        assert!(kf_keys.is_empty());
        assert!(strings.is_empty());
    }

    #[test]
    fn test_build_asset_keys_skips_empty_animations() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", ""), // Empty animation should be skipped
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();

        assert_eq!(mesh_keys.len(), 1, "Should skip moves with empty animation");
        assert_eq!(kf_keys.len(), 1, "Should skip moves with empty animation");
    }

    #[test]
    fn test_export_zx_fspack_magic_and_section_count() {
        // Create minimal character data
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_zx_fspack(&char_data);
        assert!(result.is_ok(), "export_zx_fspack should succeed");

        let bytes = result.unwrap();

        // Verify magic bytes "FSPK"
        assert!(bytes.len() >= 16, "Output should have at least header size");
        assert_eq!(&bytes[0..4], b"FSPK", "Magic should be FSPK");

        // Verify flags
        let flags = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        assert_eq!(flags, 0, "Flags should be 0");

        // Verify total length matches actual output
        let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        assert_eq!(
            total_len as usize,
            bytes.len(),
            "Total length should match actual output size"
        );

        // Verify section count
        let section_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert_eq!(section_count, 9, "Section count should be 9");
    }

    #[test]
    fn test_export_zx_fspack_empty_character() {
        // Create character data with no moves
        let char_data = CharacterData {
            character: make_test_character("empty"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_zx_fspack(&char_data);
        assert!(
            result.is_ok(),
            "export_zx_fspack should succeed with no moves"
        );

        let bytes = result.unwrap();

        // Should still have valid FSPK header
        assert_eq!(&bytes[0..4], b"FSPK");

        // Section count should still be 8
        let section_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert_eq!(section_count, 8);
    }

    #[test]
    fn test_export_zx_fspack_is_deterministic_for_shuffled_moves() {
        let move_a = make_test_move("5M", "stand_medium");
        let move_b = make_test_move("5L", "stand_light");
        let move_c = make_test_move("236P", "fireball");

        let char_data1 = CharacterData {
            character: make_test_character("test"),
            moves: vec![move_a.clone(), move_b.clone(), move_c.clone()],
            cancel_table: make_empty_cancel_table(),
        };

        let char_data2 = CharacterData {
            character: make_test_character("test"),
            moves: vec![move_c, move_a, move_b],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes1 = export_zx_fspack(&char_data1).unwrap();
        let bytes2 = export_zx_fspack(&char_data2).unwrap();
        assert_eq!(
            bytes1, bytes2,
            "Export should be deterministic regardless of move order"
        );
    }

    #[test]
    fn test_export_zx_fspack_section_headers() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_zx_fspack(&char_data).unwrap();

        let section_count = read_u32_le(&bytes, 12) as usize;
        let header_end = HEADER_SIZE + (section_count * SECTION_HEADER_SIZE);
        assert!(
            bytes.len() >= header_end,
            "Output should have room for all section headers"
        );

        // Check that section kinds are correct (base v1 sections in order)
        let expected_base_kinds = [1, 2, 3, 4, 5, 6, 7, 8]; // STRING_TABLE through CANCELS_U16
        for (i, &expected_kind) in expected_base_kinds.iter().enumerate() {
            let offset = HEADER_SIZE + i * SECTION_HEADER_SIZE;
            let kind = u32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            assert_eq!(
                kind, expected_kind,
                "Section {} should have kind {}",
                i, expected_kind
            );
        }

        // MOVE_EXTRAS is expected when there are moves.
        assert_eq!(
            section_count, 9,
            "Expected MOVE_EXTRAS section to be present"
        );
        let extras_kind_off = HEADER_SIZE + 8 * SECTION_HEADER_SIZE;
        let extras_kind = u32::from_le_bytes([
            bytes[extras_kind_off],
            bytes[extras_kind_off + 1],
            bytes[extras_kind_off + 2],
            bytes[extras_kind_off + 3],
        ]);
        assert_eq!(extras_kind, SECTION_STATE_EXTRAS);
    }

    #[test]
    fn test_export_zx_fspack_section_offsets_aligned_and_non_overlapping() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_zx_fspack(&char_data).unwrap();
        let section_count = read_u32_le(&bytes, 12) as usize;
        let header_end = HEADER_SIZE + section_count * SECTION_HEADER_SIZE;

        let mut prev_end = header_end as u32;
        for i in 0..section_count {
            let base = HEADER_SIZE + i * SECTION_HEADER_SIZE;
            let off = read_u32_le(&bytes, base + 4);
            let len = read_u32_le(&bytes, base + 8);
            let align = read_u32_le(&bytes, base + 12);

            assert!(align != 0, "section {} has zero alignment", i);
            assert_eq!(
                off % align,
                0,
                "section {} offset {} must be aligned to {}",
                i,
                off,
                align
            );
            assert!(
                off >= prev_end,
                "section {} overlaps previous: off={} prev_end={}",
                i,
                off,
                prev_end
            );

            prev_end = off + len;
        }
    }

    #[test]
    fn test_export_zx_fspack_rejects_string_len_overflow() {
        let mut character = make_test_character("test");
        character.resources = vec![CharacterResource {
            name: "a".repeat(u16::MAX as usize + 1),
            start: 0,
            max: 1,
        }];

        let char_data = CharacterData {
            character,
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_zx_fspack(&char_data);
        assert!(result.is_err(), "expected overflow to return Err");
    }

    #[test]
    fn test_export_zx_fspack_rejects_hurt_windows_off_overflow() {
        fn hb() -> FrameHitbox {
            FrameHitbox {
                frames: (0, 0),
                r#box: Rect {
                    x: 0,
                    y: 0,
                    w: 1,
                    h: 1,
                },
            }
        }

        // Ensure the second move's hurt_windows_off (u16 byte offset) overflows.
        let hurtbox_count = (u16::MAX as usize / HURT_WINDOW12_SIZE) + 1;
        let hurtboxes: Vec<FrameHitbox> = (0..hurtbox_count).map(|_| hb()).collect();

        let mut mv1 = make_test_move("5L", "stand_light");
        mv1.hurtboxes = hurtboxes;
        let mv2 = make_test_move("5M", "stand_medium");

        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![mv1, mv2],
            cancel_table: make_empty_cancel_table(),
        };

        let result = export_zx_fspack(&char_data);
        assert!(result.is_err(), "expected overflow to return Err");
    }

    #[test]
    fn test_export_zx_fspack_with_animation_keys() {
        // Create character with multiple moves sharing some animations
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
                make_test_move("2L", "stand_light"), // Shares animation with 5L
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_zx_fspack(&char_data).unwrap();

        // Verify magic
        assert_eq!(&bytes[0..4], b"FSPK");

        // The string table should contain "test.stand_light", "stand_light",
        // "test.stand_medium", "stand_medium" (deduplicated, sorted)
        // Just verify the export succeeded and has reasonable size
        assert!(bytes.len() > HEADER_SIZE + 8 * SECTION_HEADER_SIZE);
    }

    // ==========================================================================
    // Move Packing Tests
    // ==========================================================================

    fn make_test_rect() -> Rect {
        Rect {
            x: 10,
            y: 20,
            w: 50,
            h: 60,
        }
    }

    fn make_test_hitbox() -> FrameHitbox {
        FrameHitbox {
            frames: (5, 8),
            r#box: make_test_rect(),
        }
    }

    fn make_move_with_hitboxes() -> State {
        State {
            input: "5L".to_string(),
            name: "Light Punch".to_string(),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 500,
            hitstun: 12,
            blockstun: 8,
            hitstop: 10,
            guard: GuardType::Mid,
            hitboxes: vec![make_test_hitbox()],
            hurtboxes: vec![FrameHitbox {
                frames: (1, 18),
                r#box: Rect {
                    x: -20,
                    y: 0,
                    w: 40,
                    h: 80,
                },
            }],
            pushback: Pushback { hit: 10, block: 5 },
            meter_gain: MeterGain { hit: 10, whiff: 5 },
            animation: "stand_light".to_string(),
            move_type: None,
            trigger: None,
            parent: None,
            total: None,
            hits: None,
            preconditions: None,
            costs: None,
            movement: None,
            super_freeze: None,
            on_use: None,
            on_hit: None,
            on_block: None,
            notifies: vec![],
            advanced_hurtboxes: None,
            base: None,
            id: None,
        }
    }

    #[test]
    fn test_pack_shape() {
        let rect = make_test_rect();
        let shape = pack_shape(&rect);

        assert_eq!(shape.len(), SHAPE12_SIZE);
        assert_eq!(shape[0], SHAPE_KIND_AABB);
        assert_eq!(shape[1], 0); // flags

        // x=10 -> Q12.4 = 160 = 0x00A0
        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, 160);

        // y=20 -> Q12.4 = 320 = 0x0140
        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, 320);

        // w=50 -> Q12.4 = 800 = 0x0320
        let w = u16::from_le_bytes([shape[6], shape[7]]);
        assert_eq!(w, 800);

        // h=60 -> Q12.4 = 960 = 0x03C0
        let h = u16::from_le_bytes([shape[8], shape[9]]);
        assert_eq!(h, 960);
    }

    #[test]
    fn test_pack_hit_window() {
        let hb = make_test_hitbox();
        let hw = pack_hit_window(&hb, 100, 500, 12, 8, 10, 1);

        assert_eq!(hw.len(), HIT_WINDOW24_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end
        assert_eq!(hw[2], 1); // guard (mid)
        assert_eq!(hw[3], 0); // reserved

        let damage = u16::from_le_bytes([hw[4], hw[5]]);
        assert_eq!(damage, 500);

        let chip_damage = u16::from_le_bytes([hw[6], hw[7]]);
        assert_eq!(chip_damage, 0);

        assert_eq!(hw[8], 12); // hitstun
        assert_eq!(hw[9], 8); // blockstun
        assert_eq!(hw[10], 10); // hitstop
        assert_eq!(hw[11], 0); // reserved

        let shape_off = u32::from_le_bytes([hw[12], hw[13], hw[14], hw[15]]);
        assert_eq!(shape_off, 100);

        let shape_count = u16::from_le_bytes([hw[16], hw[17]]);
        assert_eq!(shape_count, 1);
    }

    #[test]
    fn test_pack_hurt_window() {
        let hb = make_test_hitbox();
        let hw = pack_hurt_window(&hb, 200);

        assert_eq!(hw.len(), HURT_WINDOW12_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end

        let shape_off = u32::from_le_bytes([hw[2], hw[3], hw[4], hw[5]]);
        assert_eq!(shape_off, 200);

        let shape_count = u16::from_le_bytes([hw[6], hw[7]]);
        assert_eq!(shape_count, 1);

        let flags = u16::from_le_bytes([hw[8], hw[9]]);
        assert_eq!(flags, 0);
    }

    #[test]
    fn test_pack_move_record() {
        let mv = make_move_with_hitboxes();
        let mr = pack_move_record(42, KEY_NONE, KEY_NONE, &mv, 100, 2, 200, 3, 0);

        assert_eq!(mr.len(), STATE_RECORD_SIZE);

        // 0-1: move_id
        let move_id = u16::from_le_bytes([mr[0], mr[1]]);
        assert_eq!(move_id, 42);

        // 2-3: mesh_key
        let mesh_key = u16::from_le_bytes([mr[2], mr[3]]);
        assert_eq!(mesh_key, KEY_NONE);

        // 4-5: keyframes_key
        let keyframes_key = u16::from_le_bytes([mr[4], mr[5]]);
        assert_eq!(keyframes_key, KEY_NONE);

        // 6: move_type (None -> 0)
        assert_eq!(mr[6], 0);
        // 7: trigger (None -> 0)
        assert_eq!(mr[7], 0);
        // 8: guard (Mid -> 1)
        assert_eq!(mr[8], 1);
        // 9: flags
        assert_eq!(mr[9], 0);

        // 10: startup
        assert_eq!(mr[10], 5);
        // 11: active
        assert_eq!(mr[11], 3);
        // 12: recovery
        assert_eq!(mr[12], 10);
        // 13: reserved
        assert_eq!(mr[13], 0);

        // 14-15: total (5+3+10=18)
        let total = u16::from_le_bytes([mr[14], mr[15]]);
        assert_eq!(total, 18);

        // 16-17: damage
        let damage = u16::from_le_bytes([mr[16], mr[17]]);
        assert_eq!(damage, 500);

        // 18: hitstun
        assert_eq!(mr[18], 12);
        // 19: blockstun
        assert_eq!(mr[19], 8);
        // 20: hitstop
        assert_eq!(mr[20], 10);
        // 21: reserved
        assert_eq!(mr[21], 0);

        // 22-25: hit_windows_off
        let hit_off = u32::from_le_bytes([mr[22], mr[23], mr[24], mr[25]]);
        assert_eq!(hit_off, 100);

        // 26-27: hit_windows_len
        let hit_len = u16::from_le_bytes([mr[26], mr[27]]);
        assert_eq!(hit_len, 2);

        // 28-29: hurt_windows_off (u16)
        let hurt_off = u16::from_le_bytes([mr[28], mr[29]]);
        assert_eq!(hurt_off, 200);

        // 30-31: hurt_windows_len
        let hurt_len = u16::from_le_bytes([mr[30], mr[31]]);
        assert_eq!(hurt_len, 3);
    }

    #[test]
    fn test_pack_moves_count_matches() {
        let moves = vec![make_move_with_hitboxes(), make_move_with_hitboxes()];
        let packed = pack_moves(&moves, None, None).unwrap();

        // Move count matches input
        let move_count = packed.moves.len() / STATE_RECORD_SIZE;
        assert_eq!(move_count, 2);

        // MOVES section length is correct
        assert_eq!(packed.moves.len(), 2 * STATE_RECORD_SIZE);
    }

    #[test]
    fn test_pack_moves_section_sizes() {
        let mv = make_move_with_hitboxes();
        let moves = vec![mv];
        let packed = pack_moves(&moves, None, None).unwrap();

        // 1 move with 1 hitbox and 1 hurtbox
        assert_eq!(packed.moves.len(), STATE_RECORD_SIZE);
        assert_eq!(packed.shapes.len(), 2 * SHAPE12_SIZE); // 1 hit + 1 hurt shape
        assert_eq!(packed.hit_windows.len(), HIT_WINDOW24_SIZE);
        assert_eq!(packed.hurt_windows.len(), HURT_WINDOW12_SIZE);
        assert_eq!(packed.cancels.len(), 0); // empty for v1
    }

    #[test]
    fn test_pack_moves_references_valid() {
        let moves = vec![make_move_with_hitboxes(), make_move_with_hitboxes()];
        let packed = pack_moves(&moves, None, None).unwrap();

        // Verify each move record has valid references
        for i in 0..2 {
            let offset = i * STATE_RECORD_SIZE;
            let record = &packed.moves[offset..offset + STATE_RECORD_SIZE];

            // Extract hit_windows offset (22-25) and length (26-27)
            let hit_off = u32::from_le_bytes([record[22], record[23], record[24], record[25]]);
            let hit_len = u16::from_le_bytes([record[26], record[27]]) as u32;

            // Verify hit_windows reference is within bounds
            let hit_end = hit_off + hit_len * HIT_WINDOW24_SIZE as u32;
            assert!(
                hit_end <= packed.hit_windows.len() as u32,
                "Hit window reference out of bounds for move {}: off={}, len={}, section_len={}",
                i,
                hit_off,
                hit_len,
                packed.hit_windows.len()
            );

            // Extract hurt_windows offset (28-29, u16) and length (30-31)
            let hurt_off = u16::from_le_bytes([record[28], record[29]]) as u32;
            let hurt_len = u16::from_le_bytes([record[30], record[31]]) as u32;

            // Verify hurt_windows reference is within bounds
            let hurt_end = hurt_off + hurt_len * HURT_WINDOW12_SIZE as u32;
            assert!(
                hurt_end <= packed.hurt_windows.len() as u32,
                "Hurt window reference out of bounds for move {}: off={}, len={}, section_len={}",
                i,
                hurt_off,
                hurt_len,
                packed.hurt_windows.len()
            );
        }
    }

    #[test]
    fn test_pack_moves_shape_references_valid() {
        let mv = make_move_with_hitboxes();
        let packed = pack_moves(&[mv], None, None).unwrap();

        // Verify hit window shape reference (shapes_off at bytes 12-15, shapes_len at 16-17)
        let hit_window = &packed.hit_windows[0..HIT_WINDOW24_SIZE];
        let shape_off =
            u32::from_le_bytes([hit_window[12], hit_window[13], hit_window[14], hit_window[15]]);
        let shape_count = u16::from_le_bytes([hit_window[16], hit_window[17]]) as u32;

        let shape_end = shape_off + shape_count * SHAPE12_SIZE as u32;
        assert!(
            shape_end <= packed.shapes.len() as u32,
            "Hit window shape reference out of bounds"
        );

        // Verify hurt window shape reference
        let hurt_window = &packed.hurt_windows[0..HURT_WINDOW12_SIZE];
        let shape_off = u32::from_le_bytes([
            hurt_window[2],
            hurt_window[3],
            hurt_window[4],
            hurt_window[5],
        ]);
        let shape_count = u16::from_le_bytes([hurt_window[6], hurt_window[7]]) as u32;

        let shape_end = shape_off + shape_count * SHAPE12_SIZE as u32;
        assert!(
            shape_end <= packed.shapes.len() as u32,
            "Hurt window shape reference out of bounds"
        );
    }

    #[test]
    fn test_pack_moves_empty() {
        let packed = pack_moves(&[], None, None).unwrap();

        assert_eq!(packed.moves.len(), 0);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);
        assert_eq!(packed.cancels.len(), 0);
    }

    #[test]
    fn test_pack_moves_no_hitboxes() {
        let mv = State {
            input: "5L".to_string(),
            name: "Light Punch".to_string(),
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 500,
            hitstun: 12,
            blockstun: 8,
            hitstop: 10,
            guard: GuardType::Mid,
            hitboxes: vec![],  // No hitboxes
            hurtboxes: vec![], // No hurtboxes
            ..Default::default()
        };

        let packed = pack_moves(&[mv], None, None).unwrap();

        assert_eq!(packed.moves.len(), STATE_RECORD_SIZE);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);

        // Move record should have zero-length references
        let record = &packed.moves[0..STATE_RECORD_SIZE];
        let hit_len = u16::from_le_bytes([record[26], record[27]]);
        let hurt_len = u16::from_le_bytes([record[30], record[31]]);
        assert_eq!(hit_len, 0);
        assert_eq!(hurt_len, 0);
    }

    #[test]
    fn test_guard_type_encoding() {
        assert_eq!(guard_type_to_u8(&GuardType::High), 0);
        assert_eq!(guard_type_to_u8(&GuardType::Mid), 1);
        assert_eq!(guard_type_to_u8(&GuardType::Low), 2);
        assert_eq!(guard_type_to_u8(&GuardType::Unblockable), 3);
    }

    #[test]
    fn test_negative_coordinates() {
        let rect = Rect {
            x: -50,
            y: -100,
            w: 30,
            h: 40,
        };
        let shape = pack_shape(&rect);

        // x=-50 -> Q12.4 = -800
        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, -800);

        // y=-100 -> Q12.4 = -1600
        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, -1600);
    }

    // ==========================================================================
    // Roundtrip Tests (Export + Parse via framesmith-fspack reader)
    // ==========================================================================

    /// Roundtrip test: export character data and parse it back with framesmith_fspack.
    ///
    /// This test verifies that:
    /// 1. Exported bytes can be successfully parsed by the reader crate
    /// 2. Move count matches
    /// 3. Keyframes keys exist when moves have animations
    /// 4. String table can be resolved
    #[test]
    fn test_roundtrip_export_and_parse() {
        // Create a character with multiple moves and animations
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_zx_fspack(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // MOVE_EXTRAS is expected when there are moves.
        assert_eq!(pack.section_count(), 9);

        // Verify move count matches
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 3, "move count should match");

        // Verify keyframes keys exist (since all moves have animations)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(
            kf_keys.len(),
            3,
            "should have 3 unique keyframes keys (stand_heavy, stand_light, stand_medium - sorted)"
        );

        // Verify mesh keys exist
        let mesh_keys = pack.mesh_keys().expect("should have MESH_KEYS section");
        assert_eq!(mesh_keys.len(), 3, "should have 3 mesh keys");

        // Verify we can resolve a string from the string table
        // First mesh key should be "test_char.stand_heavy" (sorted alphabetically)
        let (off, len) = mesh_keys.get(0).expect("should get mesh key 0");
        let mesh_key_str = pack
            .string(off, len)
            .expect("should resolve mesh key string");
        assert_eq!(mesh_key_str, "test_char.stand_heavy");

        // First keyframes key should be "stand_heavy" (sorted alphabetically)
        let (kf_off, kf_len) = kf_keys.get(0).expect("should get keyframes key 0");
        let kf_key_str = pack
            .string(kf_off, kf_len)
            .expect("should resolve keyframes key string");
        assert_eq!(kf_key_str, "stand_heavy");
    }

    /// Roundtrip test with a character with hitboxes and hurtboxes.
    #[test]
    fn test_roundtrip_with_hitboxes() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_move_with_hitboxes()],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_zx_fspack(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify move count
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 1);

        // Get the move via the typed view
        let mv = moves.get(0).expect("should get move 0");
        assert_eq!(mv.state_id(), 0);

        // Verify the move has a valid mesh/keyframes key (not KEY_NONE since it has an animation)
        // The move "make_move_with_hitboxes" has animation "stand_light"
        assert_ne!(
            mv.mesh_key(),
            framesmith_fspack::KEY_NONE,
            "mesh_key should not be KEY_NONE"
        );
        assert_ne!(
            mv.keyframes_key(),
            framesmith_fspack::KEY_NONE,
            "keyframes_key should not be KEY_NONE"
        );

        // Verify HIT_WINDOWS section exists and has data
        let hit_windows = pack
            .get_section(framesmith_fspack::SECTION_HIT_WINDOWS)
            .expect("should have HIT_WINDOWS section");
        assert!(
            !hit_windows.is_empty(),
            "HIT_WINDOWS section should have data"
        );

        // Verify HURT_WINDOWS section exists and has data
        let hurt_windows = pack
            .get_section(framesmith_fspack::SECTION_HURT_WINDOWS)
            .expect("should have HURT_WINDOWS section");
        assert!(
            !hurt_windows.is_empty(),
            "HURT_WINDOWS section should have data"
        );

        // Verify SHAPES section exists and has data
        let shapes = pack
            .get_section(framesmith_fspack::SECTION_SHAPES)
            .expect("should have SHAPES section");
        assert!(!shapes.is_empty(), "SHAPES section should have data");
    }

    /// Roundtrip test with empty character (no moves).
    #[test]
    fn test_roundtrip_empty_character() {
        let char_data = CharacterData {
            character: make_test_character("empty"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_zx_fspack(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify section count
        assert_eq!(pack.section_count(), 8);

        // Verify moves section is empty
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 0);
        assert!(moves.is_empty());

        // Verify keyframes keys section is empty
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert!(kf_keys.is_empty());
    }

    /// Roundtrip test verifying animation deduplication.
    #[test]
    fn test_roundtrip_animation_deduplication() {
        // Create character with moves that share animations
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation as 5L
                make_test_move("5M", "stand_medium"),
                make_test_move("2M", "stand_medium"), // Same animation as 5M
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_zx_fspack(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify we have 4 moves
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 4);

        // But only 2 unique animations (stand_light, stand_medium)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(kf_keys.len(), 2, "should have only 2 unique keyframes keys");

        // Verify both keyframes keys can be resolved
        for i in 0..kf_keys.len() {
            let (off, len) = kf_keys.get(i).expect("should get keyframes key");
            let key_str = pack.string(off, len).expect("should resolve string");
            assert!(
                key_str == "stand_light" || key_str == "stand_medium",
                "unexpected keyframes key: {}",
                key_str
            );
        }
    }

    /// Roundtrip test verifying moves with empty animations get KEY_NONE.
    #[test]
    fn test_roundtrip_moves_without_animation() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("idle", ""), // No animation
            ],
            cancel_table: make_empty_cancel_table(),
        };

        // Export to bytes
        let bytes = export_zx_fspack(&char_data).expect("export should succeed");

        // Parse with framesmith_fspack reader
        let pack = framesmith_fspack::PackView::parse(&bytes).expect("parse should succeed");

        // Verify we have 2 moves
        let moves = pack.states().expect("should have MOVES section");
        assert_eq!(moves.len(), 2);

        // But only 1 keyframes key (stand_light)
        let kf_keys = pack
            .keyframes_keys()
            .expect("should have KEYFRAMES_KEYS section");
        assert_eq!(kf_keys.len(), 1);

        // First move (5L) should have a valid key
        let mv0 = moves.get(0).expect("should get move 0");
        assert_ne!(mv0.mesh_key(), framesmith_fspack::KEY_NONE);

        // Second move (idle) should have KEY_NONE since it has no animation
        let mv1 = moves.get(1).expect("should get move 1");
        assert_eq!(mv1.mesh_key(), framesmith_fspack::KEY_NONE);
        assert_eq!(mv1.keyframes_key(), framesmith_fspack::KEY_NONE);
    }

    /// Test that cancel flags are correctly exported to MoveRecord.flags
    #[test]
    fn test_cancel_flags_exported() {
        use crate::commands::CharacterData;

        let mut chains = HashMap::new();
        chains.insert("5L".to_string(), vec!["5M".to_string()]);

        let char_data = CharacterData {
            character: make_test_character("t"),
            moves: vec![
                State {
                    input: "5L".into(),
                    name: "Jab".into(),
                    guard: GuardType::Mid,
                    animation: "a".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
                State {
                    input: "5M".into(),
                    name: "Medium".into(),
                    guard: GuardType::Mid,
                    animation: "b".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
            ],
            cancel_table: CancelTable {
                chains,
                special_cancels: vec!["5L".to_string()],
                super_cancels: vec!["5M".to_string()],
                jump_cancels: vec!["5L".to_string()],
                ..Default::default()
            },
        };

        let bytes = export_zx_fspack(&char_data).unwrap();
        let pack = framesmith_fspack::PackView::parse(&bytes).unwrap();
        let moves = pack.states().unwrap();

        // 5L should have: chain + special + jump flags
        let mv0 = moves.get(0).unwrap();
        assert_eq!(
            mv0.flags() & 0x01,
            0x01,
            "5L should have CHAIN flag (flags=0x{:02x})",
            mv0.flags()
        );
        assert_eq!(
            mv0.flags() & 0x02,
            0x02,
            "5L should have SPECIAL flag (flags=0x{:02x})",
            mv0.flags()
        );
        assert_eq!(
            mv0.flags() & 0x08,
            0x08,
            "5L should have JUMP flag (flags=0x{:02x})",
            mv0.flags()
        );

        // 5M should have: super flag only
        let mv1 = moves.get(1).unwrap();
        assert_eq!(
            mv1.flags() & 0x04,
            0x04,
            "5M should have SUPER flag (flags=0x{:02x})",
            mv1.flags()
        );
        assert_eq!(
            mv1.flags() & 0x01,
            0x00,
            "5M should NOT have CHAIN flag (flags=0x{:02x})",
            mv1.flags()
        );
    }

    /// Test that chain cancel routes are packed into CANCELS_U16 section
    #[test]
    fn test_chain_cancel_routes_exported() {
        use crate::commands::CharacterData;

        // Create chains: 5L -> [5M, 5H], 5M -> [5H]
        let mut chains = HashMap::new();
        chains.insert("5L".to_string(), vec!["5M".to_string(), "5H".to_string()]);
        chains.insert("5M".to_string(), vec!["5H".to_string()]);

        let char_data = CharacterData {
            character: make_test_character("t"),
            moves: vec![
                State {
                    input: "5L".into(),
                    name: "Jab".into(),
                    guard: GuardType::Mid,
                    animation: "a".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
                State {
                    input: "5M".into(),
                    name: "Medium".into(),
                    guard: GuardType::Mid,
                    animation: "b".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
                State {
                    input: "5H".into(),
                    name: "Heavy".into(),
                    guard: GuardType::Mid,
                    animation: "c".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
            ],
            cancel_table: CancelTable {
                chains,
                ..Default::default()
            },
        };

        let bytes = export_zx_fspack(&char_data).unwrap();
        let pack = framesmith_fspack::PackView::parse(&bytes).unwrap();

        // Get raw CANCELS_U16 section
        let cancels = pack
            .get_section(framesmith_fspack::SECTION_CANCELS_U16)
            .expect("CANCELS_U16 section should exist");
        assert!(
            !cancels.is_empty(),
            "CANCELS_U16 should not be empty (got {} bytes)",
            cancels.len()
        );

        // Canonical ordering is by input string ascending.
        // For these inputs, that yields: 5H=0, 5L=1, 5M=2.
        // We expect: 5L has 2 targets (5M=2, 5H=0), 5M has 1 target (5H=0)
        // Total: 3 cancel entries = 6 bytes
        assert_eq!(cancels.len(), 6, "Expected 3 u16 cancel targets = 6 bytes");

        // The first two u16 should be 5L's targets (indices 2 and 0)
        let target0 = u16::from_le_bytes([cancels[0], cancels[1]]);
        let target1 = u16::from_le_bytes([cancels[2], cancels[3]]);
        // 5M is at index 2, 5H is at index 0
        assert_eq!(
            target0, 2,
            "5L's first chain target should be move index 2 (5M)"
        );
        assert_eq!(
            target1, 0,
            "5L's second chain target should be move index 0 (5H)"
        );

        // The third u16 should be 5M's target (index 0)
        let target2 = u16::from_le_bytes([cancels[4], cancels[5]]);
        assert_eq!(target2, 0, "5M's chain target should be move index 0 (5H)");
    }

    /// Test that MoveExtras includes cancel offset/length for each move
    #[test]
    fn test_move_extras_cancel_offsets() {
        use crate::commands::CharacterData;

        // Create chains: 5L -> [5M, 5H], 5M -> [5H]
        let mut chains = HashMap::new();
        chains.insert("5L".to_string(), vec!["5M".to_string(), "5H".to_string()]);
        chains.insert("5M".to_string(), vec!["5H".to_string()]);

        let char_data = CharacterData {
            character: make_test_character("t"),
            moves: vec![
                State {
                    input: "5L".into(),
                    name: "Jab".into(),
                    guard: GuardType::Mid,
                    animation: "a".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
                State {
                    input: "5M".into(),
                    name: "Medium".into(),
                    guard: GuardType::Mid,
                    animation: "b".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
                State {
                    input: "5H".into(),
                    name: "Heavy".into(),
                    guard: GuardType::Mid,
                    animation: "c".into(),
                    pushback: Pushback { hit: 0, block: 0 },
                    meter_gain: MeterGain { hit: 0, whiff: 0 },
                    ..Default::default()
                },
            ],
            cancel_table: CancelTable {
                chains,
                ..Default::default()
            },
        };

        let bytes = export_zx_fspack(&char_data).unwrap();
        let pack = framesmith_fspack::PackView::parse(&bytes).unwrap();

        let extras = pack.state_extras().expect("MOVE_EXTRAS section");
        assert_eq!(extras.len(), 3);

        // Canonical ordering is by input string ascending: 5H=0, 5L=1, 5M=2.
        let ex_h = extras.get(0).expect("extras 5H");
        let (_off_h, len_h) = ex_h.cancels();
        assert_eq!(len_h, 0, "5H should have 0 cancel targets");

        // 5L (index 1) has 2 chain targets at offset 0
        let ex_l = extras.get(1).expect("extras 5L");
        let (off_l, len_l) = ex_l.cancels();
        assert_eq!(off_l, 0, "5L cancel offset should be 0");
        assert_eq!(len_l, 2, "5L should have 2 cancel targets");

        // 5M (index 2) has 1 chain target at offset 4 (after 5L's 2 targets * 2 bytes)
        let ex_m = extras.get(2).expect("extras 5M");
        let (off_m, len_m) = ex_m.cancels();
        assert_eq!(off_m, 4, "5M cancel offset should be 4");
        assert_eq!(len_m, 1, "5M should have 1 cancel target");

        // Verify we can use CancelsView to read targets
        let cancels = pack.cancels().expect("cancels view");

        // Read 5L's targets
        assert_eq!(cancels.get_at(off_l, 0), Some(2), "5L -> 5M (index 2)");
        assert_eq!(cancels.get_at(off_l, 1), Some(0), "5L -> 5H (index 0)");

        // Read 5M's targets
        assert_eq!(cancels.get_at(off_m, 0), Some(0), "5M -> 5H (index 0)");
    }
}
