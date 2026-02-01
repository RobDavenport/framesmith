//! Zero-copy view into an FSPK pack.

use crate::bytes::{
    read_f32_le, read_i32_le, read_i64_le, read_u16_le, read_u32_le, read_u64_le, read_u8,
};
use crate::error::Error;

/// Magic bytes identifying an FSPK file.
pub const MAGIC: [u8; 4] = [b'F', b'S', b'P', b'K'];

/// Size of the main header in bytes.
/// Layout: magic(4) + flags(4) + total_len(4) + section_count(4)
pub const HEADER_SIZE: usize = 16;

/// Header field offsets.
pub const HEADER_MAGIC_OFF: usize = 0;
pub const HEADER_FLAGS_OFF: usize = 4;
pub const HEADER_TOTAL_LEN_OFF: usize = 8;
pub const HEADER_SECTION_COUNT_OFF: usize = 12;

/// Size of each section header in bytes.
/// Layout: kind(4) + offset(4) + len(4) + align(4)
pub const SECTION_HEADER_SIZE: usize = 16;

/// Maximum number of sections supported.
pub const MAX_SECTIONS: usize = 24;  // Increased for tag and cancel rule sections

// =============================================================================
// Section Kind Constants
// =============================================================================

/// Raw UTF-8 string data, referenced by (off, len) pairs
pub const SECTION_STRING_TABLE: u32 = 1;

/// Array of StrRef pointing to mesh asset keys
pub const SECTION_MESH_KEYS: u32 = 2;

/// Array of StrRef pointing to keyframes asset keys
pub const SECTION_KEYFRAMES_KEYS: u32 = 3;

/// Array of StateRecord structs
pub const SECTION_STATES: u32 = 4;

/// Array of HitWindow24 structs
pub const SECTION_HIT_WINDOWS: u32 = 5;

/// Array of HurtWindow12 structs
pub const SECTION_HURT_WINDOWS: u32 = 6;

/// Array of Shape12 structs (hitboxes/hurtboxes geometry)
pub const SECTION_SHAPES: u32 = 7;

/// Array of u16 move IDs for cancel targets
pub const SECTION_CANCELS_U16: u32 = 8;

/// Array of ResourceDef12 structs
pub const SECTION_RESOURCE_DEFS: u32 = 9;

/// Array of StateExtras structs (parallel to STATES)
pub const SECTION_STATE_EXTRAS: u32 = 10;

/// Array of EventEmit16 structs
pub const SECTION_EVENT_EMITS: u32 = 11;

/// Array of EventArg20 structs
pub const SECTION_EVENT_ARGS: u32 = 12;

/// Array of MoveNotify12 structs
pub const SECTION_MOVE_NOTIFIES: u32 = 13;

/// Array of MoveResourceCost12 structs
pub const SECTION_MOVE_RESOURCE_COSTS: u32 = 14;

/// Array of MoveResourcePrecondition12 structs
pub const SECTION_MOVE_RESOURCE_PRECONDITIONS: u32 = 15;

/// Array of MoveResourceDelta16 structs
pub const SECTION_MOVE_RESOURCE_DELTAS: u32 = 16;

/// Array of StateTagRange8 structs (parallel to MOVES)
pub const SECTION_STATE_TAG_RANGES: u32 = 17;

/// Array of StrRef pointing to tag strings
pub const SECTION_STATE_TAGS: u32 = 18;

/// Section containing tag-based cancel rules
pub const SECTION_CANCEL_TAG_RULES: u32 = 19;

/// Section containing explicit deny pairs
pub const SECTION_CANCEL_DENIES: u32 = 20;

/// Array of CharacterProp12 structs
pub const SECTION_CHARACTER_PROPS: u32 = 21;

/// Array of PushWindow12 structs (body collision)
pub const SECTION_PUSH_WINDOWS: u32 = 22;

// =============================================================================
// Structure Sizes
// =============================================================================

/// String reference size: off(4) + len(2) + pad(2)
pub const STRREF_SIZE: usize = 8;

/// State record size (see StateRecord in module docs)
pub const STATE_RECORD_SIZE: usize = 36;

/// ResourceDef record size
pub const RESOURCE_DEF_SIZE: usize = 12;

/// StateExtras record size (expanded from 64 to 72 for cancel offset/length)
pub const STATE_EXTRAS_SIZE: usize = 72;

/// EventEmit record size
pub const EVENT_EMIT_SIZE: usize = 16;

/// EventArg record size
pub const EVENT_ARG_SIZE: usize = 20;

/// MoveNotify record size
pub const MOVE_NOTIFY_SIZE: usize = 12;

/// MoveResourceCost record size
pub const MOVE_RESOURCE_COST_SIZE: usize = 12;

/// MoveResourcePrecondition record size
pub const MOVE_RESOURCE_PRECONDITION_SIZE: usize = 12;

/// MoveResourceDelta record size
pub const MOVE_RESOURCE_DELTA_SIZE: usize = 16;

/// StateTagRange record size: off(4) + count(2) + pad(2) = 8 bytes
pub const STATE_TAG_RANGE_SIZE: usize = 8;

/// CancelTagRule record size (24 bytes)
pub const CANCEL_TAG_RULE_SIZE: usize = 24;

/// CancelDeny record size (4 bytes: from u16, to u16)
pub const CANCEL_DENY_SIZE: usize = 4;

/// HitWindow record size (24 bytes)
pub const HIT_WINDOW_SIZE: usize = 24;

/// HurtWindow record size (12 bytes)
pub const HURT_WINDOW_SIZE: usize = 12;

/// Shape record size (12 bytes)
pub const SHAPE_SIZE: usize = 12;

/// Character property record size (12 bytes)
/// Layout: name_off(4) + name_len(2) + value_type(1) + pad(1) + value(4)
pub const CHARACTER_PROP_SIZE: usize = 12;

/// PushWindow record size (12 bytes) - same layout as HurtWindow
pub const PUSH_WINDOW_SIZE: usize = 12;

// =============================================================================
// Shape Type Constants
// =============================================================================

/// Shape type: axis-aligned bounding box
pub const SHAPE_KIND_AABB: u8 = 0;

/// Shape type: rotated rectangle
pub const SHAPE_KIND_RECT: u8 = 1;

/// Shape type: circle
pub const SHAPE_KIND_CIRCLE: u8 = 2;

/// Shape type: capsule (two endpoints + radius)
pub const SHAPE_KIND_CAPSULE: u8 = 3;

// =============================================================================
// Sentinel Values
// =============================================================================

/// Sentinel value for "no mesh" or "no keyframes" (u16::MAX)
pub const KEY_NONE: u16 = 0xFFFF;

/// Sentinel value for an absent optional u16.
pub const OPT_U16_NONE: u16 = 0xFFFF;

// =============================================================================
// Event Arg Tags
// =============================================================================

pub const EVENT_ARG_TAG_BOOL: u8 = 0;
pub const EVENT_ARG_TAG_I64: u8 = 1;
pub const EVENT_ARG_TAG_F32: u8 = 2;
pub const EVENT_ARG_TAG_STRING: u8 = 3;

// =============================================================================
// Resource Delta Trigger Tags
// =============================================================================

pub const RESOURCE_DELTA_TRIGGER_ON_USE: u8 = 0;
pub const RESOURCE_DELTA_TRIGGER_ON_HIT: u8 = 1;
pub const RESOURCE_DELTA_TRIGGER_ON_BLOCK: u8 = 2;

/// Information about a single section in the pack.
#[derive(Debug, Clone, Copy, Default)]
struct SectionInfo {
    kind: u32,
    offset: u32,
    len: u32,
    #[allow(dead_code)]
    align: u32,
}

/// A zero-copy view into an FSPK binary pack.
///
/// This struct provides read-only access to the pack's contents without
/// allocating memory. All data is read directly from the underlying byte slice.
pub struct PackView<'a> {
    data: &'a [u8],
    sections: [SectionInfo; MAX_SECTIONS],
    section_count: usize,
}

impl<'a> PackView<'a> {
    /// Parse the given bytes as an FSPK pack.
    ///
    /// Returns a `PackView` that provides zero-copy access to the pack contents.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The data is too short to contain a valid header (`TooShort`)
    /// - The magic bytes are incorrect (`InvalidMagic`)
    /// - Section headers or data are out of bounds (`OutOfBounds`)
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        // Check minimum length for header
        if bytes.len() < HEADER_SIZE {
            return Err(Error::TooShort);
        }

        // Validate magic bytes
        if bytes[HEADER_MAGIC_OFF..HEADER_MAGIC_OFF + 4] != MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Read header fields
        // flags at offset 4 (u32) - currently unused
        let _flags = read_u32_le(bytes, HEADER_FLAGS_OFF).ok_or(Error::TooShort)?;

        let total_len = read_u32_le(bytes, HEADER_TOTAL_LEN_OFF).ok_or(Error::TooShort)? as usize;
        let section_count =
            read_u32_le(bytes, HEADER_SECTION_COUNT_OFF).ok_or(Error::TooShort)? as usize;

        // Validate total_len matches actual data length
        if total_len > bytes.len() {
            return Err(Error::OutOfBounds);
        }

        // Validate section count
        if section_count > MAX_SECTIONS {
            return Err(Error::OutOfBounds);
        }

        // Calculate where section headers end
        let section_table_size = section_count
            .checked_mul(SECTION_HEADER_SIZE)
            .ok_or(Error::OutOfBounds)?;
        let section_table_end = HEADER_SIZE
            .checked_add(section_table_size)
            .ok_or(Error::OutOfBounds)?;

        // Validate section table fits within declared total_len.
        if section_table_end > total_len {
            return Err(Error::OutOfBounds);
        }

        // Validate section table fits within data
        if section_table_end > bytes.len() {
            return Err(Error::OutOfBounds);
        }

        // Parse section headers
        let mut sections = [SectionInfo::default(); MAX_SECTIONS];
        for i in 0..section_count {
            let header_offset = HEADER_SIZE + i * SECTION_HEADER_SIZE;

            let kind = read_u32_le(bytes, header_offset).ok_or(Error::OutOfBounds)?;
            let offset = read_u32_le(bytes, header_offset + 4).ok_or(Error::OutOfBounds)?;
            let len = read_u32_le(bytes, header_offset + 8).ok_or(Error::OutOfBounds)?;
            let align = read_u32_le(bytes, header_offset + 12).ok_or(Error::OutOfBounds)?;

            // Validate section data fits within total_len
            let section_end = (offset as usize)
                .checked_add(len as usize)
                .ok_or(Error::OutOfBounds)?;
            if section_end > total_len {
                return Err(Error::OutOfBounds);
            }

            sections[i] = SectionInfo {
                kind,
                offset,
                len,
                align,
            };
        }

