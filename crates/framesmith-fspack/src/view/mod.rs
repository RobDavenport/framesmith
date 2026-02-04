//! Zero-copy view into an FSPK pack.

use crate::bytes::{read_u16_le, read_u32_le};
use crate::error::Error;

// Declare submodules
mod cancel;
mod event;
mod hitbox;
mod hurtbox;
mod property;
mod resource;
mod schema;
mod state;

// Re-export everything from submodules
pub use cancel::*;
pub use event::*;
pub use hitbox::*;
pub use hurtbox::*;
pub use property::*;
pub use resource::*;
pub use schema::*;
pub use state::*;

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
pub const MAX_SECTIONS: usize = 24; // Increased for tag and cancel rule sections

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

/// Per-state properties (fixed 12-byte records, same format as CHARACTER_PROPS)
/// Format: index array (8 bytes per state: offset u32 + count u16 + pad u16) followed by props data
/// Each property record is 12 bytes: name_off(4) + name_len(2) + type(1) + pad(1) + value(4)
pub const SECTION_STATE_PROPS: u32 = 23;

/// Schema section defining property and tag names.
/// When present, property records use 8-byte schema-based format instead of 12-byte.
pub const SECTION_SCHEMA: u32 = 24;

/// StatePropsIndex entry size: offset(4) + len(2) + pad(2) = 8 bytes
pub const STATE_PROPS_INDEX_ENTRY_SIZE: usize = 8;

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

    /// Get the raw string pool bytes for direct access.
    ///
    /// Returns the STRING_TABLE section bytes, or an empty slice if not present.
    pub fn string_pool(&self) -> &'a [u8] {
        self.get_section(SECTION_STRING_TABLE).unwrap_or(&[])
    }

    /// Get mesh keys section as a typed view.
    ///
    /// Returns `None` if no mesh keys section exists.
    pub fn mesh_keys(&self) -> Option<MeshKeysView<'a>> {
        let data = self.get_section(SECTION_MESH_KEYS)?;
        Some(MeshKeysView::new(data))
    }

    /// Get keyframes keys section as a typed view.
    ///
    /// Returns `None` if no keyframes keys section exists.
    pub fn keyframes_keys(&self) -> Option<KeyframesKeysView<'a>> {
        let data = self.get_section(SECTION_KEYFRAMES_KEYS)?;
        Some(KeyframesKeysView::new(data))
    }

    /// Get states section as a typed view.
    ///
    /// Returns `None` if no states section exists.
    pub fn states(&self) -> Option<StatesView<'a>> {
        let data = self.get_section(SECTION_STATES)?;
        Some(StatesView::new(data))
    }

    /// Get resource definitions as a typed view.
    pub fn resource_defs(&self) -> Option<ResourceDefsView<'a>> {
        let data = self.get_section(SECTION_RESOURCE_DEFS)?;
        Some(ResourceDefsView::new(data))
    }

    /// Get per-state extras as a typed view.
    pub fn state_extras(&self) -> Option<StateExtrasView<'a>> {
        let data = self.get_section(SECTION_STATE_EXTRAS)?;
        Some(StateExtrasView::new(data))
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
        Some(EventEmitsView::new(data))
    }

    /// Get event args as a typed view.
    pub fn event_args(&self) -> Option<EventArgsView<'a>> {
        let data = self.get_section(SECTION_EVENT_ARGS)?;
        Some(EventArgsView::new(data))
    }

    /// Get move notifies as a typed view.
    pub fn move_notifies(&self) -> Option<MoveNotifiesView<'a>> {
        let data = self.get_section(SECTION_MOVE_NOTIFIES)?;
        Some(MoveNotifiesView::new(data))
    }

    /// Get move resource costs as a typed view.
    pub fn move_resource_costs(&self) -> Option<MoveResourceCostsView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_COSTS)?;
        Some(MoveResourceCostsView::new(data))
    }

    /// Get move resource preconditions as a typed view.
    pub fn move_resource_preconditions(&self) -> Option<MoveResourcePreconditionsView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_PRECONDITIONS)?;
        Some(MoveResourcePreconditionsView::new(data))
    }

    /// Get move resource deltas as a typed view.
    pub fn move_resource_deltas(&self) -> Option<MoveResourceDeltasView<'a>> {
        let data = self.get_section(SECTION_MOVE_RESOURCE_DELTAS)?;
        Some(MoveResourceDeltasView::new(data))
    }

    /// Get cancel targets as a typed view.
    ///
    /// Returns `None` if no cancels section exists.
    pub fn cancels(&self) -> Option<CancelsView<'a>> {
        let data = self.get_section(SECTION_CANCELS_U16)?;
        Some(CancelsView::new(data))
    }

    /// Get hit windows section as a typed view.
    ///
    /// Returns `None` if no hit windows section exists.
    pub fn hit_windows(&self) -> Option<HitWindowsView<'a>> {
        let data = self.get_section(SECTION_HIT_WINDOWS)?;
        Some(HitWindowsView::new(data))
    }

    /// Get hurt windows section as a typed view.
    ///
    /// Returns `None` if no hurt windows section exists.
    pub fn hurt_windows(&self) -> Option<HurtWindowsView<'a>> {
        let data = self.get_section(SECTION_HURT_WINDOWS)?;
        Some(HurtWindowsView::new(data))
    }

    /// Get push windows section as a typed view.
    ///
    /// Returns `None` if no push windows section exists.
    pub fn push_windows(&self) -> Option<PushWindowsView<'a>> {
        let data = self.get_section(SECTION_PUSH_WINDOWS)?;
        Some(PushWindowsView::new(data))
    }

    /// Get shapes section as a typed view.
    ///
    /// Returns `None` if no shapes section exists.
    pub fn shapes(&self) -> Option<ShapesView<'a>> {
        let data = self.get_section(SECTION_SHAPES)?;
        Some(ShapesView::new(data))
    }

    /// Get the state tag ranges section as a typed view.
    ///
    /// Returns `None` if no STATE_TAG_RANGES section exists.
    pub fn state_tag_ranges(&self) -> Option<StateTagRangesView<'a>> {
        let data = self.get_section(SECTION_STATE_TAG_RANGES)?;
        Some(StateTagRangesView::new(data))
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
        Some(CancelTagRulesView::new(data, self))
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
        Some(CharacterPropsView::new(data))
    }

    /// Get raw property record bytes for a state.
    ///
    /// Returns `None` if:
    /// - No STATE_PROPS section exists
    /// - The state index is out of bounds
    /// - The state has no properties (len == 0)
    ///
    /// The returned bytes contain fixed 12-byte property records.
    /// Each record: name_off(u32) + name_len(u16) + type(u8) + pad(u8) + value(4 bytes).
    /// Use the string pool to look up property names from (name_off, name_len).
    pub fn state_props_raw(&self, state_idx: usize) -> Option<&'a [u8]> {
        let section = self.get_section(SECTION_STATE_PROPS)?;

        // The section starts with an index: one entry per state
        // Each entry is 8 bytes: offset(4) + len(2) + pad(2)
        let index_entry_off = state_idx.checked_mul(STATE_PROPS_INDEX_ENTRY_SIZE)?;

        // Read the index entry
        let off = read_u32_le(section, index_entry_off)? as usize;
        let len = read_u16_le(section, index_entry_off + 4)? as usize;

        // Empty properties (len == 0) means no data
        if len == 0 {
            return None;
        }

        // Bounds check
        let end = off.checked_add(len)?;
        if end > section.len() {
            return None;
        }

        Some(&section[off..end])
    }

    /// Check if a state has properties.
    pub fn has_state_props(&self, state_idx: usize) -> bool {
        self.state_props_raw(state_idx).is_some()
    }

    /// Check if this pack has a schema section.
    pub fn has_schema(&self) -> bool {
        self.get_section(SECTION_SCHEMA).is_some()
    }

    /// Get the schema section as a typed view.
    ///
    /// Returns `None` if no SECTION_SCHEMA exists.
    pub fn schema(&self) -> Option<SchemaView<'a>> {
        let data = self.get_section(SECTION_SCHEMA)?;
        let string_pool = self.string_pool();
        SchemaView::new(data, string_pool)
    }

    /// Get schema-based character properties (8-byte records).
    ///
    /// Returns `None` if no CHARACTER_PROPS section exists or no schema is present.
    /// When schema is present, CHARACTER_PROPS uses 8-byte records instead of 12-byte.
    pub fn schema_character_props(&self) -> Option<SchemaCharacterPropsView<'a>> {
        // Only return schema props view if schema section exists
        if !self.has_schema() {
            return None;
        }
        let data = self.get_section(SECTION_CHARACTER_PROPS)?;
        Some(SchemaCharacterPropsView::new(data))
    }
}
