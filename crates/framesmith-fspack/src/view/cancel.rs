//! Cancel target and tag rule views.

use crate::bytes::{read_u16_le, read_u32_le, read_u8};

/// StateTagRange record size: off(4) + count(2) + pad(2) = 8 bytes
pub const STATE_TAG_RANGE_SIZE: usize = 8;

/// CancelTagRule record size (24 bytes)
pub const CANCEL_TAG_RULE_SIZE: usize = 24;

/// CancelDeny record size (4 bytes: from u16, to u16)
pub const CANCEL_DENY_SIZE: usize = 4;

/// Zero-copy view over cancel targets (CANCELS_U16 section).
///
/// Each entry is a u16 move ID representing a cancel target.
#[derive(Clone, Copy)]
pub struct CancelsView<'a> {
    data: &'a [u8],
}

impl<'a> CancelsView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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

/// Zero-copy view over STATE_TAG_RANGES section.
///
/// Each entry is a StateTagRange8 (8 bytes): offset(4) + count(2) + padding(2)
/// This section is parallel to MOVES - one entry per state/move.
#[derive(Clone, Copy)]
pub struct StateTagRangesView<'a> {
    data: &'a [u8],
}

impl<'a> StateTagRangesView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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

/// View into a single cancel tag rule.
///
/// Binary layout (24 bytes):
/// - from_tag StrRef: offset(4) + len(2) + pad(2) = 8 bytes (0xFFFFFFFF offset = "any")
/// - to_tag StrRef: offset(4) + len(2) + pad(2) = 8 bytes
/// - condition: u8 bitfield (bit 0=hit, bit 1=block, bit 2=whiff; 7=always)
/// - min_frame: u8
/// - max_frame: u8
/// - flags: u8
/// - padding: 4 bytes
pub struct CancelTagRuleView<'a> {
    data: &'a [u8],
    pack: &'a super::PackView<'a>,
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

    /// Get the condition bitfield.
    ///
    /// Bits: 0=hit, 1=block, 2=whiff
    /// Common values: 7=always, 3=hit+block, 1=hit, 2=block, 4=whiff
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
    pack: &'a super::PackView<'a>,
}

impl<'a> CancelTagRulesView<'a> {
    /// Create a new view from raw bytes and pack reference.
    pub fn new(data: &'a [u8], pack: &'a super::PackView<'a>) -> Self {
        Self { data, pack }
    }

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