        Ok(Self {
            data: bytes,
            sections,
            section_count,
        })
    }

    /// Get the data for a section with the given kind.
    ///
    /// Returns `None` if no section with that kind exists.
    pub fn get_section(&self, kind: u32) -> Option<&'a [u8]> {
        for i in 0..self.section_count {
            if self.sections[i].kind == kind {
                let offset = self.sections[i].offset as usize;
                let len = self.sections[i].len as usize;
                return Some(&self.data[offset..offset + len]);
            }
        }
        None
    }

    /// Returns the number of sections in the pack.
    pub fn section_count(&self) -> usize {
        self.section_count
    }

    /// Get a string from the string table by offset and length.
    ///
    /// Returns `None` if:
    /// - No string table section exists
    /// - The offset/length is out of bounds
    /// - The bytes are not valid UTF-8
    pub fn string(&self, off: u32, len: u16) -> Option<&'a str> {
        let table = self.get_section(SECTION_STRING_TABLE)?;
        let start = off as usize;
        let end = start.checked_add(len as usize)?;
        if end > table.len() {
            return None;
        }
        core::str::from_utf8(&table[start..end]).ok()
    }

    /// Get mesh keys section as a typed view.
    ///
    /// Returns `None` if no mesh keys section exists.
    pub fn mesh_keys(&self) -> Option<MeshKeysView<'a>> {
        let data = self.get_section(SECTION_MESH_KEYS)?;
        Some(MeshKeysView { data })
    }

    /// Get keyframes keys section as a typed view.
    ///
    /// Returns `None` if no keyframes keys section exists.
    pub fn keyframes_keys(&self) -> Option<KeyframesKeysView<'a>> {
        let data = self.get_section(SECTION_KEYFRAMES_KEYS)?;
        Some(KeyframesKeysView { data })
    }

    /// Get states section as a typed view.
    ///
    /// Returns `None` if no states section exists.
    pub fn states(&self) -> Option<StatesView<'a>> {
        let data = self.get_section(SECTION_STATES)?;
        Some(StatesView { data })
    }

    /// Get resource definitions as a typed view.
    pub fn resource_defs(&self) -> Option<ResourceDefsView<'a>> {
        let data = self.get_section(SECTION_RESOURCE_DEFS)?;
        Some(ResourceDefsView { data })
    }

    /// Get per-state extras as a typed view.
    pub fn state_extras(&self) -> Option<StateExtrasView<'a>> {
        let data = self.get_section(SECTION_STATE_EXTRAS)?;
        Some(StateExtrasView { data })
    }

    /// Find a state by input notation (e.g., "5L", "236P").
    ///
    /// Returns the state index and view if found.
    pub fn find_state_by_input(&self, input: &str) -> Option<(usize, StateView<'a>)> {
        let states = self.states()?;
        let extras = self.state_extras()?;

        for i in 0..states.len() {
            let ex = extras.get(i)?;
            let (off, len) = ex.input();
            if let Some(state_input) = self.string(off, len) {
                if state_input == input {
                    return Some((i, states.get(i)?));
                }
            }
        }

        None
    }

    /// Get event emits as a typed view.
    pub fn event_emits(&self) -> Option<EventEmitsView<'a>> {
        let data = self.get_section(SECTION_EVENT_EMITS)?;
        Some(EventEmitsView { data })
    }

    /// Get event args as a typed view.
    pub fn event_args(&self) -> Option<EventArgsView<'a>> {
        let data = self.get_section(SECTION_EVENT_ARGS)?;
        Some(EventArgsView { data })
    }

    /// Get move notifies as a typed view.
    pub fn move_notifies(&self) -> Option<MoveNotifiesView<'a>> {
        let data = self.get_section(SECTION_MOVE_NOTIFIES)?;
        Some(MoveNotifiesView { data })
    }

    /// Get move resource costs as a typed view.
    pub fn move_resource_costs(&self) -> Option<MoveResourceCostsView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_COSTS)?;
        Some(MoveResourceCostsView { data })
    }

    /// Get move resource preconditions as a typed view.
    pub fn move_resource_preconditions(&self) -> Option<MoveResourcePreconditionsView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_PRECONDITIONS)?;
        Some(MoveResourcePreconditionsView { data })
    }

    /// Get move resource deltas as a typed view.
    pub fn move_resource_deltas(&self) -> Option<MoveResourceDeltasView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_DELTAS)?;
        Some(MoveResourceDeltasView { data })
    }

    /// Get cancel targets as a typed view.
    ///
    /// Returns `None` if no cancels section exists.
    pub fn cancels(&self) -> Option<CancelsView<'a>> {
        let data = self.get_section(SECTION_CANCELS_U16)?;
        Some(CancelsView { data })
    }

    /// Get hit windows section as a typed view.
    ///
    /// Returns `None` if no hit windows section exists.
    pub fn hit_windows(&self) -> Option<HitWindowsView<'a>> {
        let data = self.get_section(SECTION_HIT_WINDOWS)?;
        Some(HitWindowsView { data })
    }

    /// Get hurt windows section as a typed view.
    ///
    /// Returns `None` if no hurt windows section exists.
    pub fn hurt_windows(&self) -> Option<HurtWindowsView<'a>> {
        let data = self.get_section(SECTION_HURT_WINDOWS)?;
        Some(HurtWindowsView { data })
    }

    /// Get push windows section as a typed view.
    ///
    /// Returns `None` if no push windows section exists.
    pub fn push_windows(&self) -> Option<PushWindowsView<'a>> {
        let data = self.get_section(SECTION_PUSH_WINDOWS)?;
        Some(PushWindowsView { data })
    }

    /// Get shapes section as a typed view.
    ///
    /// Returns `None` if no shapes section exists.
    pub fn shapes(&self) -> Option<ShapesView<'a>> {
        let data = self.get_section(SECTION_SHAPES)?;
        Some(ShapesView { data })
    }

    /// Get the state tag ranges section as a typed view.
    ///
    /// Returns `None` if no STATE_TAG_RANGES section exists.
    pub fn state_tag_ranges(&self) -> Option<StateTagRangesView<'a>> {
        let data = self.get_section(SECTION_STATE_TAG_RANGES)?;
        Some(StateTagRangesView { data })
    }

    /// Get tags for a state/move by its index.
    ///
    /// Returns an iterator over the tag strings for the given state,
    /// or `None` if tag sections don't exist or the index is invalid.
    pub fn state_tags(&self, state_idx: usize) -> Option<impl Iterator<Item = &'a str> + 'a> {
        let ranges = self.state_tag_ranges()?;
        let (off, count) = ranges.get(state_idx)?;
        let tags_section = self.get_section(SECTION_STATE_TAGS)?;
        let string_table = self.get_section(SECTION_STRING_TABLE)?;

        Some((0..count).filter_map(move |i| {
            let tag_offset = (off as usize) + (i as usize) * STRREF_SIZE;
            if tag_offset + STRREF_SIZE > tags_section.len() {
                return None;
            }
            let str_off = read_u32_le(tags_section, tag_offset)?;
            let str_len = read_u16_le(tags_section, tag_offset + 4)?;
            // Resolve string from string table
            let start = str_off as usize;
            let end = start.checked_add(str_len as usize)?;
            if end > string_table.len() {
                return None;
            }
            core::str::from_utf8(&string_table[start..end]).ok()
        }))
    }

    /// Get the cancel tag rules section as a typed view.
    ///
    /// Returns `None` if no CANCEL_TAG_RULES section exists.
    pub fn cancel_tag_rules(&'a self) -> Option<CancelTagRulesView<'a>> {
        let data = self.get_section(SECTION_CANCEL_TAG_RULES)?;
        Some(CancelTagRulesView { data, pack: self })
    }

    /// Get the cancel denies section as raw bytes.
    ///
    /// Returns `None` if no CANCEL_DENIES section exists.
    pub fn cancel_denies(&self) -> Option<&'a [u8]> {
        self.get_section(SECTION_CANCEL_DENIES)
    }

    /// Check if a specific cancel is denied.
    ///
    /// Searches the deny list for a matching (from, to) pair.
    pub fn has_cancel_deny(&self, from: u16, to: u16) -> bool {
        let Some(denies) = self.cancel_denies() else {
            return false;
        };
        let count = denies.len() / CANCEL_DENY_SIZE;
        for i in 0..count {
            let off = i * CANCEL_DENY_SIZE;
            let deny_from = read_u16_le(denies, off).unwrap_or(0xFFFF);
            let deny_to = read_u16_le(denies, off + 2).unwrap_or(0xFFFF);
            if deny_from == from && deny_to == to {
                return true;
            }
        }
        false
    }

    /// Get character properties section as a typed view.
    ///
    /// Returns `None` if no CHARACTER_PROPS section exists.
    pub fn character_props(&self) -> Option<CharacterPropsView<'a>> {
        let data = self.get_section(SECTION_CHARACTER_PROPS)?;
        Some(CharacterPropsView { data })
    }
}

// =============================================================================
// Typed Views
// =============================================================================

/// Zero-copy view over the mesh keys section.
///
/// Each entry is a StrRef (8 bytes): off(4) + len(2) + pad(2)
#[derive(Clone, Copy)]
pub struct MeshKeysView<'a> {
    data: &'a [u8],
}

impl<'a> MeshKeysView<'a> {
    /// Returns the number of mesh keys in this section.
    pub fn len(&self) -> usize {
        self.data.len() / STRREF_SIZE
    }

    /// Returns true if there are no mesh keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the offset and length for a mesh key at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let base = index.checked_mul(STRREF_SIZE)?;
        if base + STRREF_SIZE > self.data.len() {
            return None;
        }
        let off = read_u32_le(self.data, base)?;
        let len = read_u16_le(self.data, base + 4)?;
        Some((off, len))
    }
}

/// Zero-copy view over the keyframes keys section.
///
/// Each entry is a StrRef (8 bytes): off(4) + len(2) + pad(2)
#[derive(Clone, Copy)]
pub struct KeyframesKeysView<'a> {
    data: &'a [u8],
}

impl<'a> KeyframesKeysView<'a> {
    /// Returns the number of keyframes keys in this section.
    pub fn len(&self) -> usize {
        self.data.len() / STRREF_SIZE
    }

    /// Returns true if there are no keyframes keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the offset and length for a keyframes key at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let base = index.checked_mul(STRREF_SIZE)?;
        if base + STRREF_SIZE > self.data.len() {
            return None;
        }
        let off = read_u32_le(self.data, base)?;
        let len = read_u16_le(self.data, base + 4)?;
        Some((off, len))
    }
}

/// Zero-copy view over the states section.
///
/// Each entry is a StateRecord (36 bytes).
#[derive(Clone, Copy)]
pub struct StatesView<'a> {
    data: &'a [u8],
}

impl<'a> StatesView<'a> {
    /// Returns the number of states in this section.
    pub fn len(&self) -> usize {
        self.data.len() / STATE_RECORD_SIZE
    }

