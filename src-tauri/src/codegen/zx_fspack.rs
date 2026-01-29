//! ZX FSPK binary export adapter
//!
//! Exports character data to the FSPK binary format for use with ZX runtime.

use crate::commands::CharacterData;
use crate::schema::{FrameHitbox, GuardType, Move, Rect};
use std::collections::HashMap;

use super::zx_fspack_format::{
    to_q12_4, to_q12_4_unsigned, write_u16_le, write_u32_le, FLAGS_RESERVED, HEADER_SIZE,
    HIT_WINDOW24_SIZE, HURT_WINDOW12_SIZE, KEY_NONE, MAGIC, MOVE_RECORD_SIZE, SECTION_CANCELS_U16,
    SECTION_HEADER_SIZE, SECTION_HIT_WINDOWS, SECTION_HURT_WINDOWS, SECTION_KEYFRAMES_KEYS,
    SECTION_MESH_KEYS, SECTION_MOVES, SECTION_SHAPES, SECTION_STRING_TABLE, SHAPE12_SIZE,
    SHAPE_KIND_AABB, STRREF_SIZE, VERSION,
};

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
    pub fn intern(&mut self, s: &str) -> (u32, u16) {
        if let Some(&loc) = self.index.get(s) {
            return loc;
        }

        let offset = self.data.len() as u32;
        let len = s.len() as u16;
        self.data.extend_from_slice(s.as_bytes());
        self.index.insert(s.to_string(), (offset, len));
        (offset, len)
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
/// HitWindow24 layout (24 bytes):
/// - frame_start (u8): first active frame
/// - frame_end (u8): last active frame
/// - shape_off (u32): offset into SHAPES section
/// - shape_count (u16): number of shapes (always 1 for v1)
/// - damage (u16): damage value
/// - hitstun (u8): hitstun frames
/// - blockstun (u8): blockstun frames
/// - hitstop (u8): hitstop frames
/// - guard (u8): guard type (0=high, 1=mid, 2=low, 3=unblockable)
/// - reserved (8 bytes): padding for future use
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

    buf[0] = hb.frames.0; // frame_start
    buf[1] = hb.frames.1; // frame_end
    buf[2..6].copy_from_slice(&shapes_off.to_le_bytes()); // shape_off
    buf[6..8].copy_from_slice(&1u16.to_le_bytes()); // shape_count = 1
    buf[8..10].copy_from_slice(&damage.to_le_bytes()); // damage
    buf[10] = hitstun; // hitstun
    buf[11] = blockstun; // blockstun
    buf[12] = hitstop; // hitstop
    buf[13] = guard; // guard
    // bytes 14-23 are reserved/padding (already zeroed)

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

/// Pack a Move into a MoveRecord structure.
///
/// MoveRecord layout (32 bytes):
/// - move_id (u16): move identifier
/// - mesh_key (u16): index into MESH_KEYS or KEY_NONE
/// - keyframes_key (u16): index into KEYFRAMES_KEYS or KEY_NONE
/// - startup (u8): startup frames
/// - active (u8): active frames
/// - recovery (u8): recovery frames
/// - total (u8): total frames (0 = auto-calculated)
/// - hit_windows_off (u32): offset into HIT_WINDOWS section
/// - hit_windows_len (u16): number of hit windows
/// - hurt_windows_off (u32): offset into HURT_WINDOWS section
/// - hurt_windows_len (u16): number of hurt windows
/// - cancels_off (u32): offset into CANCELS_U16 section
/// - cancels_len (u16): number of cancel target move IDs
fn pack_move_record(
    move_id: u16,
    mesh_key: u16,
    keyframes_key: u16,
    mv: &Move,
    hit_windows_off: u32,
    hit_windows_len: u16,
    hurt_windows_off: u32,
    hurt_windows_len: u16,
) -> [u8; MOVE_RECORD_SIZE] {
    let mut buf = [0u8; MOVE_RECORD_SIZE];

    buf[0..2].copy_from_slice(&move_id.to_le_bytes()); // move_id
    buf[2..4].copy_from_slice(&mesh_key.to_le_bytes()); // mesh_key
    buf[4..6].copy_from_slice(&keyframes_key.to_le_bytes()); // keyframes_key
    buf[6] = mv.startup; // startup
    buf[7] = mv.active; // active
    buf[8] = mv.recovery; // recovery
    buf[9] = mv.total.unwrap_or(0); // total
    buf[10..14].copy_from_slice(&hit_windows_off.to_le_bytes()); // hit_windows_off
    buf[14..16].copy_from_slice(&hit_windows_len.to_le_bytes()); // hit_windows_len
    buf[16..20].copy_from_slice(&hurt_windows_off.to_le_bytes()); // hurt_windows_off
    buf[20..22].copy_from_slice(&hurt_windows_len.to_le_bytes()); // hurt_windows_len
    buf[22..26].copy_from_slice(&0u32.to_le_bytes()); // cancels_off (empty for v1)
    buf[26..28].copy_from_slice(&0u16.to_le_bytes()); // cancels_len (empty for v1)
    // bytes 28-31 are reserved/padding (already zeroed)

    buf
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
    /// CANCELS_U16 section: array of u16 move IDs (empty for v1)
    pub cancels: Vec<u8>,
}

/// Pack all moves into binary sections.
///
/// Returns packed move data with all backing arrays.
///
/// The `anim_to_index` map provides indices into the MESH_KEYS/KEYFRAMES_KEYS arrays
/// for each animation name. If None, all moves use KEY_NONE for asset references.
pub fn pack_moves(moves: &[Move], anim_to_index: Option<&HashMap<String, u16>>) -> PackedMoveData {
    let mut packed = PackedMoveData {
        moves: Vec::new(),
        shapes: Vec::new(),
        hit_windows: Vec::new(),
        hurt_windows: Vec::new(),
        cancels: Vec::new(),
    };

    for (idx, mv) in moves.iter().enumerate() {
        let move_id = idx as u16;

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
        let hit_windows_off = packed.hit_windows.len() as u32;
        let hurt_windows_off = packed.hurt_windows.len() as u32;

        // Pack hitboxes -> shapes + hit_windows
        for hb in &mv.hitboxes {
            let shape_off = packed.shapes.len() as u32;
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
            let shape_off = packed.shapes.len() as u32;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed
                .hurt_windows
                .extend_from_slice(&pack_hurt_window(hb, shape_off));
        }

        // Calculate lengths
        let hit_windows_len = mv.hitboxes.len() as u16;
        let hurt_windows_len = mv.hurtboxes.len() as u16;

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
        ));
    }

    packed
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
) -> (Vec<StrRef>, Vec<StrRef>) {
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
        .collect();

    // Build keyframes keys: just the animation name
    let keyframes_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| strings.intern(anim))
        .collect();

    (mesh_keys, keyframes_keys)
}

