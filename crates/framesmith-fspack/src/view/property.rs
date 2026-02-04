//! Character and state property views.

use crate::bytes::{read_i32_le, read_u16_le, read_u32_le, read_u8};

/// Character property record size (12 bytes)
/// Layout: name_off(4) + name_len(2) + value_type(1) + pad(1) + value(4)
pub const CHARACTER_PROP_SIZE: usize = 12;

/// Zero-copy view over a single character property (12 bytes).
///
/// Layout:
/// - 0-3: name_off (u32) - offset into string table
/// - 4-5: name_len (u16) - length of name in string table
/// - 6: value_type (u8) - 0=Q24.8, 1=bool, 2=string ref
/// - 7: reserved (u8)
/// - 8-11: value (4 bytes, interpretation depends on value_type)
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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
