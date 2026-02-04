//! Schema section views for efficient property lookup.
//!
//! When a SECTION_SCHEMA is present, properties use schema IDs instead of
//! embedded string references, reducing record size from 12 to 8 bytes.

use crate::bytes::{read_i32_le, read_u16_le, read_u32_le, read_u8};

/// Schema-based property record size (8 bytes).
/// Layout: schema_id(2) + value_type(1) + reserved(1) + value(4)
pub const SCHEMA_PROP_SIZE: usize = 8;

/// Schema header size (8 bytes).
/// Layout: char_prop_count(2) + state_prop_count(2) + tag_count(2) + padding(2)
pub const SCHEMA_HEADER_SIZE: usize = 8;

/// String reference size in schema: offset(4) + length(2) + padding(2)
const STRREF_SIZE: usize = 8;

/// Zero-copy view over the schema section.
///
/// The schema defines property and tag names, allowing property records
/// to use compact schema IDs instead of embedding string references.
///
/// Layout:
/// - Header (8 bytes): char_prop_count(2) + state_prop_count(2) + tag_count(2) + padding(2)
/// - Character property names: [StringRef; char_prop_count]
/// - State property names: [StringRef; state_prop_count]
/// - Tag names: [StringRef; tag_count]
#[derive(Clone, Copy)]
pub struct SchemaView<'a> {
    data: &'a [u8],
    string_pool: &'a [u8],
    char_prop_count: u16,
    state_prop_count: u16,
    tag_count: u16,
}

impl<'a> SchemaView<'a> {
    /// Create a new schema view from raw bytes.
    ///
    /// # Arguments
    /// * `data` - The SECTION_SCHEMA bytes
    /// * `string_pool` - The STRING_TABLE bytes for resolving names
    pub fn new(data: &'a [u8], string_pool: &'a [u8]) -> Option<Self> {
        if data.len() < SCHEMA_HEADER_SIZE {
            return None;
        }

        let char_prop_count = read_u16_le(data, 0)?;
        let state_prop_count = read_u16_le(data, 2)?;
        let tag_count = read_u16_le(data, 4)?;

        Some(Self {
            data,
            string_pool,
            char_prop_count,
            state_prop_count,
            tag_count,
        })
    }

    /// Number of character property names in the schema.
    pub fn char_prop_count(&self) -> usize {
        self.char_prop_count as usize
    }

    /// Number of state property names in the schema.
    pub fn state_prop_count(&self) -> usize {
        self.state_prop_count as usize
    }

    /// Number of tag names in the schema.
    pub fn tag_count(&self) -> usize {
        self.tag_count as usize
    }

    /// Get a character property name by schema ID.
    pub fn char_prop_name(&self, schema_id: u16) -> Option<&'a str> {
        if schema_id as usize >= self.char_prop_count as usize {
            return None;
        }
        let strref_off = SCHEMA_HEADER_SIZE + (schema_id as usize) * STRREF_SIZE;
        self.read_strref(strref_off)
    }

    /// Get a state property name by schema ID.
    pub fn state_prop_name(&self, schema_id: u16) -> Option<&'a str> {
        if schema_id as usize >= self.state_prop_count as usize {
            return None;
        }
        let char_props_end = SCHEMA_HEADER_SIZE + (self.char_prop_count as usize) * STRREF_SIZE;
        let strref_off = char_props_end + (schema_id as usize) * STRREF_SIZE;
        self.read_strref(strref_off)
    }

    /// Get a tag name by schema ID.
    pub fn tag_name(&self, schema_id: u16) -> Option<&'a str> {
        if schema_id as usize >= self.tag_count as usize {
            return None;
        }
        let char_props_end = SCHEMA_HEADER_SIZE + (self.char_prop_count as usize) * STRREF_SIZE;
        let state_props_end = char_props_end + (self.state_prop_count as usize) * STRREF_SIZE;
        let strref_off = state_props_end + (schema_id as usize) * STRREF_SIZE;
        self.read_strref(strref_off)
    }

    /// Read a string reference at the given offset and resolve it.
    fn read_strref(&self, off: usize) -> Option<&'a str> {
        if off + STRREF_SIZE > self.data.len() {
            return None;
        }
        let str_off = read_u32_le(self.data, off)? as usize;
        let str_len = read_u16_le(self.data, off + 4)? as usize;

        let end = str_off.checked_add(str_len)?;
        if end > self.string_pool.len() {
            return None;
        }
        core::str::from_utf8(&self.string_pool[str_off..end]).ok()
    }
}

/// Zero-copy view over a single schema-based character property (8 bytes).
///
/// Layout:
/// - 0-1: schema_id (u16) - index into schema's character property names
/// - 2: value_type (u8) - 0=Q24.8, 1=bool, 2=string ref
/// - 3: reserved (u8)
/// - 4-7: value (4 bytes, interpretation depends on value_type)
#[derive(Clone, Copy)]
pub struct SchemaCharacterPropView<'a> {
    data: &'a [u8],
}

impl<'a> SchemaCharacterPropView<'a> {
    /// Get the schema ID for looking up the property name.
    pub fn schema_id(&self) -> u16 {
        read_u16_le(self.data, 0).unwrap_or(0)
    }

    /// Get the value type tag.
    /// 0 = i32 (Q24.8 fixed-point), 1 = bool, 2 = string reference
    pub fn value_type(&self) -> u8 {
        read_u8(self.data, 2).unwrap_or(0)
    }

    /// Get the raw value as u32.
    pub fn value_raw(&self) -> u32 {
        read_u32_le(self.data, 4).unwrap_or(0)
    }

    /// Interpret the value as Q24.8 fixed-point (signed).
    pub fn as_q24_8(&self) -> i32 {
        read_i32_le(self.data, 4).unwrap_or(0)
    }

    /// Interpret the value as boolean.
    pub fn as_bool(&self) -> bool {
        read_u8(self.data, 4).unwrap_or(0) != 0
    }

    /// Interpret the value as a string reference (offset, length).
    /// The offset is stored in the lower 16 bits, length in the upper 16 bits.
    pub fn as_str_ref(&self) -> (u16, u16) {
        let off = read_u16_le(self.data, 4).unwrap_or(0);
        let len = read_u16_le(self.data, 6).unwrap_or(0);
        (off, len)
    }
}

/// Zero-copy view over schema-based character properties.
///
/// Each entry is 8 bytes (vs 12 for non-schema properties).
#[derive(Clone, Copy)]
pub struct SchemaCharacterPropsView<'a> {
    data: &'a [u8],
}

impl<'a> SchemaCharacterPropsView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns the number of character properties.
    pub fn len(&self) -> usize {
        self.data.len() / SCHEMA_PROP_SIZE
    }

    /// Returns true if there are no character properties.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get a character property by index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<SchemaCharacterPropView<'a>> {
        let off = index.checked_mul(SCHEMA_PROP_SIZE)?;
        let end = off.checked_add(SCHEMA_PROP_SIZE)?;
        if end > self.data.len() {
            return None;
        }
        Some(SchemaCharacterPropView {
            data: &self.data[off..end],
        })
    }

    /// Returns an iterator over all character properties.
    pub fn iter(&self) -> impl Iterator<Item = SchemaCharacterPropView<'a>> + '_ {
        (0..self.len()).filter_map(|i| self.get(i))
    }
}
