//! Event emission and argument views.

use crate::bytes::{read_f32_le, read_i64_le, read_u16_le, read_u32_le, read_u64_le, read_u8};

/// EventEmit record size
pub const EVENT_EMIT_SIZE: usize = 16;

/// EventArg record size
pub const EVENT_ARG_SIZE: usize = 20;

// Event arg tags
pub const EVENT_ARG_TAG_BOOL: u8 = 0;
pub const EVENT_ARG_TAG_I64: u8 = 1;
pub const EVENT_ARG_TAG_F32: u8 = 2;
pub const EVENT_ARG_TAG_STRING: u8 = 3;

/// Helper to read a (u32 offset, u16 length) pair.
#[inline]
fn read_range(data: &[u8], base: usize) -> Option<(u32, u16)> {
    let off = read_u32_le(data, base)?;
    let len = read_u16_le(data, base + 4)?;
    Some((off, len))
}

/// Zero-copy view over event emits.
#[derive(Clone, Copy)]
pub struct EventEmitsView<'a> {
    data: &'a [u8],
}

impl<'a> EventEmitsView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