    /// Returns true if there are no states.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a view of the state record at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<StateView<'a>> {
        let base = index.checked_mul(STATE_RECORD_SIZE)?;
        let end = base.checked_add(STATE_RECORD_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(StateView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single state record (36 bytes).
///
/// Layout:
/// - 0-1: state_id (u16)
/// - 2-3: mesh_key (u16)
/// - 4-5: keyframes_key (u16)
/// - 6: state_type (u8)
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
/// - 32-33: push_windows_off (u16)
/// - 34-35: push_windows_len (u16)
#[derive(Clone, Copy)]
pub struct StateView<'a> {
    data: &'a [u8],
}

/// Decoded cancel flags from state flags byte.
#[derive(Debug, Clone, Copy, Default)]
pub struct CancelFlags {
    pub chain: bool,
    pub special: bool,
    pub super_cancel: bool,
    pub jump: bool,
    pub self_gatling: bool,
}

impl<'a> StateView<'a> {
    /// Returns the state ID (index in the states array).
    pub fn state_id(&self) -> u16 {
        read_u16_le(self.data, 0).unwrap_or(0)
    }

    /// Returns the mesh key index, or KEY_NONE (0xFFFF) if no mesh.
    pub fn mesh_key(&self) -> u16 {
        read_u16_le(self.data, 2).unwrap_or(KEY_NONE)
    }

    /// Returns the keyframes key index, or KEY_NONE (0xFFFF) if no keyframes.
    pub fn keyframes_key(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(KEY_NONE)
    }

    /// Returns the state type.
    pub fn state_type(&self) -> u8 {
        read_u8(self.data, 6).unwrap_or(0)
    }

    /// Returns the trigger type.
    pub fn trigger(&self) -> u8 {
        read_u8(self.data, 7).unwrap_or(0)
    }

    /// Returns the guard type.
    pub fn guard(&self) -> u8 {
        read_u8(self.data, 8).unwrap_or(0)
    }

    /// Returns the state flags.
    pub fn flags(&self) -> u8 {
        read_u8(self.data, 9).unwrap_or(0)
    }

    /// Decode cancel flags from the flags byte.
    pub fn cancel_flags(&self) -> CancelFlags {
        let f = self.flags();
        CancelFlags {
            chain: f & 0x01 != 0,
            special: f & 0x02 != 0,
            super_cancel: f & 0x04 != 0,
            jump: f & 0x08 != 0,
            self_gatling: f & 0x10 != 0,
        }
    }

    /// Returns the startup frames.
    pub fn startup(&self) -> u8 {
        read_u8(self.data, 10).unwrap_or(0)
    }

    /// Returns the active frames.
    pub fn active(&self) -> u8 {
        read_u8(self.data, 11).unwrap_or(0)
    }

    /// Returns the recovery frames.
    pub fn recovery(&self) -> u8 {
        read_u8(self.data, 12).unwrap_or(0)
    }

    /// Returns the total frame count.
    pub fn total(&self) -> u16 {
        read_u16_le(self.data, 14).unwrap_or(0)
    }

    /// Returns the damage value.
    pub fn damage(&self) -> u16 {
        read_u16_le(self.data, 16).unwrap_or(0)
    }

    /// Returns the hitstun frames.
    pub fn hitstun(&self) -> u8 {
        read_u8(self.data, 18).unwrap_or(0)
    }

    /// Returns the blockstun frames.
    pub fn blockstun(&self) -> u8 {
        read_u8(self.data, 19).unwrap_or(0)
    }

    /// Returns the hitstop frames.
    pub fn hitstop(&self) -> u8 {
        read_u8(self.data, 20).unwrap_or(0)
    }

    /// Returns the byte offset within the HIT_WINDOWS section.
    pub fn hit_windows_off(&self) -> u32 {
        read_u32_le(self.data, 22).unwrap_or(0)
    }

    /// Returns the hit windows count.
    pub fn hit_windows_len(&self) -> u16 {
        read_u16_le(self.data, 26).unwrap_or(0)
    }

    /// Returns the byte offset within the HURT_WINDOWS section.
    ///
    /// Note: stored as u16 for compact layout.
    pub fn hurt_windows_off(&self) -> u16 {
        read_u16_le(self.data, 28).unwrap_or(0)
    }

    /// Returns the hurt windows count.
    pub fn hurt_windows_len(&self) -> u16 {
        read_u16_le(self.data, 30).unwrap_or(0)
    }

    /// Returns the byte offset within the PUSH_WINDOWS section.
    pub fn push_windows_off(&self) -> u16 {
        read_u16_le(self.data, 32).unwrap_or(0)
    }

    /// Returns the push windows count.
    pub fn push_windows_len(&self) -> u16 {
        read_u16_le(self.data, 34).unwrap_or(0)
    }
}

// =============================================================================
// Optional Sections Typed Views
// =============================================================================

#[inline]
fn read_range(data: &[u8], base: usize) -> Option<(u32, u16)> {
    let off = read_u32_le(data, base)?;
    let len = read_u16_le(data, base + 4)?;
    Some((off, len))
}

/// Zero-copy view over resource definitions.
#[derive(Clone, Copy)]
pub struct ResourceDefsView<'a> {
    data: &'a [u8],
}

impl<'a> ResourceDefsView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / RESOURCE_DEF_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<ResourceDefView<'a>> {
        let base = index.checked_mul(RESOURCE_DEF_SIZE)?;
        let end = base.checked_add(RESOURCE_DEF_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(ResourceDefView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct ResourceDefView<'a> {
    data: &'a [u8],
}

impl<'a> ResourceDefView<'a> {
    pub fn name_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn name_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn start(&self) -> u16 {
        read_u16_le(self.data, 8).unwrap_or(0)
    }

    pub fn max(&self) -> u16 {
        read_u16_le(self.data, 10).unwrap_or(0)
    }
}

/// Zero-copy view over per-state extras (parallel to STATES).
#[derive(Clone, Copy)]
pub struct StateExtrasView<'a> {
    data: &'a [u8],
}

impl<'a> StateExtrasView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / STATE_EXTRAS_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<StateExtrasRecordView<'a>> {
        let base = index.checked_mul(STATE_EXTRAS_SIZE)?;
        let end = base.checked_add(STATE_EXTRAS_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(StateExtrasRecordView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct StateExtrasRecordView<'a> {
    data: &'a [u8],
}

impl<'a> StateExtrasRecordView<'a> {
    pub fn on_use_emits(&self) -> (u32, u16) {
        read_range(self.data, 0).unwrap_or((0, 0))
    }

    pub fn on_hit_emits(&self) -> (u32, u16) {
        read_range(self.data, 8).unwrap_or((0, 0))
    }

    pub fn on_block_emits(&self) -> (u32, u16) {
        read_range(self.data, 16).unwrap_or((0, 0))
    }

    pub fn notifies(&self) -> (u32, u16) {
        read_range(self.data, 24).unwrap_or((0, 0))
    }

    pub fn resource_costs(&self) -> (u32, u16) {
        read_range(self.data, 32).unwrap_or((0, 0))
    }

    pub fn resource_preconditions(&self) -> (u32, u16) {
        read_range(self.data, 40).unwrap_or((0, 0))
    }

    pub fn resource_deltas(&self) -> (u32, u16) {
        read_range(self.data, 48).unwrap_or((0, 0))
    }

    /// Get the input notation string reference (offset, length).
    pub fn input(&self) -> (u32, u16) {
        read_range(self.data, 56).unwrap_or((0, 0))
    }

    /// Get the cancel routes offset and length into CANCELS_U16.
    ///
    /// Returns (byte_offset, count) where count is the number of u16 target IDs.
    pub fn cancels(&self) -> (u32, u16) {
        read_range(self.data, 64).unwrap_or((0, 0))
    }
}

/// Zero-copy view over event emits.
#[derive(Clone, Copy)]
pub struct EventEmitsView<'a> {
    data: &'a [u8],
}

impl<'a> EventEmitsView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / EVENT_EMIT_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<EventEmitView<'a>> {
        let base = index.checked_mul(EVENT_EMIT_SIZE)?;
        let end = base.checked_add(EVENT_EMIT_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(EventEmitView {
            data: &self.data[base..end],
        })
    }

    /// Get the event emit at `offset_bytes + index * EVENT_EMIT_SIZE`.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<EventEmitView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(EVENT_EMIT_SIZE)?)?;
        let end = base.checked_add(EVENT_EMIT_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(EventEmitView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct EventEmitView<'a> {
    data: &'a [u8],
}

impl<'a> EventEmitView<'a> {
    pub fn id_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn id_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn args(&self) -> (u32, u16) {
        read_range(self.data, 8).unwrap_or((0, 0))
    }
}

/// Zero-copy view over event args.
#[derive(Clone, Copy)]
pub struct EventArgsView<'a> {
    data: &'a [u8],
}

impl<'a> EventArgsView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / EVENT_ARG_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<EventArgView<'a>> {
        let base = index.checked_mul(EVENT_ARG_SIZE)?;
        let end = base.checked_add(EVENT_ARG_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(EventArgView {
            data: &self.data[base..end],
        })
    }

    /// Get the event arg at `offset_bytes + index * EVENT_ARG_SIZE`.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<EventArgView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(EVENT_ARG_SIZE)?)?;
        let end = base.checked_add(EVENT_ARG_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(EventArgView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct EventArgView<'a> {
    data: &'a [u8],
}

impl<'a> EventArgView<'a> {
    pub fn key_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn key_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn tag(&self) -> u8 {
        read_u8(self.data, 8).unwrap_or(0)
    }

    pub fn value_bool(&self) -> Option<bool> {
        if self.tag() != EVENT_ARG_TAG_BOOL {
            return None;
        }
        Some(read_u64_le(self.data, 12)? != 0)
    }

    pub fn value_i64(&self) -> Option<i64> {
        if self.tag() != EVENT_ARG_TAG_I64 {
            return None;
        }
        Some(read_i64_le(self.data, 12)?)
    }

    pub fn value_f32(&self) -> Option<f32> {
        if self.tag() != EVENT_ARG_TAG_F32 {
            return None;
        }
        read_f32_le(self.data, 12)
    }

    pub fn value_string(&self) -> Option<(u32, u16)> {
        if self.tag() != EVENT_ARG_TAG_STRING {
            return None;
        }
        let off = read_u32_le(self.data, 12)?;
        let len = read_u16_le(self.data, 16)?;
        Some((off, len))
    }
}

/// Zero-copy view over move notify records.
#[derive(Clone, Copy)]
pub struct MoveNotifiesView<'a> {
    data: &'a [u8],
}

impl<'a> MoveNotifiesView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / MOVE_NOTIFY_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<MoveNotifyView<'a>> {
        let base = index.checked_mul(MOVE_NOTIFY_SIZE)?;
        let end = base.checked_add(MOVE_NOTIFY_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveNotifyView {
            data: &self.data[base..end],
        })
    }

    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<MoveNotifyView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(MOVE_NOTIFY_SIZE)?)?;
        let end = base.checked_add(MOVE_NOTIFY_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveNotifyView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct MoveNotifyView<'a> {
    data: &'a [u8],
}

impl<'a> MoveNotifyView<'a> {
    pub fn frame(&self) -> u16 {
        read_u16_le(self.data, 0).unwrap_or(0)
    }

    pub fn emits(&self) -> (u32, u16) {
        // frame(u16) + pad(u16) => emits at offset 4
        read_range(self.data, 4).unwrap_or((0, 0))
    }
}

/// Zero-copy view over move resource cost records.
#[derive(Clone, Copy)]
pub struct MoveResourceCostsView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourceCostsView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / MOVE_RESOURCE_COST_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<MoveResourceCostView<'a>> {
        let base = index.checked_mul(MOVE_RESOURCE_COST_SIZE)?;
        let end = base.checked_add(MOVE_RESOURCE_COST_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourceCostView {
            data: &self.data[base..end],
        })
    }

    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<MoveResourceCostView<'a>> {
        let base =
            (offset_bytes as usize).checked_add(index.checked_mul(MOVE_RESOURCE_COST_SIZE)?)?;
        let end = base.checked_add(MOVE_RESOURCE_COST_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourceCostView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct MoveResourceCostView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourceCostView<'a> {
    pub fn name_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn name_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn amount(&self) -> u16 {
        read_u16_le(self.data, 8).unwrap_or(0)
    }
}

/// Zero-copy view over move resource precondition records.
#[derive(Clone, Copy)]
pub struct MoveResourcePreconditionsView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourcePreconditionsView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / MOVE_RESOURCE_PRECONDITION_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<MoveResourcePreconditionView<'a>> {
        let base = index.checked_mul(MOVE_RESOURCE_PRECONDITION_SIZE)?;
        let end = base.checked_add(MOVE_RESOURCE_PRECONDITION_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourcePreconditionView {
            data: &self.data[base..end],
        })
    }

    pub fn get_at(
        &self,
        offset_bytes: u32,
        index: usize,
    ) -> Option<MoveResourcePreconditionView<'a>> {
        let base = (offset_bytes as usize)
            .checked_add(index.checked_mul(MOVE_RESOURCE_PRECONDITION_SIZE)?)?;
        let end = base.checked_add(MOVE_RESOURCE_PRECONDITION_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourcePreconditionView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct MoveResourcePreconditionView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourcePreconditionView<'a> {
    pub fn name_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn name_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn min_raw(&self) -> u16 {
        read_u16_le(self.data, 8).unwrap_or(OPT_U16_NONE)
    }

    pub fn max_raw(&self) -> u16 {
        read_u16_le(self.data, 10).unwrap_or(OPT_U16_NONE)
    }

    pub fn min(&self) -> Option<u16> {
        let v = self.min_raw();
        if v == OPT_U16_NONE {
            None
        } else {
            Some(v)
        }
    }

    pub fn max(&self) -> Option<u16> {
        let v = self.max_raw();
        if v == OPT_U16_NONE {
            None
        } else {
            Some(v)
        }
    }
}

/// Zero-copy view over move resource delta records.
#[derive(Clone, Copy)]
pub struct MoveResourceDeltasView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourceDeltasView<'a> {
    pub fn len(&self) -> usize {
        self.data.len() / MOVE_RESOURCE_DELTA_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<MoveResourceDeltaView<'a>> {
        let base = index.checked_mul(MOVE_RESOURCE_DELTA_SIZE)?;
        let end = base.checked_add(MOVE_RESOURCE_DELTA_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourceDeltaView {
            data: &self.data[base..end],
        })
    }

    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<MoveResourceDeltaView<'a>> {
        let base =
            (offset_bytes as usize).checked_add(index.checked_mul(MOVE_RESOURCE_DELTA_SIZE)?)?;
        let end = base.checked_add(MOVE_RESOURCE_DELTA_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(MoveResourceDeltaView {
            data: &self.data[base..end],
        })
    }
}

#[derive(Clone, Copy)]
pub struct MoveResourceDeltaView<'a> {
    data: &'a [u8],
}

impl<'a> MoveResourceDeltaView<'a> {
    pub fn name_off(&self) -> u32 {
        read_u32_le(self.data, 0).unwrap_or(0)
    }

