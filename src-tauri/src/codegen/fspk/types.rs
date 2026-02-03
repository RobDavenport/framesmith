//! Core types for FSPK binary packing.

use std::collections::HashMap;

use super::utils::{checked_u16, checked_u32};

/// A string reference as (offset, length) pair into the string table.
pub type StrRef = (u32, u16);

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

/// Cancel lookup data for export.
///
/// Maps move input notation to move index for resolving cancel denies.
pub struct CancelLookup<'a> {
    /// Map from input notation to move index
    pub input_to_index: HashMap<&'a str, u16>,
}

/// Packed move data with backing arrays.
pub struct PackedMoveData {
    /// MOVES section: array of MoveRecord (36 bytes each)
    pub moves: Vec<u8>,
    /// SHAPES section: array of Shape12 (12 bytes each)
    pub shapes: Vec<u8>,
    /// HIT_WINDOWS section: array of HitWindow24 (24 bytes each)
    pub hit_windows: Vec<u8>,
    /// HURT_WINDOWS section: array of HurtWindow12 (12 bytes each)
    pub hurt_windows: Vec<u8>,
    /// PUSH_WINDOWS section: array of PushWindow12 (12 bytes each, same format as HurtWindow12)
    pub push_windows: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_ne!(loc1.0, loc2.0, "Different strings should have different offsets");
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
}
