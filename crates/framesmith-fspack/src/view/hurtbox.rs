//! Hurtbox and pushbox window views.

use crate::bytes::{read_u16_le, read_u32_le, read_u8};

/// HurtWindow record size (12 bytes)
pub const HURT_WINDOW_SIZE: usize = 12;

/// PushWindow record size (12 bytes) - same layout as HurtWindow
pub const PUSH_WINDOW_SIZE: usize = 12;

/// Zero-copy view over hurt windows section.
///
/// Each entry is a HurtWindow12 (12 bytes).
#[derive(Clone, Copy)]
pub struct HurtWindowsView<'a> {
    data: &'a [u8],
}

impl<'a> HurtWindowsView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