    pub fn name_len(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    pub fn delta(&self) -> i32 {
        read_i32_le(self.data, 8).unwrap_or(0)
    }

    pub fn trigger(&self) -> u8 {
        read_u8(self.data, 12).unwrap_or(RESOURCE_DELTA_TRIGGER_ON_USE)
    }
}

// =============================================================================
// Hit Windows Views
// =============================================================================

/// Zero-copy view over hit windows section.
///
/// Each entry is a HitWindow24 (24 bytes).
#[derive(Clone, Copy)]
pub struct HitWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> HitWindowsView<'a> {
    /// Returns the total number of hit windows.
    pub fn len(&self) -> usize {
        self.data.len() / HIT_WINDOW_SIZE
    }

    /// Returns true if there are no hit windows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a hit window by global index.
    pub fn get(&self, index: usize) -> Option<HitWindowView<'a>> {
        let off = index.checked_mul(HIT_WINDOW_SIZE)?;
        let end = off.checked_add(HIT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HitWindowView {
            data: &self.data[off..end],
        })
    }

    /// Get a hit window at a byte offset + index.
    ///
    /// This is used to access a move's hit windows when you have the
    /// byte offset and want to iterate by index within that range.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<HitWindowView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(HIT_WINDOW_SIZE)?)?;
        let end = base.checked_add(HIT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HitWindowView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single HitWindow24 record (24 bytes minimum).
///
/// Layout:
/// - 0: start_f (u8)
/// - 1: end_f (u8)
/// - 2: guard (u8)
/// - 3: _reserved (u8)
/// - 4-5: dmg (u16)
/// - 6-7: chip (u16)
/// - 8: hitstun (u8)
/// - 9: blockstun (u8)
/// - 10: hitstop (u8)
/// - 11: _reserved (u8)
/// - 12-15: shapes_off (u32)
/// - 16-17: shapes_len (u16)
/// - 18-21: cancels_off (u32)
/// - 22-23: cancels_len (u16)
/// Optional extended fields (backwards-compatible):
/// - 24-25: hit_pushback (i16, Q12.4 fixed-point)
/// - 26-27: block_pushback (i16, Q12.4 fixed-point)
#[derive(Clone, Copy)]
pub struct HitWindowView<'a> {
    data: &'a [u8],
}

impl<'a> HitWindowView<'a> {
    /// Start frame of this hit window.
    pub fn start_frame(&self) -> u8 {
        read_u8(self.data, 0).unwrap_or(0)
    }

    /// End frame of this hit window.
    pub fn end_frame(&self) -> u8 {
        read_u8(self.data, 1).unwrap_or(0)
    }

    /// Guard type for this hit window.
    pub fn guard(&self) -> u8 {
        read_u8(self.data, 2).unwrap_or(0)
    }

    /// Damage value for this hit window.
    pub fn damage(&self) -> u16 {
        read_u16_le(self.data, 4).unwrap_or(0)
    }

    /// Chip damage for this hit window (0 = none).
    pub fn chip_damage(&self) -> u16 {
        read_u16_le(self.data, 6).unwrap_or(0)
    }

    /// Hitstun frames for this hit window.
    pub fn hitstun(&self) -> u8 {
        read_u8(self.data, 8).unwrap_or(0)
    }

    /// Blockstun frames for this hit window.
    pub fn blockstun(&self) -> u8 {
        read_u8(self.data, 9).unwrap_or(0)
    }

    /// Hitstop frames for this hit window.
    pub fn hitstop(&self) -> u8 {
        read_u8(self.data, 10).unwrap_or(0)
    }

    /// Byte offset into SHAPES section.
    pub fn shapes_off(&self) -> u32 {
        read_u32_le(self.data, 12).unwrap_or(0)
    }

    /// Number of shapes in this hit window.
    pub fn shapes_len(&self) -> u16 {
        read_u16_le(self.data, 16).unwrap_or(0)
    }

    /// Byte offset into CANCELS_U16 section.
    pub fn cancels_off(&self) -> u32 {
        read_u32_le(self.data, 18).unwrap_or(0)
    }

    /// Number of cancel targets for this hit window.
    pub fn cancels_len(&self) -> u16 {
        read_u16_le(self.data, 22).unwrap_or(0)
    }

    /// Hit pushback (Q12.4 fixed-point). Returns 0 if not present.
    pub fn hit_pushback_raw(&self) -> i16 {
        if self.data.len() >= 26 {
            read_u16_le(self.data, 24).unwrap_or(0) as i16
        } else {
            0
        }
    }

    /// Block pushback (Q12.4 fixed-point). Returns 0 if not present.
    pub fn block_pushback_raw(&self) -> i16 {
        if self.data.len() >= 28 {
            read_u16_le(self.data, 26).unwrap_or(0) as i16
        } else {
            0
        }
    }

    /// Hit pushback in pixels.
    pub fn hit_pushback_px(&self) -> i32 {
        (self.hit_pushback_raw() as i32) >> 4
    }

    /// Block pushback in pixels.
    pub fn block_pushback_px(&self) -> i32 {
        (self.block_pushback_raw() as i32) >> 4
    }
}

// =============================================================================
// Hurt Windows Views
// =============================================================================

/// Zero-copy view over hurt windows section.
///
/// Each entry is a HurtWindow12 (12 bytes).
#[derive(Clone, Copy)]
pub struct HurtWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> HurtWindowsView<'a> {
    /// Returns the total number of hurt windows.
    pub fn len(&self) -> usize {
        self.data.len() / HURT_WINDOW_SIZE
    }

    /// Returns true if there are no hurt windows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a hurt window by global index.
    pub fn get(&self, index: usize) -> Option<HurtWindowView<'a>> {
        let off = index.checked_mul(HURT_WINDOW_SIZE)?;
        let end = off.checked_add(HURT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HurtWindowView {
            data: &self.data[off..end],
        })
    }

    /// Get a hurt window at a byte offset + index.
    pub fn get_at(&self, offset_bytes: u16, index: usize) -> Option<HurtWindowView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(HURT_WINDOW_SIZE)?)?;
        let end = base.checked_add(HURT_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HurtWindowView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single HurtWindow12 record (12 bytes).
///
/// Layout:
/// - 0: start_f (u8)
/// - 1: end_f (u8)
/// - 2-3: hurt_flags (u16)
/// - 4-7: shapes_off (u32)
/// - 8-9: shapes_len (u16)
/// - 10-11: _pad (u16)
#[derive(Clone, Copy)]
pub struct HurtWindowView<'a> {
    data: &'a [u8],
}

impl<'a> HurtWindowView<'a> {
    /// Start frame of this hurt window.
    pub fn start_frame(&self) -> u8 {
        read_u8(self.data, 0).unwrap_or(0)
    }

    /// End frame of this hurt window.
    pub fn end_frame(&self) -> u8 {
        read_u8(self.data, 1).unwrap_or(0)
    }

    /// Hurtbox flags (invincibility, armor, etc.).
    pub fn hurt_flags(&self) -> u16 {
        read_u16_le(self.data, 2).unwrap_or(0)
    }

    /// Byte offset into SHAPES section.
    pub fn shapes_off(&self) -> u32 {
        read_u32_le(self.data, 4).unwrap_or(0)
    }

    /// Number of shapes in this hurt window.
    pub fn shapes_len(&self) -> u16 {
        read_u16_le(self.data, 8).unwrap_or(0)
    }
}

// =============================================================================
// Push Windows Views
// =============================================================================

/// Type alias for push window view - same binary layout as hurt windows.
///
/// Push windows define body collision boxes (pushboxes) that prevent
/// characters from overlapping. They use the same 12-byte format:
/// - 0: start_f (u8)
/// - 1: end_f (u8)
/// - 2-3: flags (u16)
/// - 4-7: shapes_off (u32)
/// - 8-9: shapes_len (u16)
/// - 10-11: _pad (u16)
pub type PushWindowView<'a> = HurtWindowView<'a>;

/// Zero-copy view over push windows section.
///
/// Each entry is 12 bytes (same layout as HurtWindow12).
#[derive(Clone, Copy)]
pub struct PushWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> PushWindowsView<'a> {
    /// Returns the total number of push windows.
    pub fn len(&self) -> usize {
        self.data.len() / PUSH_WINDOW_SIZE
    }

    /// Returns true if there are no push windows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a push window by global index.
    pub fn get(&self, index: usize) -> Option<PushWindowView<'a>> {
        let off = index.checked_mul(PUSH_WINDOW_SIZE)?;
        let end = off.checked_add(PUSH_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HurtWindowView {
            data: &self.data[off..end],
        })
    }

    /// Get a push window at a byte offset + index.
    ///
    /// This is used to access push windows referenced by a state.
    pub fn get_at(&self, offset_bytes: u16, index: usize) -> Option<PushWindowView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(PUSH_WINDOW_SIZE)?)?;
        let end = base.checked_add(PUSH_WINDOW_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(HurtWindowView {
            data: &self.data[base..end],
        })
    }
}

// =============================================================================
// Shape Views
// =============================================================================

/// Zero-copy view over shapes section.
///
/// Each entry is a Shape12 (12 bytes).
#[derive(Clone, Copy)]
pub struct ShapesView<'a> {
    data: &'a [u8],
}

impl<'a> ShapesView<'a> {
    /// Returns the total number of shapes.
    pub fn len(&self) -> usize {
        self.data.len() / SHAPE_SIZE
    }

    /// Returns true if there are no shapes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a shape by global index.
    pub fn get(&self, index: usize) -> Option<ShapeView<'a>> {
        let off = index.checked_mul(SHAPE_SIZE)?;
        let end = off.checked_add(SHAPE_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(ShapeView {
            data: &self.data[off..end],
        })
    }

    /// Get a shape at a byte offset + index.
    ///
    /// This is used to access shapes referenced by a hit/hurt window.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<ShapeView<'a>> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(SHAPE_SIZE)?)?;
        let end = base.checked_add(SHAPE_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(ShapeView {
            data: &self.data[base..end],
        })
    }
}

/// Zero-copy view over a single Shape12 record (12 bytes).
///
/// Uses Q12.4 fixed-point coordinates (1/16 pixel precision).
///
/// Layout:
/// - 0: kind (u8) - 0=aabb, 1=rect, 2=circle, 3=capsule
/// - 1: flags (u8) - reserved
/// - 2-3: a (i16 Q12.4) - x for aabb/rect/circle, x1 for capsule
/// - 4-5: b (i16 Q12.4) - y for aabb/rect/circle, y1 for capsule
/// - 6-7: c (i16 Q12.4) - width for aabb/rect, radius for circle, x2 for capsule
/// - 8-9: d (i16 Q12.4) - height for aabb/rect, unused for circle, y2 for capsule
/// - 10-11: e (i16 Q8.8) - angle for rect, radius for capsule
#[derive(Clone, Copy)]
pub struct ShapeView<'a> {
    data: &'a [u8],
}

impl<'a> ShapeView<'a> {
    /// Shape type: 0=aabb, 1=rect, 2=circle, 3=capsule.
    pub fn kind(&self) -> u8 {
        read_u8(self.data, 0).unwrap_or(0)
    }

    /// Shape flags (reserved).
    pub fn flags(&self) -> u8 {
        read_u8(self.data, 1).unwrap_or(0)
    }

    /// Raw field a (Q12.4 fixed-point).
    /// For AABB/rect/circle: x coordinate.
    /// For capsule: x1 coordinate.
    pub fn a_raw(&self) -> i16 {
        read_u16_le(self.data, 2).unwrap_or(0) as i16
    }

    /// Raw field b (Q12.4 fixed-point).
    /// For AABB/rect/circle: y coordinate.
    /// For capsule: y1 coordinate.
    pub fn b_raw(&self) -> i16 {
        read_u16_le(self.data, 4).unwrap_or(0) as i16
    }

    /// Raw field c (Q12.4 fixed-point).
    /// For AABB/rect: width.
    /// For circle: radius.
    /// For capsule: x2 coordinate.
    pub fn c_raw(&self) -> i16 {
        read_u16_le(self.data, 6).unwrap_or(0) as i16
    }

    /// Raw field d (Q12.4 fixed-point).
    /// For AABB/rect: height.
    /// For circle: unused.
    /// For capsule: y2 coordinate.
    pub fn d_raw(&self) -> i16 {
        read_u16_le(self.data, 8).unwrap_or(0) as i16
    }

    /// Raw field e (Q8.8 fixed-point).
    /// For rect: rotation angle.
    /// For capsule: radius.
    pub fn e_raw(&self) -> i16 {
        read_u16_le(self.data, 10).unwrap_or(0) as i16
    }

