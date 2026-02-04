//! Resource definitions, state extras, and move resource views.

use crate::bytes::{read_i32_le, read_u16_le, read_u32_le, read_u8};

/// ResourceDef record size
pub const RESOURCE_DEF_SIZE: usize = 12;

/// StateExtras record size (expanded from 64 to 72 for cancel offset/length)
pub const STATE_EXTRAS_SIZE: usize = 72;

/// MoveNotify record size
pub const MOVE_NOTIFY_SIZE: usize = 12;

/// MoveResourceCost record size
pub const MOVE_RESOURCE_COST_SIZE: usize = 12;

/// MoveResourcePrecondition record size
pub const MOVE_RESOURCE_PRECONDITION_SIZE: usize = 12;

/// MoveResourceDelta record size
pub const MOVE_RESOURCE_DELTA_SIZE: usize = 16;

/// Sentinel value for an absent optional u16.
pub const OPT_U16_NONE: u16 = 0xFFFF;

// Resource Delta Trigger Tags
pub const RESOURCE_DELTA_TRIGGER_ON_USE: u8 = 0;
pub const RESOURCE_DELTA_TRIGGER_ON_HIT: u8 = 1;
pub const RESOURCE_DELTA_TRIGGER_ON_BLOCK: u8 = 2;

/// Helper to read a (u32 offset, u16 length) pair.
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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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

/// Zero-copy view over move notify records.
#[derive(Clone, Copy)]
pub struct MoveNotifiesView<'a> {
    data: &'a [u8],
}

impl<'a> MoveNotifiesView<'a> {
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
    /// Create a new view from raw bytes.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

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
