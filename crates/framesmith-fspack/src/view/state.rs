//! State, mesh keys, and keyframes views.

use crate::bytes::{read_u16_le, read_u32_le, read_u8};

/// String reference size: off(4) + len(2) + pad(2)
pub const STRREF_SIZE: usize = 8;

/// State record size (see StateRecord in module docs)
pub const STATE_RECORD_SIZE: usize = 36;

/// Sentinel value for "no mesh" or "no keyframes" (u16::MAX)
pub const KEY_NONE: u16 = 0xFFFF;

/// Zero-copy view over the mesh keys section.
///
/// Each entry is a StrRef (8 bytes): off(4) + len(2) + pad(2)
#[derive(Clone, Copy)]
pub struct MeshKeysView<'a> {
    data: &'a [u8],
}

impl<'a> MeshKeysView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