    /// Convert Q12.4 fixed-point to integer pixels (rounding down).
    #[inline]
    fn q12_4_to_px(v: i16) -> i32 {
        (v as i32) >> 4
    }

    /// Get AABB x coordinate in pixels (valid for kind=0,1,2).
    pub fn x_px(&self) -> i32 {
        Self::q12_4_to_px(self.a_raw())
    }

    /// Get AABB y coordinate in pixels (valid for kind=0,1,2).
    pub fn y_px(&self) -> i32 {
        Self::q12_4_to_px(self.b_raw())
    }

    /// Get width in pixels (valid for kind=0,1).
    pub fn width_px(&self) -> u32 {
        Self::q12_4_to_px(self.c_raw()).max(0) as u32
    }

    /// Get height in pixels (valid for kind=0,1).
    pub fn height_px(&self) -> u32 {
        Self::q12_4_to_px(self.d_raw()).max(0) as u32
    }

    /// Get radius in pixels (valid for kind=2).
    pub fn radius_px(&self) -> u32 {
        Self::q12_4_to_px(self.c_raw()).max(0) as u32
    }

    /// Check if this is an AABB shape.
    pub fn is_aabb(&self) -> bool {
        self.kind() == SHAPE_KIND_AABB
    }
}

// =============================================================================
// Cancel Views
// =============================================================================

/// Zero-copy view over cancel targets (CANCELS_U16 section).
///
/// Each entry is a u16 move ID representing a cancel target.
#[derive(Clone, Copy)]
pub struct CancelsView<'a> {
    data: &'a [u8],
}

impl<'a> CancelsView<'a> {
    /// Returns the total number of cancel target entries.
    pub fn len(&self) -> usize {
        self.data.len() / 2
    }

    /// Returns true if there are no cancel targets.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a cancel target by global index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<u16> {
        let off = index.checked_mul(2)?;
        if off + 2 > self.data.len() {
            return None;
        }
        read_u16_le(self.data, off)
    }

    /// Get a cancel target at a byte offset + index.
    ///
    /// This is used to access a move's chain targets when you have the
    /// offset (in bytes) and want to iterate by index within that range.
    ///
    /// Returns `None` if the computed position is out of bounds.
    pub fn get_at(&self, offset_bytes: u32, index: usize) -> Option<u16> {
        let base = (offset_bytes as usize).checked_add(index.checked_mul(2)?)?;
        if base + 2 > self.data.len() {
            return None;
        }
        read_u16_le(self.data, base)
    }

    /// Returns an iterator over all cancel target move IDs.
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
}

// =============================================================================
// State Tag Views
// =============================================================================

/// Zero-copy view over STATE_TAG_RANGES section.
///
/// Each entry is a StateTagRange8 (8 bytes): offset(4) + count(2) + padding(2)
/// This section is parallel to MOVES - one entry per state/move.
#[derive(Clone, Copy)]
pub struct StateTagRangesView<'a> {
    data: &'a [u8],
}

impl<'a> StateTagRangesView<'a> {
    /// Get the tag range (offset, count) for a state by index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let offset = index * STATE_TAG_RANGE_SIZE;
        if offset + STATE_TAG_RANGE_SIZE > self.data.len() {
            return None;
        }
        let slice = &self.data[offset..offset + STATE_TAG_RANGE_SIZE];
        let off = read_u32_le(slice, 0)?;
        let count = read_u16_le(slice, 4)?;
        Some((off, count))
    }

    /// Returns the number of entries (one per state/move).
    pub fn len(&self) -> usize {
        self.data.len() / STATE_TAG_RANGE_SIZE
    }

    /// Returns true if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// =============================================================================
// Cancel Tag Rules View
// =============================================================================

/// View into a single cancel tag rule.
///
/// Binary layout (24 bytes):
/// - from_tag StrRef: offset(4) + len(2) + pad(2) = 8 bytes (0xFFFFFFFF offset = "any")
/// - to_tag StrRef: offset(4) + len(2) + pad(2) = 8 bytes
/// - condition: u8 (0=always, 1=hit, 2=block, 3=whiff)
/// - min_frame: u8
/// - max_frame: u8
/// - flags: u8
/// - padding: 4 bytes
pub struct CancelTagRuleView<'a> {
    data: &'a [u8],
    pack: &'a PackView<'a>,
}

impl<'a> CancelTagRuleView<'a> {
    /// Get the source tag. Returns None if "any" (sentinel 0xFFFFFFFF).
    pub fn from_tag(&self) -> Option<&'a str> {
        let off = read_u32_le(self.data, 0)?;
        let len = read_u16_le(self.data, 4)?;
        if off == 0xFFFFFFFF {
            return None;
        } // "any"
        self.pack.string(off, len)
    }

    /// Get the target tag. Returns None if "any".
    pub fn to_tag(&self) -> Option<&'a str> {
        let off = read_u32_le(self.data, 8)?;
        let len = read_u16_le(self.data, 12)?;
        if off == 0xFFFFFFFF {
            return None;
        }
        self.pack.string(off, len)
    }

    /// Get the condition (0=always, 1=hit, 2=block, 3=whiff).
    pub fn condition(&self) -> u8 {
        read_u8(self.data, 16).unwrap_or(0)
    }

    /// Get the minimum frame for this cancel.
    pub fn min_frame(&self) -> u8 {
        read_u8(self.data, 17).unwrap_or(0)
    }

    /// Get the maximum frame for this cancel.
    pub fn max_frame(&self) -> u8 {
        read_u8(self.data, 18).unwrap_or(0)
    }

    /// Get the flags byte.
    pub fn flags(&self) -> u8 {
        read_u8(self.data, 19).unwrap_or(0)
    }
}

/// View into cancel tag rules section.
#[derive(Clone, Copy)]
pub struct CancelTagRulesView<'a> {
    data: &'a [u8],
    pack: &'a PackView<'a>,
}

impl<'a> CancelTagRulesView<'a> {
    /// Get a cancel tag rule by index.
    pub fn get(&self, index: usize) -> Option<CancelTagRuleView<'a>> {
        let offset = index * CANCEL_TAG_RULE_SIZE;
        if offset + CANCEL_TAG_RULE_SIZE > self.data.len() {
            return None;
        }
        Some(CancelTagRuleView {
            data: &self.data[offset..offset + CANCEL_TAG_RULE_SIZE],
            pack: self.pack,
        })
    }

    /// Returns the number of cancel tag rules.
    pub fn len(&self) -> usize {
        self.data.len() / CANCEL_TAG_RULE_SIZE
    }

    /// Returns true if there are no cancel tag rules.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over all cancel tag rules.
    pub fn iter(&self) -> impl Iterator<Item = CancelTagRuleView<'a>> + '_ {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
}

// =============================================================================
// Character Property Views
// =============================================================================

/// View into a single character property record.
///
/// Binary layout (12 bytes):
/// - name_off: u32 (offset into string table)
/// - name_len: u16 (length in bytes)
/// - value_type: u8 (0=i32/Q24.8, 1=bool, 2=string ref)
/// - _pad: u8
/// - value: u32 (raw value, interpretation depends on type)
#[derive(Clone, Copy)]
pub struct CharacterPropView<'a> {
    data: &'a [u8],
}

impl<'a> CharacterPropView<'a> {
    /// Get the property name string reference (offset, length).
    pub fn name(&self) -> (u32, u16) {
        let off = read_u32_le(self.data, 0).unwrap_or(0);
        let len = read_u16_le(self.data, 4).unwrap_or(0);
        (off, len)
    }

    /// Get the value type tag.
    /// 0 = i32 (Q24.8 fixed-point), 1 = bool, 2 = string reference
    pub fn value_type(&self) -> u8 {
        read_u8(self.data, 6).unwrap_or(0)
    }

    /// Get the raw value as u32.
    pub fn value_raw(&self) -> u32 {
        read_u32_le(self.data, 8).unwrap_or(0)
    }

    /// Interpret the value as Q24.8 fixed-point (signed).
    pub fn as_q24_8(&self) -> i32 {
        read_i32_le(self.data, 8).unwrap_or(0)
    }

    /// Interpret the value as boolean.
    pub fn as_bool(&self) -> bool {
        read_u8(self.data, 8).unwrap_or(0) != 0
    }

    /// Interpret the value as a string reference (offset, length).
    /// The offset is stored in the lower 16 bits, length in the upper 16 bits.
    pub fn as_str_ref(&self) -> (u16, u16) {
        let off = read_u16_le(self.data, 8).unwrap_or(0);
        let len = read_u16_le(self.data, 10).unwrap_or(0);
        (off, len)
    }
}

/// Zero-copy view over the character properties section.
///
/// Each entry is a CharacterProp12 (12 bytes).
#[derive(Clone, Copy)]
pub struct CharacterPropsView<'a> {
    data: &'a [u8],
}

impl<'a> CharacterPropsView<'a> {
    /// Returns the number of character properties.
    pub fn len(&self) -> usize {
        self.data.len() / CHARACTER_PROP_SIZE
    }

    /// Returns true if there are no character properties.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get a character property by index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<CharacterPropView<'a>> {
        let off = index.checked_mul(CHARACTER_PROP_SIZE)?;
        let end = off.checked_add(CHARACTER_PROP_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(CharacterPropView {
            data: &self.data[off..end],
        })
    }