/// Number of sections in the FSPK format (v1)
const SECTION_COUNT: u32 = 8;

/// Write a string reference (StrRef) to the buffer.
///
/// StrRef layout: offset(u32) + length(u16) + padding(u16)
fn write_strref(buf: &mut Vec<u8>, strref: StrRef) {
    write_u32_le(buf, strref.0); // offset
    write_u16_le(buf, strref.1); // length
    write_u16_le(buf, 0); // padding
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
pub fn export_zx_fspack(char_data: &CharacterData) -> Result<Vec<u8>, String> {
    // Step 1: Build string table and asset keys
    let mut strings = StringTable::new();
    let (mesh_keys, keyframes_keys) = build_asset_keys(char_data, &mut strings);

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

    let anim_to_index: HashMap<String, u16> = animations
        .iter()
        .enumerate()
        .map(|(i, anim)| (anim.to_string(), i as u16))
        .collect();

    // Step 2: Pack moves with animation indices
    let packed = pack_moves(&char_data.moves, Some(&anim_to_index));

    // Step 3: Build section data
    let string_table_data = strings.into_bytes();

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

    // Step 4: Calculate section offsets
    // Layout: Header + Section Headers + Section Data
    let header_and_sections_size = HEADER_SIZE + (SECTION_COUNT as usize * SECTION_HEADER_SIZE);

    let mut current_offset = header_and_sections_size;

    let string_table_off = current_offset as u32;
    let string_table_len = string_table_data.len() as u32;
    current_offset += string_table_data.len();

    let mesh_keys_off = current_offset as u32;
    let mesh_keys_len = mesh_keys_data.len() as u32;
    current_offset += mesh_keys_data.len();

    let keyframes_keys_off = current_offset as u32;
    let keyframes_keys_len = keyframes_keys_data.len() as u32;
    current_offset += keyframes_keys_data.len();

    let moves_off = current_offset as u32;
    let moves_len = packed.moves.len() as u32;
    current_offset += packed.moves.len();

    let hit_windows_off = current_offset as u32;
    let hit_windows_len = packed.hit_windows.len() as u32;
    current_offset += packed.hit_windows.len();

    let hurt_windows_off = current_offset as u32;
    let hurt_windows_len = packed.hurt_windows.len() as u32;
    current_offset += packed.hurt_windows.len();

    let shapes_off = current_offset as u32;
    let shapes_len = packed.shapes.len() as u32;
    current_offset += packed.shapes.len();

    let cancels_off = current_offset as u32;
    let cancels_len = packed.cancels.len() as u32;
    current_offset += packed.cancels.len();

    let total_len = current_offset as u32;

    // Step 5: Build the final binary
    let mut output = Vec::with_capacity(total_len as usize);

    // Write header
    output.extend_from_slice(&MAGIC);
    write_u16_le(&mut output, VERSION);
    write_u16_le(&mut output, FLAGS_RESERVED);
    write_u32_le(&mut output, total_len);
    write_u32_le(&mut output, SECTION_COUNT);

    // Write section headers (8 sections)
    write_section_header(
        &mut output,
        SECTION_STRING_TABLE,
        string_table_off,
        string_table_len,
        1,
    );
    write_section_header(
        &mut output,
        SECTION_MESH_KEYS,
        mesh_keys_off,
        mesh_keys_len,
        4,
    );
    write_section_header(
        &mut output,
        SECTION_KEYFRAMES_KEYS,
        keyframes_keys_off,
        keyframes_keys_len,
        4,
    );
    write_section_header(&mut output, SECTION_MOVES, moves_off, moves_len, 4);
    write_section_header(
        &mut output,
        SECTION_HIT_WINDOWS,
        hit_windows_off,
        hit_windows_len,
        4,
    );
    write_section_header(
        &mut output,
        SECTION_HURT_WINDOWS,
        hurt_windows_off,
        hurt_windows_len,
        4,
    );
    write_section_header(&mut output, SECTION_SHAPES, shapes_off, shapes_len, 4);
    write_section_header(
        &mut output,
        SECTION_CANCELS_U16,
        cancels_off,
        cancels_len,
        2,
    );

    // Write section data
    output.extend_from_slice(&string_table_data);
    output.extend_from_slice(&mesh_keys_data);
    output.extend_from_slice(&keyframes_keys_data);
    output.extend_from_slice(&packed.moves);
    output.extend_from_slice(&packed.hit_windows);
    output.extend_from_slice(&packed.hurt_windows);
    output.extend_from_slice(&packed.shapes);
    output.extend_from_slice(&packed.cancels);

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
        CancelTable, Character, FrameHitbox, GuardType, MeterGain, Move, Pushback, Rect,
    };
    use std::collections::HashMap;

    /// Create a minimal test character.
    fn make_test_character(id: &str) -> Character {
        Character {
            id: id.to_string(),
            name: "Test Character".to_string(),
            archetype: "rushdown".to_string(),
            health: 1000,
            walk_speed: 3.5,
            back_walk_speed: 2.5,
            jump_height: 120,
            jump_duration: 40,
            dash_distance: 80,
            dash_duration: 20,
        }
    }

    /// Create a minimal test move with the given input and animation.
    fn make_test_move(input: &str, animation: &str) -> Move {
        Move {
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
            advanced_hurtboxes: None,
        }
    }

    /// Create an empty cancel table.
    fn make_empty_cancel_table() -> CancelTable {
        CancelTable {
            chains: HashMap::new(),
            special_cancels: vec![],
            super_cancels: vec![],
            jump_cancels: vec![],
        }
    }

    #[test]
    fn test_string_table_intern_returns_same_location_for_same_string() {
        let mut table = StringTable::new();

        let loc1 = table.intern("hello");
        let loc2 = table.intern("hello");

        assert_eq!(loc1, loc2, "Same string should return same location");
    }

    #[test]
    fn test_string_table_intern_different_strings() {
        let mut table = StringTable::new();

        let loc1 = table.intern("hello");
        let loc2 = table.intern("world");

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
        table.intern("abc");
        table.intern("def");

        let bytes = table.into_bytes();
        assert_eq!(bytes, b"abcdef");
    }

    #[test]
    fn test_build_asset_keys_deterministic() {
        // Create character data with moves in non-alphabetical order
        let char_data = CharacterData {
            character: make_test_character("glitch"),
            moves: vec![
                make_test_move("5H", "stand_heavy"),
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings1 = StringTable::new();
        let (mesh_keys1, kf_keys1) = build_asset_keys(&char_data, &mut strings1);

        // Create the same data but with moves in different order
        let char_data2 = CharacterData {
            character: make_test_character("glitch"),
            moves: vec![
                make_test_move("5M", "stand_medium"),
                make_test_move("5L", "stand_light"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings2 = StringTable::new();
        let (mesh_keys2, kf_keys2) = build_asset_keys(&char_data2, &mut strings2);

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
            character: make_test_character("glitch"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation as 5L
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings);

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
        let (mesh_keys, _kf_keys) = build_asset_keys(&char_data, &mut strings);
        let bytes = strings.into_bytes();

        // Verify that strings appear in sorted order: alpha_anim, beta_anim, zebra_anim
        // First mesh key should be "test.alpha_anim" starting at offset 0
        assert_eq!(mesh_keys[0].0, 0);

        // Extract the first mesh key string
        let first_key_start = mesh_keys[0].0 as usize;
        let first_key_len = mesh_keys[0].1 as usize;
        let first_key = std::str::from_utf8(&bytes[first_key_start..first_key_start + first_key_len])
            .unwrap();
        assert_eq!(first_key, "test.alpha_anim");
    }

    #[test]
    fn test_build_asset_keys_mesh_format() {
        let char_data = CharacterData {
            character: make_test_character("glitch"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, _kf_keys) = build_asset_keys(&char_data, &mut strings);
        let bytes = strings.into_bytes();

        // Extract mesh key string
        let mesh_key_start = mesh_keys[0].0 as usize;
        let mesh_key_len = mesh_keys[0].1 as usize;
        let mesh_key =
            std::str::from_utf8(&bytes[mesh_key_start..mesh_key_start + mesh_key_len]).unwrap();

        assert_eq!(mesh_key, "glitch.stand_light");
    }

    #[test]
    fn test_build_asset_keys_keyframes_format() {
        let char_data = CharacterData {
            character: make_test_character("glitch"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (_mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings);
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
            character: make_test_character("glitch"),
            moves: vec![],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings);

        assert!(mesh_keys.is_empty());
        assert!(kf_keys.is_empty());
        assert!(strings.is_empty());
    }

    #[test]
    fn test_build_asset_keys_skips_empty_animations() {
        let char_data = CharacterData {
            character: make_test_character("glitch"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("5M", ""), // Empty animation should be skipped
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings);

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

        // Verify version
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        assert_eq!(version, 1, "Version should be 1");

        // Verify flags
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
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
        assert_eq!(section_count, 8, "Section count should be 8");
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
        assert!(result.is_ok(), "export_zx_fspack should succeed with no moves");

        let bytes = result.unwrap();

        // Should still have valid FSPK header
        assert_eq!(&bytes[0..4], b"FSPK");

        // Section count should still be 8
        let section_count = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        assert_eq!(section_count, 8);
    }

    #[test]
    fn test_export_zx_fspack_section_headers() {
        let char_data = CharacterData {
            character: make_test_character("test"),
            moves: vec![make_test_move("5L", "stand_light")],
            cancel_table: make_empty_cancel_table(),
        };

        let bytes = export_zx_fspack(&char_data).unwrap();

        // Verify we can read all 8 section headers
        let header_end = HEADER_SIZE + (8 * SECTION_HEADER_SIZE);
        assert!(
            bytes.len() >= header_end,
            "Output should have room for all section headers"
        );

        // Check that section kinds are correct (in order)
        let expected_kinds = [1, 2, 3, 4, 5, 6, 7, 8]; // STRING_TABLE through CANCELS_U16
        for (i, &expected_kind) in expected_kinds.iter().enumerate() {
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

    fn make_move_with_hitboxes() -> Move {
        Move {
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
            advanced_hurtboxes: None,
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

        let shape_off = u32::from_le_bytes([hw[2], hw[3], hw[4], hw[5]]);
        assert_eq!(shape_off, 100);

        let shape_count = u16::from_le_bytes([hw[6], hw[7]]);
        assert_eq!(shape_count, 1);

        let damage = u16::from_le_bytes([hw[8], hw[9]]);
        assert_eq!(damage, 500);

        assert_eq!(hw[10], 12); // hitstun
        assert_eq!(hw[11], 8); // blockstun
        assert_eq!(hw[12], 10); // hitstop
        assert_eq!(hw[13], 1); // guard (mid)
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
        let mr = pack_move_record(42, KEY_NONE, KEY_NONE, &mv, 100, 2, 200, 3);

        assert_eq!(mr.len(), MOVE_RECORD_SIZE);

        let move_id = u16::from_le_bytes([mr[0], mr[1]]);
        assert_eq!(move_id, 42);

        let mesh_key = u16::from_le_bytes([mr[2], mr[3]]);
        assert_eq!(mesh_key, KEY_NONE);

        let keyframes_key = u16::from_le_bytes([mr[4], mr[5]]);
        assert_eq!(keyframes_key, KEY_NONE);

        assert_eq!(mr[6], 5); // startup
        assert_eq!(mr[7], 3); // active
        assert_eq!(mr[8], 10); // recovery
        assert_eq!(mr[9], 0); // total (not set)

        let hit_off = u32::from_le_bytes([mr[10], mr[11], mr[12], mr[13]]);
        assert_eq!(hit_off, 100);

        let hit_len = u16::from_le_bytes([mr[14], mr[15]]);
        assert_eq!(hit_len, 2);

        let hurt_off = u32::from_le_bytes([mr[16], mr[17], mr[18], mr[19]]);
        assert_eq!(hurt_off, 200);

        let hurt_len = u16::from_le_bytes([mr[20], mr[21]]);
        assert_eq!(hurt_len, 3);
    }

    #[test]
    fn test_pack_moves_count_matches() {
        let moves = vec![make_move_with_hitboxes(), make_move_with_hitboxes()];
        let packed = pack_moves(&moves, None);

        // Move count matches input
        let move_count = packed.moves.len() / MOVE_RECORD_SIZE;
        assert_eq!(move_count, 2);

        // MOVES section length is correct
        assert_eq!(packed.moves.len(), 2 * MOVE_RECORD_SIZE);
    }

    #[test]
    fn test_pack_moves_section_sizes() {
        let mv = make_move_with_hitboxes();
        let moves = vec![mv];
        let packed = pack_moves(&moves, None);

        // 1 move with 1 hitbox and 1 hurtbox
        assert_eq!(packed.moves.len(), 1 * MOVE_RECORD_SIZE);
        assert_eq!(packed.shapes.len(), 2 * SHAPE12_SIZE); // 1 hit + 1 hurt shape
        assert_eq!(packed.hit_windows.len(), 1 * HIT_WINDOW24_SIZE);
        assert_eq!(packed.hurt_windows.len(), 1 * HURT_WINDOW12_SIZE);
        assert_eq!(packed.cancels.len(), 0); // empty for v1
    }

    #[test]
    fn test_pack_moves_references_valid() {
        let moves = vec![make_move_with_hitboxes(), make_move_with_hitboxes()];
        let packed = pack_moves(&moves, None);

        // Verify each move record has valid references
        for i in 0..2 {
            let offset = i * MOVE_RECORD_SIZE;
            let record = &packed.moves[offset..offset + MOVE_RECORD_SIZE];

            // Extract hit_windows offset and length
            let hit_off = u32::from_le_bytes([record[10], record[11], record[12], record[13]]);
            let hit_len = u16::from_le_bytes([record[14], record[15]]) as u32;

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

            // Extract hurt_windows offset and length
            let hurt_off = u32::from_le_bytes([record[16], record[17], record[18], record[19]]);
            let hurt_len = u16::from_le_bytes([record[20], record[21]]) as u32;

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
        let packed = pack_moves(&[mv], None);

        // Verify hit window shape reference
        let hit_window = &packed.hit_windows[0..HIT_WINDOW24_SIZE];
        let shape_off =
            u32::from_le_bytes([hit_window[2], hit_window[3], hit_window[4], hit_window[5]]);
        let shape_count = u16::from_le_bytes([hit_window[6], hit_window[7]]) as u32;

        let shape_end = shape_off + shape_count * SHAPE12_SIZE as u32;
        assert!(
            shape_end <= packed.shapes.len() as u32,
            "Hit window shape reference out of bounds"
        );

        // Verify hurt window shape reference
        let hurt_window = &packed.hurt_windows[0..HURT_WINDOW12_SIZE];
        let shape_off =
            u32::from_le_bytes([hurt_window[2], hurt_window[3], hurt_window[4], hurt_window[5]]);
        let shape_count = u16::from_le_bytes([hurt_window[6], hurt_window[7]]) as u32;

        let shape_end = shape_off + shape_count * SHAPE12_SIZE as u32;
        assert!(
            shape_end <= packed.shapes.len() as u32,
            "Hurt window shape reference out of bounds"
        );
    }

    #[test]
    fn test_pack_moves_empty() {
        let packed = pack_moves(&[], None);

        assert_eq!(packed.moves.len(), 0);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);
        assert_eq!(packed.cancels.len(), 0);
    }

    #[test]
    fn test_pack_moves_no_hitboxes() {
        let mv = Move {
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
            hitboxes: vec![], // No hitboxes
            hurtboxes: vec![], // No hurtboxes
            ..Default::default()
        };

        let packed = pack_moves(&[mv], None);

        assert_eq!(packed.moves.len(), MOVE_RECORD_SIZE);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);

        // Move record should have zero-length references
        let record = &packed.moves[0..MOVE_RECORD_SIZE];
        let hit_len = u16::from_le_bytes([record[14], record[15]]);
        let hurt_len = u16::from_le_bytes([record[20], record[21]]);
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
            character: make_test_character("glitch"),
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

        // Verify section count (8 sections)
        assert_eq!(pack.section_count(), 8);

        // Verify move count matches
        let moves = pack.moves().expect("should have MOVES section");
        assert_eq!(moves.len(), 3, "move count should match");

        // Verify keyframes keys exist (since all moves have animations)
        let kf_keys = pack.keyframes_keys().expect("should have KEYFRAMES_KEYS section");
        assert_eq!(
            kf_keys.len(),
            3,
            "should have 3 unique keyframes keys (stand_heavy, stand_light, stand_medium - sorted)"
        );

        // Verify mesh keys exist
        let mesh_keys = pack.mesh_keys().expect("should have MESH_KEYS section");
        assert_eq!(mesh_keys.len(), 3, "should have 3 mesh keys");

        // Verify we can resolve a string from the string table
        // First mesh key should be "glitch.stand_heavy" (sorted alphabetically)
        let (off, len) = mesh_keys.get(0).expect("should get mesh key 0");
        let mesh_key_str = pack.string(off, len).expect("should resolve mesh key string");
        assert_eq!(mesh_key_str, "glitch.stand_heavy");

        // First keyframes key should be "stand_heavy" (sorted alphabetically)
        let (kf_off, kf_len) = kf_keys.get(0).expect("should get keyframes key 0");
        let kf_key_str = pack.string(kf_off, kf_len).expect("should resolve keyframes key string");
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
        let moves = pack.moves().expect("should have MOVES section");
        assert_eq!(moves.len(), 1);

        // Get the move via the typed view
        let mv = moves.get(0).expect("should get move 0");
        assert_eq!(mv.move_id(), 0);

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
        let moves = pack.moves().expect("should have MOVES section");
        assert_eq!(moves.len(), 0);
        assert!(moves.is_empty());

        // Verify keyframes keys section is empty
        let kf_keys = pack.keyframes_keys().expect("should have KEYFRAMES_KEYS section");
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
        let moves = pack.moves().expect("should have MOVES section");
        assert_eq!(moves.len(), 4);

        // But only 2 unique animations (stand_light, stand_medium)
        let kf_keys = pack.keyframes_keys().expect("should have KEYFRAMES_KEYS section");
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
        let moves = pack.moves().expect("should have MOVES section");
        assert_eq!(moves.len(), 2);

        // But only 1 keyframes key (stand_light)
        let kf_keys = pack.keyframes_keys().expect("should have KEYFRAMES_KEYS section");
        assert_eq!(kf_keys.len(), 1);

        // First move (5L) should have a valid key
        let mv0 = moves.get(0).expect("should get move 0");
        assert_ne!(mv0.mesh_key(), framesmith_fspack::KEY_NONE);

        // Second move (idle) should have KEY_NONE since it has no animation
        let mv1 = moves.get(1).expect("should get move 1");
        assert_eq!(mv1.mesh_key(), framesmith_fspack::KEY_NONE);
        assert_eq!(mv1.keyframes_key(), framesmith_fspack::KEY_NONE);
    }
}