    /// Returns an iterator over all character properties.
    pub fn iter(&self) -> impl Iterator<Item = CharacterPropView<'a>> + '_ {
        (0..self.len()).filter_map(|i| self.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a valid FSPK header.
    fn build_header(flags: u32, total_len: u32, section_count: u32) -> [u8; 16] {
        let mut header = [0u8; 16];
        header[0..4].copy_from_slice(&MAGIC);
        header[4..8].copy_from_slice(&flags.to_le_bytes());
        header[8..12].copy_from_slice(&total_len.to_le_bytes());
        header[12..16].copy_from_slice(&section_count.to_le_bytes());
        header
    }

    /// Helper to build a section header.
    fn build_section_header(kind: u32, offset: u32, len: u32, align: u32) -> [u8; 16] {
        let mut section = [0u8; 16];
        section[0..4].copy_from_slice(&kind.to_le_bytes());
        section[4..8].copy_from_slice(&offset.to_le_bytes());
        section[8..12].copy_from_slice(&len.to_le_bytes());
        section[12..16].copy_from_slice(&align.to_le_bytes());
        section
    }

    #[test]
    fn parse_too_short() {
        let result = PackView::parse(&[0u8; 15]);
        assert!(matches!(result, Err(Error::TooShort)));
    }

    #[test]
    fn parse_wrong_magic() {
        let mut data = build_header(0, 16, 0);
        data[0..4].copy_from_slice(b"XXXX");

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::InvalidMagic)));
    }

    #[test]
    fn parse_section_table_out_of_bounds() {
        // Header claims 2 sections but data only has room for header
        let header = build_header(0, 16, 2);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_section_table_end_exceeds_total_len() {
        // total_len is smaller than HEADER_SIZE + section_count * SECTION_HEADER_SIZE,
        // but the backing buffer is long enough to contain the section header table.
        let total_len = 32u32;
        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, 2));
        // Two zero-length sections that (incorrectly) appear to fit within total_len.
        data.extend_from_slice(&build_section_header(1, 16, 0, 1));
        data.extend_from_slice(&build_section_header(2, 16, 0, 1));
        // Ensure bytes.len() is large enough that the old check (against bytes.len()) passes.
        data.resize((HEADER_SIZE + 2 * SECTION_HEADER_SIZE) as usize, 0);

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_section_data_out_of_bounds() {
        // Header + 1 section header
        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, 32, 1));
        // Section at offset 32 with len 100 - way beyond total_len
        data.extend_from_slice(&build_section_header(1, 32, 100, 1));

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_section_offset_overflow() {
        // Header + 1 section header
        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, 48, 1));
        // Section with offset + len that would overflow
        data.extend_from_slice(&build_section_header(1, u32::MAX, 1, 1));
        // Add some padding to reach total_len
        data.resize(48, 0);

        let result = PackView::parse(&data);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_total_len_exceeds_data() {
        // Header claims total_len is 1000 but we only have 16 bytes
        let header = build_header(0, 1000, 0);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_too_many_sections() {
        // Claim more than MAX_SECTIONS
        let header = build_header(0, 16, 100);
        let result = PackView::parse(&header);
        assert!(matches!(result, Err(Error::OutOfBounds)));
    }

    #[test]
    fn parse_valid_empty_pack() {
        let header = build_header(0, 16, 0);
        let view = PackView::parse(&header).expect("should parse valid empty pack");
        assert_eq!(view.section_count(), 0);
    }

    #[test]
    fn parse_valid_with_one_section() {
        let mut data = std::vec::Vec::new();
        // Header: total_len = 16 (header) + 16 (section header) + 8 (section data) = 40
        data.extend_from_slice(&build_header(0, 40, 1));
        // Section header: kind=0x1234, offset=32, len=8, align=4
        data.extend_from_slice(&build_section_header(0x1234, 32, 8, 4));
        // Section data
        data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);

        let view = PackView::parse(&data).expect("should parse valid pack");
        assert_eq!(view.section_count(), 1);

        let section = view.get_section(0x1234).expect("should find section");
        assert_eq!(section, &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
    }

    #[test]
    fn parse_valid_with_multiple_sections() {
        let mut data = std::vec::Vec::new();
        // Header: total_len = 16 + 32 (2 section headers) + 12 (section data) = 60
        data.extend_from_slice(&build_header(0, 60, 2));
        // Section 1: kind=1, offset=48, len=4, align=4
        data.extend_from_slice(&build_section_header(1, 48, 4, 4));
        // Section 2: kind=2, offset=52, len=8, align=4
        data.extend_from_slice(&build_section_header(2, 52, 8, 4));
        // Section 1 data
        data.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        // Section 2 data
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]);

        let view = PackView::parse(&data).expect("should parse valid pack");
        assert_eq!(view.section_count(), 2);

        let section1 = view.get_section(1).expect("should find section 1");
        assert_eq!(section1, &[0x11, 0x22, 0x33, 0x44]);

        let section2 = view.get_section(2).expect("should find section 2");
        assert_eq!(section2, &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]);
    }

    #[test]
    fn get_section_not_found() {
        let header = build_header(0, 16, 0);
        let view = PackView::parse(&header).expect("should parse valid empty pack");
        assert!(view.get_section(0x9999).is_none());
    }

    // =========================================================================
    // Typed View Tests
    // =========================================================================

    /// Helper to build a StrRef (8 bytes): off(4) + len(2) + pad(2)
    fn build_strref(off: u32, len: u16) -> [u8; 8] {
        let mut data = [0u8; 8];
        data[0..4].copy_from_slice(&off.to_le_bytes());
        data[4..6].copy_from_slice(&len.to_le_bytes());
        // pad bytes are 0
        data
    }

    /// Helper to build a MoveRecord (36 bytes).
    fn build_move_record(
        move_id: u16,
        mesh_key: u16,
        keyframes_key: u16,
        move_type: u8,
        trigger: u8,
        guard: u8,
        flags: u8,
        startup: u8,
        active: u8,
        recovery: u8,
        total: u16,
        damage: u16,
        hitstun: u8,
        blockstun: u8,
        hitstop: u8,
        hit_windows_off: u32,
        hit_windows_len: u16,
        hurt_windows_off: u16,
        hurt_windows_len: u16,
        push_windows_off: u16,
        push_windows_len: u16,
    ) -> [u8; 36] {
        let mut data = [0u8; 36];
        data[0..2].copy_from_slice(&move_id.to_le_bytes());
        data[2..4].copy_from_slice(&mesh_key.to_le_bytes());
        data[4..6].copy_from_slice(&keyframes_key.to_le_bytes());
        data[6] = move_type;
        data[7] = trigger;
        data[8] = guard;
        data[9] = flags;
        data[10] = startup;
        data[11] = active;
        data[12] = recovery;
        data[13] = 0; // reserved
        data[14..16].copy_from_slice(&total.to_le_bytes());
        data[16..18].copy_from_slice(&damage.to_le_bytes());
        data[18] = hitstun;
        data[19] = blockstun;
        data[20] = hitstop;
        data[21] = 0; // reserved
        data[22..26].copy_from_slice(&hit_windows_off.to_le_bytes());
        data[26..28].copy_from_slice(&hit_windows_len.to_le_bytes());
        data[28..30].copy_from_slice(&hurt_windows_off.to_le_bytes());
        data[30..32].copy_from_slice(&hurt_windows_len.to_le_bytes());
        data[32..34].copy_from_slice(&push_windows_off.to_le_bytes());
        data[34..36].copy_from_slice(&push_windows_len.to_le_bytes());
        data
    }

    /// Build a minimal FSPK pack containing:
    /// - STRING_TABLE: "test_char.5h" (12 bytes)
    /// - MESH_KEYS: one StrRef pointing to "test_char.5h"
    /// - MOVES: one MoveRecord with mesh_key=0
    fn build_minimal_typed_pack() -> std::vec::Vec<u8> {
        let mut data = std::vec::Vec::new();

        // String data: "test_char.5h" (12 bytes)
        let string_data = b"test_char.5h";
        let string_len = string_data.len() as u32; // 12

        // Calculate offsets:
        // Header: 16 bytes
        // 3 section headers: 48 bytes (16 * 3)
        // Section data starts at: 64

        // STRING_TABLE at offset 64, len 12
        // MESH_KEYS at offset 76 (64 + 12), len 8
        // MOVES at offset 84 (76 + 8), len 36

        // offset 64: STRING_TABLE (12 bytes) -> ends at 76
        // offset 76 (aligned to 4): MESH_KEYS (8 bytes) -> ends at 84
        // offset 84 (aligned to 4): MOVES (36 bytes) -> ends at 120

        let string_table_off: u32 = 64;
        let mesh_keys_off: u32 = 76; // 64 + 12 = 76 (already aligned)
        let moves_off: u32 = 84; // 76 + 8 = 84 (already aligned)

        let total_len: u32 = 120; // 84 + 36

        // Build header: 3 sections
        data.extend_from_slice(&build_header(0, total_len, 3));

        // Section headers
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            string_len,
            1,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_MESH_KEYS,
            mesh_keys_off,
            8,
            4,
        ));
        data.extend_from_slice(&build_section_header(SECTION_STATES, moves_off, 36, 4));

        // String table data
        data.extend_from_slice(string_data);

        // No padding needed - 64 + 12 = 76 is already 4-byte aligned

        // Mesh keys: one StrRef pointing to "test_char.5h" at offset 0, len 12
        data.extend_from_slice(&build_strref(0, 12));

        // Move record: mesh_key=0, keyframes_key=0xFFFF (none)
        data.extend_from_slice(&build_move_record(
            0,        // move_id
            0,        // mesh_key (index 0 in MESH_KEYS)
            KEY_NONE, // keyframes_key (none)
            1,        // move_type
            2,        // trigger
            3,        // guard
            4,        // flags
            5,        // startup
            3,        // active
            7,        // recovery
            15,       // total
            100,      // damage
            12,       // hitstun
            8,        // blockstun
            6,        // hitstop
            0,        // hit_windows_off
            0,        // hit_windows_len
            0,        // hurt_windows_off
            0,        // hurt_windows_len
            0,        // push_windows_off
            0,        // push_windows_len
        ));

        assert_eq!(data.len(), total_len as usize);
        data
    }

    #[test]
    fn typed_views_string_from_strref() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        // Get mesh keys view
        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert_eq!(mesh_keys.len(), 1);

        // Get the first mesh key strref
        let (off, len) = mesh_keys.get(0).expect("should get mesh key 0");
        assert_eq!(off, 0);
        assert_eq!(len, 12);

        // Resolve to string
        let key_str = pack.string(off, len).expect("should resolve string");
        assert_eq!(key_str, "test_char.5h");
    }

    #[test]
    fn typed_views_move_record_fields() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let moves = pack.states().expect("should have moves section");
        assert_eq!(moves.len(), 1);

        let mv = moves.get(0).expect("should get move 0");
        assert_eq!(mv.state_id(), 0);
        assert_eq!(mv.mesh_key(), 0);
        assert_eq!(mv.keyframes_key(), KEY_NONE);
        assert_eq!(mv.state_type(), 1);
        assert_eq!(mv.trigger(), 2);
        assert_eq!(mv.guard(), 3);
        assert_eq!(mv.flags(), 4);
        assert_eq!(mv.startup(), 5);
        assert_eq!(mv.active(), 3);
        assert_eq!(mv.recovery(), 7);
        assert_eq!(mv.total(), 15);
        assert_eq!(mv.damage(), 100);
        assert_eq!(mv.hitstun(), 12);
        assert_eq!(mv.blockstun(), 8);
        assert_eq!(mv.hitstop(), 6);
    }

    #[test]
    fn move_view_cancel_flags_decode() {
        // Pack with a single MOVES section.
        let total_len: u32 = (HEADER_SIZE + SECTION_HEADER_SIZE + STATE_RECORD_SIZE) as u32;
        let moves_off: u32 = (HEADER_SIZE + SECTION_HEADER_SIZE) as u32;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, 1));
        data.extend_from_slice(&build_section_header(
            SECTION_STATES,
            moves_off,
            STATE_RECORD_SIZE as u32,
            4,
        ));

        // Flags byte: bits 0..4 set.
        data.extend_from_slice(&build_move_record(
            0,        // move_id
            KEY_NONE, // mesh_key
            KEY_NONE, // keyframes_key
            0,        // move_type
            0,        // trigger
            0,        // guard
            0x1F,     // flags
            0,        // startup
            0,        // active
            0,        // recovery
            0,        // total
            0,        // damage
            0,        // hitstun
            0,        // blockstun
            0,        // hitstop
            0,        // hit_windows_off
            0,        // hit_windows_len
            0,        // hurt_windows_off
            0,        // hurt_windows_len
            0,        // push_windows_off
            0,        // push_windows_len
        ));

        assert_eq!(data.len(), total_len as usize);

        let pack = PackView::parse(&data).expect("should parse pack");
        let mv = pack.states().expect("moves").get(0).expect("move 0");
        let flags = mv.cancel_flags();
        assert!(flags.chain);
        assert!(flags.special);
        assert!(flags.super_cancel);
        assert!(flags.jump);
        assert!(flags.self_gatling);
    }

    #[test]
    fn typed_views_mesh_key_lookup() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let moves = pack.states().expect("should have moves section");
        let mv = moves.get(0).expect("should get move 0");
        let mesh_key_idx = mv.mesh_key();

        // Look up the mesh key string
        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        let (off, len) = mesh_keys
            .get(mesh_key_idx as usize)
            .expect("should get mesh key");
        let mesh_key_str = pack
            .string(off, len)
            .expect("should resolve mesh key string");
        assert_eq!(mesh_key_str, "test_char.5h");
    }

    #[test]
    fn typed_views_out_of_bounds() {
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse typed pack");

        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert!(mesh_keys.get(1).is_none()); // Only 1 key at index 0

        let moves = pack.states().expect("should have moves section");
        assert!(moves.get(1).is_none()); // Only 1 move at index 0

        // Invalid string offset
        assert!(pack.string(1000, 10).is_none());
    }

    #[test]
    fn typed_views_empty_sections() {
        // Build a pack with empty mesh_keys and moves sections
        let mut data = std::vec::Vec::new();

        // String table with some data
        let string_data = b"test";
        let string_table_off: u32 = 64;
        let total_len: u32 = 68; // 64 + 4

        data.extend_from_slice(&build_header(0, total_len, 3));
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            4,
            1,
        ));
        data.extend_from_slice(&build_section_header(SECTION_MESH_KEYS, 68, 0, 4)); // empty
        data.extend_from_slice(&build_section_header(SECTION_STATES, 68, 0, 4)); // empty
        data.extend_from_slice(string_data);

        let pack = PackView::parse(&data).expect("should parse pack with empty sections");

        let mesh_keys = pack.mesh_keys().expect("should have mesh keys section");
        assert_eq!(mesh_keys.len(), 0);
        assert!(mesh_keys.is_empty());
        assert!(mesh_keys.get(0).is_none());

        let moves = pack.states().expect("should have moves section");
        assert_eq!(moves.len(), 0);
        assert!(moves.is_empty());
        assert!(moves.get(0).is_none());
    }

    #[test]
    fn typed_views_keyframes_keys() {
        // Build a pack with keyframes keys
        let mut data = std::vec::Vec::new();

        let string_data = b"anim.idle";
        let string_table_off: u32 = 48; // 16 + 32 (header + 2 section headers)
        let keyframes_keys_off: u32 = 60; // 48 + 9 = 57, round to 60
        let total_len: u32 = 68; // 60 + 8

        data.extend_from_slice(&build_header(0, total_len, 2));
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_table_off,
            9,
            1,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_KEYFRAMES_KEYS,
            keyframes_keys_off,
            8,
            4,
        ));
        data.extend_from_slice(string_data);
        data.extend_from_slice(&[0, 0, 0]); // padding
        data.extend_from_slice(&build_strref(0, 9));

        let pack = PackView::parse(&data).expect("should parse pack with keyframes keys");

        let keyframes_keys = pack
            .keyframes_keys()
            .expect("should have keyframes keys section");
        assert_eq!(keyframes_keys.len(), 1);

        let (off, len) = keyframes_keys.get(0).expect("should get keyframes key 0");
        let key_str = pack.string(off, len).expect("should resolve string");
        assert_eq!(key_str, "anim.idle");
    }

    #[test]
    fn find_state_by_input_notation() {
        fn align_up(v: u32, align: u32) -> u32 {
            if align <= 1 {
                return v;
            }
            let mask = align - 1;
            (v + mask) & !mask
        }

        fn pad_to(data: &mut std::vec::Vec<u8>, off: u32) {
            assert!(data.len() <= off as usize);
            data.resize(off as usize, 0);
        }

        fn build_move_extras_record72(input: (u32, u16)) -> [u8; 72] {
            fn write_range(dst: &mut [u8], base: usize, r: (u32, u16)) {
                dst[base..base + 4].copy_from_slice(&r.0.to_le_bytes());
                dst[base + 4..base + 6].copy_from_slice(&r.1.to_le_bytes());
                dst[base + 6..base + 8].copy_from_slice(&0u16.to_le_bytes());
            }

            let mut data = [0u8; 72];
            // All other extras empty for this test.
            write_range(&mut data, 56, input);
            // cancels at offset 64 left as zeros
            data
        }

        // Provide a string table containing the input notation itself.
        let string_data = b"5L";
        let input_ref = (0u32, 2u16);

        // Section order:
        // STRING_TABLE, MOVES, MOVE_EXTRAS
        let section_count = 3u32;
        let section_headers_bytes = section_count as usize * SECTION_HEADER_SIZE;
        let data_start = (HEADER_SIZE + section_headers_bytes) as u32;

        let string_off = data_start;
        let string_len = string_data.len() as u32;

        let moves_off = align_up(string_off + string_len, 4);
        let moves_len = 36u32;

        let extras_off = align_up(moves_off + moves_len, 4);
        let extras_len = 72u32;

        let total_len = extras_off + extras_len;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));
        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_off,
            string_len,
            1,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_STATES,
            moves_off,
            moves_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_STATE_EXTRAS,
            extras_off,
            extras_len,
            4,
        ));

        // STRING_TABLE
        pad_to(&mut data, string_off);
        data.extend_from_slice(string_data);

        // MOVES (one placeholder move)
        pad_to(&mut data, moves_off);
        data.extend_from_slice(&build_move_record(
            0,        // move_id
            KEY_NONE, // mesh_key
            KEY_NONE, // keyframes_key
            0,        // move_type
            0,        // trigger
            0,        // guard
            0,        // flags
            0,        // startup
            0,        // active
            0,        // recovery
            0,        // total
            0,        // damage
            0,        // hitstun
            0,        // blockstun
            0,        // hitstop
            0,        // hit_windows_off
            0,        // hit_windows_len
            0,        // hurt_windows_off
            0,        // hurt_windows_len
            0,        // push_windows_off
            0,        // push_windows_len
        ));

        // MOVE_EXTRAS (one record)
        pad_to(&mut data, extras_off);
        data.extend_from_slice(&build_move_extras_record72(input_ref));

        assert_eq!(data.len(), total_len as usize);

        let pack = PackView::parse(&data).expect("should parse pack");

        let extras = pack.state_extras().expect("move extras");
        let ex0 = extras.get(0).expect("extras 0");
        let (off, len) = ex0.input();
        assert_eq!(pack.string(off, len), Some("5L"));

        let (idx, mv) = pack
            .find_state_by_input("5L")
            .expect("expected to find move by input");
        assert_eq!(idx, 0);
        assert_eq!(mv.state_id(), 0);
        assert!(pack.find_state_by_input("2M").is_none());
    }

    fn build_move_extras_record(
        on_use_emits: (u32, u16),
        on_hit_emits: (u32, u16),
        on_block_emits: (u32, u16),
        notifies: (u32, u16),
        resource_costs: (u32, u16),
        resource_preconditions: (u32, u16),
        resource_deltas: (u32, u16),
        input: (u32, u16),
        cancels: (u32, u16),
    ) -> [u8; 72] {
        fn write_range(dst: &mut [u8], base: usize, r: (u32, u16)) {
            dst[base..base + 4].copy_from_slice(&r.0.to_le_bytes());
            dst[base + 4..base + 6].copy_from_slice(&r.1.to_le_bytes());
            dst[base + 6..base + 8].copy_from_slice(&0u16.to_le_bytes());
        }

        let mut data = [0u8; 72];
        write_range(&mut data, 0, on_use_emits);
        write_range(&mut data, 8, on_hit_emits);
        write_range(&mut data, 16, on_block_emits);
        write_range(&mut data, 24, notifies);
        write_range(&mut data, 32, resource_costs);
        write_range(&mut data, 40, resource_preconditions);
        write_range(&mut data, 48, resource_deltas);
        write_range(&mut data, 56, input);
        write_range(&mut data, 64, cancels);
        data
    }

    fn build_resource_def(name: (u32, u16), start: u16, max: u16) -> [u8; 12] {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(&name.0.to_le_bytes());
        data[4..6].copy_from_slice(&name.1.to_le_bytes());
        data[8..10].copy_from_slice(&start.to_le_bytes());
        data[10..12].copy_from_slice(&max.to_le_bytes());
        data
    }

    fn build_event_emit(id: (u32, u16), args: (u32, u16)) -> [u8; 16] {
        let mut data = [0u8; 16];
        data[0..4].copy_from_slice(&id.0.to_le_bytes());
        data[4..6].copy_from_slice(&id.1.to_le_bytes());
        data[8..12].copy_from_slice(&args.0.to_le_bytes());
        data[12..14].copy_from_slice(&args.1.to_le_bytes());
        data
    }

    fn build_event_arg_string(key: (u32, u16), value: (u32, u16)) -> [u8; 20] {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(&key.0.to_le_bytes());
        data[4..6].copy_from_slice(&key.1.to_le_bytes());
        data[8] = EVENT_ARG_TAG_STRING;

        // value u64 stores StrRef: off(u32) + len(u16) + pad(u16)
        data[12..16].copy_from_slice(&value.0.to_le_bytes());
        data[16..18].copy_from_slice(&value.1.to_le_bytes());
        data
    }

    fn build_move_notify(frame: u16, emits: (u32, u16)) -> [u8; 12] {
        let mut data = [0u8; 12];
        data[0..2].copy_from_slice(&frame.to_le_bytes());
        data[4..8].copy_from_slice(&emits.0.to_le_bytes());
        data[8..10].copy_from_slice(&emits.1.to_le_bytes());
        data
    }

    fn build_move_resource_cost(name: (u32, u16), amount: u16) -> [u8; 12] {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(&name.0.to_le_bytes());
        data[4..6].copy_from_slice(&name.1.to_le_bytes());
        data[8..10].copy_from_slice(&amount.to_le_bytes());
        data
    }

    fn build_move_resource_precondition(name: (u32, u16), min: u16, max: u16) -> [u8; 12] {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(&name.0.to_le_bytes());
        data[4..6].copy_from_slice(&name.1.to_le_bytes());
        data[8..10].copy_from_slice(&min.to_le_bytes());
        data[10..12].copy_from_slice(&max.to_le_bytes());
        data
    }

    fn build_move_resource_delta(name: (u32, u16), delta: i32, trigger: u8) -> [u8; 16] {
        let mut data = [0u8; 16];
        data[0..4].copy_from_slice(&name.0.to_le_bytes());
        data[4..6].copy_from_slice(&name.1.to_le_bytes());
        data[8..12].copy_from_slice(&delta.to_le_bytes());
        data[12] = trigger;
        data
    }

    #[test]
    fn typed_views_optional_sections_roundtrip_minimal_pack() {
        fn align_up(v: u32, align: u32) -> u32 {
            if align <= 1 {
                return v;
            }
            let mask = align - 1;
            (v + mask) & !mask
        }

        fn pad_to(data: &mut std::vec::Vec<u8>, off: u32) {
            assert!(data.len() <= off as usize);
            data.resize(off as usize, 0);
        }

        // String table layout:
        // 0: "heat" (4)
        // 4: "vfx.hit_sparks" (14)
        // 18: "strength" (8)
        // 26: "light" (5)
        let string_data = b"heatvfx.hit_sparksstrengthlight";
        assert_eq!(string_data.len(), 31);

        let heat = (0u32, 4u16);
        let hit_sparks = (4u32, 14u16);
        let strength = (18u32, 8u16);
        let light = (26u32, 5u16);

        // Section order (10 sections) for this test:
        // STRING_TABLE, MOVES, MOVE_EXTRAS, RESOURCE_DEFS, EVENT_EMITS, EVENT_ARGS,
        // MOVE_NOTIFIES, MOVE_RESOURCE_COSTS, MOVE_RESOURCE_PRECONDITIONS, MOVE_RESOURCE_DELTAS
        let section_count = 10u32;
        let section_headers_bytes = section_count as usize * SECTION_HEADER_SIZE;
        let data_start = (HEADER_SIZE + section_headers_bytes) as u32;

        let string_off = data_start;
        let string_len = string_data.len() as u32;

        let moves_off = align_up(string_off + string_len, 4);
        let moves_len = 36u32;

        let extras_off = align_up(moves_off + moves_len, 4);
        let extras_len = 72u32;

        let res_off = align_up(extras_off + extras_len, 4);
        let res_len = 12u32;

        let emits_off = align_up(res_off + res_len, 4);
        let emits_len = 16u32;

        let args_off = align_up(emits_off + emits_len, 4);
        let args_len = 20u32;

        let notifies_off = align_up(args_off + args_len, 4);
        let notifies_len = 12u32;

        let costs_off = align_up(notifies_off + notifies_len, 4);
        let costs_len = 12u32;

        let pre_off = align_up(costs_off + costs_len, 4);
        let pre_len = 12u32;

        let deltas_off = align_up(pre_off + pre_len, 4);
        let deltas_len = 16u32;

        let total_len = deltas_off + deltas_len;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));

        data.extend_from_slice(&build_section_header(
            SECTION_STRING_TABLE,
            string_off,
            string_len,
            1,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_STATES,
            moves_off,
            moves_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_STATE_EXTRAS,
            extras_off,
            extras_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_RESOURCE_DEFS,
            res_off,
            res_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_EVENT_EMITS,
            emits_off,
            emits_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_EVENT_ARGS,
            args_off,
            args_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_MOVE_NOTIFIES,
            notifies_off,
            notifies_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_MOVE_RESOURCE_COSTS,
            costs_off,
            costs_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_MOVE_RESOURCE_PRECONDITIONS,
            pre_off,
            pre_len,
            4,
        ));
        data.extend_from_slice(&build_section_header(
            SECTION_MOVE_RESOURCE_DELTAS,
            deltas_off,
            deltas_len,
            4,
        ));

        // STRING_TABLE
        pad_to(&mut data, string_off);
        data.extend_from_slice(string_data);

        // MOVES (one placeholder move)
        pad_to(&mut data, moves_off);
        data.extend_from_slice(&build_move_record(
            0,        // move_id
            KEY_NONE, // mesh_key
            KEY_NONE, // keyframes_key
            0,        // move_type
            0,        // trigger
            0,        // guard
            0,        // flags
            0,        // startup
            0,        // active
            0,        // recovery
            0,        // total
            0,        // damage
            0,        // hitstun
            0,        // blockstun
            0,        // hitstop
            0,        // hit_windows_off
            0,        // hit_windows_len
            0,        // hurt_windows_off
            0,        // hurt_windows_len
            0,        // push_windows_off
            0,        // push_windows_len
        ));

        // MOVE_EXTRAS (one record)
        pad_to(&mut data, extras_off);
        data.extend_from_slice(&build_move_extras_record(
            (0, 0), // on_use emits
            (0, 1), // on_hit emits -> EVENT_EMITS[0]
            (0, 0), // on_block emits
            (0, 1), // notifies -> MOVE_NOTIFIES[0]
            (0, 1), // costs -> MOVE_RESOURCE_COSTS[0]
            (0, 1), // preconditions -> MOVE_RESOURCE_PRECONDITIONS[0]
            (0, 1), // deltas -> MOVE_RESOURCE_DELTAS[0]
            (0, 0), // input
            (0, 0), // cancels
        ));

        // RESOURCE_DEFS
        pad_to(&mut data, res_off);
        data.extend_from_slice(&build_resource_def(heat, 0, 10));

        // EVENT_EMITS (one)
        pad_to(&mut data, emits_off);
        data.extend_from_slice(&build_event_emit(hit_sparks, (0, 1)));

        // EVENT_ARGS (one)
        pad_to(&mut data, args_off);
        data.extend_from_slice(&build_event_arg_string(strength, light));

        // MOVE_NOTIFIES (one) -> re-emit the same emit for simplicity
        pad_to(&mut data, notifies_off);
        data.extend_from_slice(&build_move_notify(7, (0, 1)));

        // MOVE_RESOURCE_COSTS
        pad_to(&mut data, costs_off);
        data.extend_from_slice(&build_move_resource_cost(heat, 1));

        // MOVE_RESOURCE_PRECONDITIONS (min=1, max=none)
        pad_to(&mut data, pre_off);
        data.extend_from_slice(&build_move_resource_precondition(heat, 1, OPT_U16_NONE));

        // MOVE_RESOURCE_DELTAS
        pad_to(&mut data, deltas_off);
        data.extend_from_slice(&build_move_resource_delta(
            heat,
            -1,
            RESOURCE_DELTA_TRIGGER_ON_USE,
        ));

        assert_eq!(data.len(), total_len as usize);

        let pack = PackView::parse(&data).expect("should parse pack with optional sections");

        let resources = pack.resource_defs().expect("resource defs view");
        assert_eq!(resources.len(), 1);
        let r0 = resources.get(0).expect("resource 0");
        assert_eq!(pack.string(r0.name_off(), r0.name_len()), Some("heat"));
        assert_eq!(r0.start(), 0);
        assert_eq!(r0.max(), 10);

        let extras = pack.state_extras().expect("move extras view");
        let ex0 = extras.get(0).expect("extras 0");
        let (hit_off, hit_len) = ex0.on_hit_emits();
        assert_eq!(hit_off, 0);
        assert_eq!(hit_len, 1);

        let emits = pack.event_emits().expect("event emits view");
        let emit0 = emits.get_at(hit_off, 0).expect("emit 0");
        assert_eq!(
            pack.string(emit0.id_off(), emit0.id_len()),
            Some("vfx.hit_sparks")
        );

        let args = pack.event_args().expect("event args view");
        let (emit_args_off, emit_args_len) = emit0.args();
        assert_eq!(emit_args_off, 0);
        assert_eq!(emit_args_len, 1);
        let arg0 = args.get_at(emit_args_off, 0).expect("arg 0");
        assert_eq!(
            pack.string(arg0.key_off(), arg0.key_len()),
            Some("strength")
        );
        let (val_off, val_len) = arg0.value_string().expect("string value");
        assert_eq!(pack.string(val_off, val_len), Some("light"));

        let notifies = pack.move_notifies().expect("move notifies view");
        let (notify_off, notify_len) = ex0.notifies();
        assert_eq!(notify_len, 1);
        let n0 = notifies.get_at(notify_off, 0).expect("notify 0");
        assert_eq!(n0.frame(), 7);
        let (n_emit_off, n_emit_len) = n0.emits();
        assert_eq!(n_emit_len, 1);
        let n_emit0 = emits.get_at(n_emit_off, 0).expect("notify emit 0");
        assert_eq!(
            pack.string(n_emit0.id_off(), n_emit0.id_len()),
            Some("vfx.hit_sparks")
        );

        let costs = pack.move_resource_costs().expect("resource costs view");
        let (cost_off, cost_len) = ex0.resource_costs();
        assert_eq!(cost_len, 1);
        let c0 = costs.get_at(cost_off, 0).expect("cost 0");
        assert_eq!(pack.string(c0.name_off(), c0.name_len()), Some("heat"));
        assert_eq!(c0.amount(), 1);

        let pre = pack
            .move_resource_preconditions()
            .expect("resource preconditions view");
        let (pre_off2, pre_len2) = ex0.resource_preconditions();
        assert_eq!(pre_len2, 1);
        let p0 = pre.get_at(pre_off2, 0).expect("pre 0");
        assert_eq!(pack.string(p0.name_off(), p0.name_len()), Some("heat"));
        assert_eq!(p0.min(), Some(1));
        assert_eq!(p0.max(), None);

        let deltas = pack.move_resource_deltas().expect("resource deltas view");
        let (d_off, d_len) = ex0.resource_deltas();
        assert_eq!(d_len, 1);
        let d0 = deltas.get_at(d_off, 0).expect("delta 0");
        assert_eq!(pack.string(d0.name_off(), d0.name_len()), Some("heat"));
        assert_eq!(d0.delta(), -1);
        assert_eq!(d0.trigger(), RESOURCE_DELTA_TRIGGER_ON_USE);
    }

    #[test]
    fn cancels_view_basic() {
        // Build a minimal pack with CANCELS_U16 section containing [1, 3, 5]
        let cancel_data: [u8; 6] = [
            0x01, 0x00, // u16 = 1
            0x03, 0x00, // u16 = 3
            0x05, 0x00, // u16 = 5
        ];

        let section_count = 1u32;
        let data_off = (HEADER_SIZE + SECTION_HEADER_SIZE) as u32;
        let total_len = data_off + cancel_data.len() as u32;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));
        data.extend_from_slice(&build_section_header(
            SECTION_CANCELS_U16,
            data_off,
            cancel_data.len() as u32,
            2,
        ));
        data.extend_from_slice(&cancel_data);

        let pack = PackView::parse(&data).expect("parse pack");
        let cancels = pack.cancels().expect("cancels view");

        assert_eq!(cancels.len(), 3);
        assert!(!cancels.is_empty());
        assert_eq!(cancels.get(0), Some(1));
        assert_eq!(cancels.get(1), Some(3));
        assert_eq!(cancels.get(2), Some(5));
        assert_eq!(cancels.get(3), None); // out of bounds

        // Test get_at: offset 2 bytes (skip first entry), index 0 should give 3
        assert_eq!(cancels.get_at(2, 0), Some(3));
        assert_eq!(cancels.get_at(2, 1), Some(5));
        assert_eq!(cancels.get_at(2, 2), None); // out of bounds

        // Test iterator
        let all: std::vec::Vec<u16> = cancels.iter().collect();
        assert_eq!(all, std::vec![1, 3, 5]);
    }

    #[test]
    fn cancels_view_empty() {
        // Build a pack with empty CANCELS_U16 section
        let section_count = 1u32;
        let data_off = (HEADER_SIZE + SECTION_HEADER_SIZE) as u32;
        let total_len = data_off;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));
        data.extend_from_slice(&build_section_header(SECTION_CANCELS_U16, data_off, 0, 2));

        let pack = PackView::parse(&data).expect("parse pack");
        let cancels = pack.cancels().expect("cancels view");

        assert_eq!(cancels.len(), 0);
        assert!(cancels.is_empty());
        assert_eq!(cancels.get(0), None);

        let all: std::vec::Vec<u16> = cancels.iter().collect();
        assert!(all.is_empty());
    }

    #[test]
    fn hit_window_has_pushback_accessors() {
        // Build a HitWindow with 28 bytes (extended with pushback data)
        let mut data = [0u8; 28];
        // Set hit_pushback at offset 24 (Q12.4: 32 = 2.0 pixels)
        data[24] = 32;
        data[25] = 0;
        // Set block_pushback at offset 26 (Q12.4: 16 = 1.0 pixel)
        data[26] = 16;
        data[27] = 0;

        let view = HitWindowView { data: &data };

        // Test raw values
        assert_eq!(view.hit_pushback_raw(), 32);
        assert_eq!(view.block_pushback_raw(), 16);

        // Test pixel values (Q12.4 >> 4)
        assert_eq!(view.hit_pushback_px(), 2);
        assert_eq!(view.block_pushback_px(), 1);
    }

    #[test]
    fn hit_window_pushback_backwards_compatible() {
        // Build a standard 24-byte HitWindow (no pushback fields)
        let data = [0u8; 24];
        let view = HitWindowView { data: &data };

        // Should return 0 for missing fields
        assert_eq!(view.hit_pushback_raw(), 0);
        assert_eq!(view.block_pushback_raw(), 0);
        assert_eq!(view.hit_pushback_px(), 0);
        assert_eq!(view.block_pushback_px(), 0);
    }

    #[test]
    fn hit_window_pushback_negative_values() {
        // Build a HitWindow with negative pushback (e.g., pull towards attacker)
        let mut data = [0u8; 28];
        // Set hit_pushback at offset 24 (Q12.4: -32 = -2.0 pixels)
        let neg32: i16 = -32;
        data[24..26].copy_from_slice(&(neg32 as u16).to_le_bytes());
        // Set block_pushback at offset 26 (Q12.4: -16 = -1.0 pixel)
        let neg16: i16 = -16;
        data[26..28].copy_from_slice(&(neg16 as u16).to_le_bytes());

        let view = HitWindowView { data: &data };

        // Test raw values (signed)
        assert_eq!(view.hit_pushback_raw(), -32);
        assert_eq!(view.block_pushback_raw(), -16);

        // Test pixel values (Q12.4 >> 4)
        assert_eq!(view.hit_pushback_px(), -2);
        assert_eq!(view.block_pushback_px(), -1);
    }

    // =========================================================================
    // State Tag Views Tests
    // =========================================================================

    #[test]
    fn state_tag_ranges_returns_none_for_missing_section() {
        // Build a minimal valid pack without STATE_TAG_RANGES section
        let pack_data = build_minimal_typed_pack();
        let pack = PackView::parse(&pack_data).expect("should parse pack");

        // The minimal pack doesn't include tag sections
        assert!(pack.state_tag_ranges().is_none());
        assert!(pack.state_tags(0).is_none());
    }

    #[test]
    fn state_tag_ranges_view_basic() {
        // Build a pack with STATE_TAG_RANGES section containing one entry
        // Entry: offset=0, count=2 (meaning 2 tags at offset 0 in STATE_TAGS)
        let mut range_data = [0u8; 8];
        range_data[0..4].copy_from_slice(&0u32.to_le_bytes()); // offset
        range_data[4..6].copy_from_slice(&2u16.to_le_bytes()); // count
        // padding bytes 6-7 left as 0

        let section_count = 1u32;
        let data_off = (HEADER_SIZE + SECTION_HEADER_SIZE) as u32;
        let total_len = data_off + range_data.len() as u32;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));
        data.extend_from_slice(&build_section_header(
            SECTION_STATE_TAG_RANGES,
            data_off,
            range_data.len() as u32,
            4,
        ));
        data.extend_from_slice(&range_data);

        let pack = PackView::parse(&data).expect("parse pack");
        let ranges = pack.state_tag_ranges().expect("state_tag_ranges view");

        assert_eq!(ranges.len(), 1);
        assert!(!ranges.is_empty());

        let (off, count) = ranges.get(0).expect("get entry 0");
        assert_eq!(off, 0);
        assert_eq!(count, 2);

        assert!(ranges.get(1).is_none()); // out of bounds
    }

    #[test]
    fn state_tag_ranges_view_empty() {
        // Build a pack with empty STATE_TAG_RANGES section
        let section_count = 1u32;
        let data_off = (HEADER_SIZE + SECTION_HEADER_SIZE) as u32;
        let total_len = data_off;

        let mut data = std::vec::Vec::new();
        data.extend_from_slice(&build_header(0, total_len, section_count));
        data.extend_from_slice(&build_section_header(
            SECTION_STATE_TAG_RANGES,
            data_off,
            0,
            4,
        ));

        let pack = PackView::parse(&data).expect("parse pack");
        let ranges = pack.state_tag_ranges().expect("state_tag_ranges view");

        assert_eq!(ranges.len(), 0);
        assert!(ranges.is_empty());
        assert!(ranges.get(0).is_none());
    }
}
