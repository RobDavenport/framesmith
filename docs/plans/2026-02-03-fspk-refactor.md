# FSPK Module Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rename `zx_fspack` → `fspk` and split the 2,773-line monolith into focused modules for LLM-friendliness.

**Architecture:** Create `fspk/` subdirectory module with 8 focused files (~150-400 lines each). Rename `zx_fspack_format.rs` → `fspk_format.rs`. Update all callers.

**Tech Stack:** Rust (Tauri backend), Svelte/TypeScript (frontend)

---

## Task 1: Create fspk/ Directory Structure

**Files:**
- Create: `src-tauri/src/codegen/fspk/mod.rs`

**Step 1: Create the module directory and mod.rs**

```rust
//! FSPK (Framesmith Pack) binary export adapter
//!
//! This module exports character data to the FSPK binary format.
//! The format is engine-agnostic and optimized for no_std/WASM runtimes.

mod export;
mod moves;
mod packing;
mod properties;
mod sections;
mod types;
mod utils;

pub use export::export_fspk;
pub use moves::{build_asset_keys, pack_moves};
pub use properties::pack_character_props;
pub use types::{CancelLookup, PackedMoveData, StrRef, StringTable};
```

**Step 2: Verify the directory was created**

Run: `dir src-tauri\src\codegen\fspk`
Expected: mod.rs exists

---

## Task 2: Extract types.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/types.rs`

**Step 1: Create types.rs with StringTable, CancelLookup, PackedMoveData, StrRef**

Extract from `zx_fspack.rs` lines 50-111 (StringTable) and 324-344 (CancelLookup, PackedMoveData, StrRef):

```rust
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
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles (will have errors until all modules exist)

---

## Task 3: Extract utils.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/utils.rs`

**Step 1: Create utils.rs with helper functions**

Extract from `zx_fspack.rs` lines 22-48 and 557-592:

```rust
//! Utility functions for FSPK binary packing.

use crate::codegen::fspk_format::{write_u16_le, write_u32_le};

use super::types::StrRef;

pub fn checked_u16(value: usize, what: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{} overflows u16: {}", what, value))
}

pub fn checked_u32(value: usize, what: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{} overflows u32: {}", what, value))
}

pub fn align_up(value: usize, align: u32) -> Result<usize, String> {
    if align == 0 {
        return Err("alignment must be non-zero".to_string());
    }
    if !align.is_power_of_two() {
        return Err(format!("alignment must be power of two, got {}", align));
    }

    let align = align as usize;
    if align == 1 {
        return Ok(value);
    }

    let mask = align - 1;
    let v = value
        .checked_add(mask)
        .ok_or_else(|| "align_up overflow".to_string())?;
    Ok(v & !mask)
}

/// Write a string reference (StrRef) to the buffer.
///
/// StrRef layout: offset(u32) + length(u16) + padding(u16)
pub fn write_strref(buf: &mut Vec<u8>, strref: StrRef) {
    write_u32_le(buf, strref.0); // offset
    write_u16_le(buf, strref.1); // length
    write_u16_le(buf, 0); // padding
}

pub fn write_range(buf: &mut Vec<u8>, off: u32, len: u16) {
    write_u32_le(buf, off);
    write_u16_le(buf, len);
    write_u16_le(buf, 0);
}

pub fn write_i32_le(buf: &mut Vec<u8>, value: i32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_i64_le(buf: &mut Vec<u8>, value: i64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u64_le(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

/// Write a section header to the buffer.
///
/// Section header layout: kind(u32) + offset(u32) + length(u32) + alignment(u32)
pub fn write_section_header(buf: &mut Vec<u8>, kind: u32, offset: u32, length: u32, alignment: u32) {
    write_u32_le(buf, kind);
    write_u32_le(buf, offset);
    write_u32_le(buf, length);
    write_u32_le(buf, alignment);
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 4: Extract packing.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/packing.rs`

**Step 1: Create packing.rs with shape/hitbox/move record packing**

Extract from `zx_fspack.rs` lines 117-322:

```rust
//! Binary record packing for shapes, hitboxes, and move records.

use crate::codegen::fspk_format::{
    to_q12_4, to_q12_4_unsigned, HIT_WINDOW24_SIZE, HURT_WINDOW12_SIZE, SHAPE12_SIZE,
    SHAPE_KIND_AABB, STATE_RECORD_SIZE,
};
use crate::schema::{FrameHitbox, GuardType, Rect, State};

/// Pack a Rect into a Shape12 (AABB) structure.
///
/// Shape12 layout:
/// - kind (u8): shape type (0 = AABB)
/// - flags (u8): reserved
/// - a (i16): x position (Q12.4)
/// - b (i16): y position (Q12.4)
/// - c (u16): width (Q12.4 unsigned)
/// - d (u16): height (Q12.4 unsigned)
/// - e (i16): unused for AABB
pub fn pack_shape(rect: &Rect) -> [u8; SHAPE12_SIZE] {
    let mut buf = [0u8; SHAPE12_SIZE];
    buf[0] = SHAPE_KIND_AABB; // kind
    buf[1] = 0; // flags

    let x = to_q12_4(rect.x as f32);
    let y = to_q12_4(rect.y as f32);
    let w = to_q12_4_unsigned(rect.w as f32);
    let h = to_q12_4_unsigned(rect.h as f32);

    buf[2..4].copy_from_slice(&x.to_le_bytes()); // a = x
    buf[4..6].copy_from_slice(&y.to_le_bytes()); // b = y
    buf[6..8].copy_from_slice(&w.to_le_bytes()); // c = w
    buf[8..10].copy_from_slice(&h.to_le_bytes()); // d = h
    buf[10..12].copy_from_slice(&0i16.to_le_bytes()); // e = 0

    buf
}

/// Convert GuardType to u8 for binary encoding.
pub fn guard_type_to_u8(guard: &GuardType) -> u8 {
    match guard {
        GuardType::High => 0,
        GuardType::Mid => 1,
        GuardType::Low => 2,
        GuardType::Unblockable => 3,
    }
}

/// Pack a FrameHitbox into a HitWindow24 structure.
///
/// HitWindow24 layout (24 bytes) - must match view.rs HitWindowView:
/// - 0: start_frame (u8)
/// - 1: end_frame (u8)
/// - 2: guard (u8)
/// - 3: reserved (u8)
/// - 4-5: damage (u16 LE)
/// - 6-7: chip_damage (u16 LE)
/// - 8: hitstun (u8)
/// - 9: blockstun (u8)
/// - 10: hitstop (u8)
/// - 11: reserved (u8)
/// - 12-15: shapes_off (u32 LE)
/// - 16-17: shapes_len (u16 LE)
/// - 18-21: cancels_off (u32 LE)
/// - 22-23: cancels_len (u16 LE)
pub fn pack_hit_window(
    hb: &FrameHitbox,
    shapes_off: u32,
    damage: u16,
    hitstun: u8,
    blockstun: u8,
    hitstop: u8,
    guard: u8,
) -> [u8; HIT_WINDOW24_SIZE] {
    let mut buf = [0u8; HIT_WINDOW24_SIZE];

    buf[0] = hb.frames.0; // start_frame
    buf[1] = hb.frames.1; // end_frame
    buf[2] = guard; // guard
    buf[3] = 0; // reserved
    buf[4..6].copy_from_slice(&damage.to_le_bytes()); // damage
    buf[6..8].copy_from_slice(&0u16.to_le_bytes()); // chip_damage (TODO: add to schema)
    buf[8] = hitstun; // hitstun
    buf[9] = blockstun; // blockstun
    buf[10] = hitstop; // hitstop
    buf[11] = 0; // reserved
    buf[12..16].copy_from_slice(&shapes_off.to_le_bytes()); // shapes_off
    buf[16..18].copy_from_slice(&1u16.to_le_bytes()); // shapes_len = 1
    // bytes 18-27 are cancels/pushback (already zeroed, not used in v1)

    buf
}

/// Pack a FrameHitbox into a HurtWindow12 structure.
///
/// HurtWindow12 layout (12 bytes):
/// - frame_start (u8): first active frame
/// - frame_end (u8): last active frame
/// - shape_off (u32): offset into SHAPES section
/// - shape_count (u16): number of shapes (always 1 for v1)
/// - flags (u16): hurtbox flags (invuln, armor, etc.)
/// - reserved (2 bytes): padding
pub fn pack_hurt_window(hb: &FrameHitbox, shapes_off: u32) -> [u8; HURT_WINDOW12_SIZE] {
    let mut buf = [0u8; HURT_WINDOW12_SIZE];

    buf[0] = hb.frames.0; // frame_start
    buf[1] = hb.frames.1; // frame_end
    buf[2..6].copy_from_slice(&shapes_off.to_le_bytes()); // shape_off
    buf[6..8].copy_from_slice(&1u16.to_le_bytes()); // shape_count = 1
    buf[8..10].copy_from_slice(&0u16.to_le_bytes()); // flags = 0 for v1
                                                     // bytes 10-11 are reserved/padding (already zeroed)

    buf
}

/// Convert move type string to u8 for binary encoding.
/// Maps common type strings to fixed IDs for runtime compatibility.
pub fn move_type_to_u8(move_type: Option<&String>) -> u8 {
    match move_type.map(|s| s.as_str()) {
        Some("normal") => 0,
        Some("command_normal") => 1,
        Some("special") => 2,
        Some("super") => 3,
        Some("movement") => 4,
        Some("throw") => 5,
        Some("ex") => 6,
        Some("rekka") => 7,
        Some(_) => 255, // unknown custom type
        None => 0,      // default to normal
    }
}

/// Convert TriggerType to u8 for binary encoding.
pub fn trigger_type_to_u8(trigger: Option<&crate::schema::TriggerType>) -> u8 {
    use crate::schema::TriggerType;
    match trigger {
        Some(TriggerType::Press) => 0,
        Some(TriggerType::Release) => 1,
        Some(TriggerType::Hold) => 2,
        None => 0, // default to Press
    }
}

/// Pack a Move into a MoveRecord structure.
///
/// MoveRecord layout (36 bytes):
/// - 0-1: move_id (u16)
/// - 2-3: mesh_key (u16)
/// - 4-5: keyframes_key (u16)
/// - 6: move_type (u8)
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
#[allow(clippy::too_many_arguments)] // Binary record packing requires all fields
pub fn pack_move_record(
    move_id: u16,
    mesh_key: u16,
    keyframes_key: u16,
    mv: &State,
    hit_windows_off: u32,
    hit_windows_len: u16,
    hurt_windows_off: u16,
    hurt_windows_len: u16,
    push_windows_off: u16,
    push_windows_len: u16,
    flags: u8,
) -> [u8; STATE_RECORD_SIZE] {
    let mut buf = [0u8; STATE_RECORD_SIZE];

    buf[0..2].copy_from_slice(&move_id.to_le_bytes()); // move_id
    buf[2..4].copy_from_slice(&mesh_key.to_le_bytes()); // mesh_key
    buf[4..6].copy_from_slice(&keyframes_key.to_le_bytes()); // keyframes_key
    buf[6] = move_type_to_u8(mv.move_type.as_ref()); // move_type
    buf[7] = trigger_type_to_u8(mv.trigger.as_ref()); // trigger
    buf[8] = guard_type_to_u8(&mv.guard); // guard
    buf[9] = flags; // cancel flags
    buf[10] = mv.startup; // startup
    buf[11] = mv.active; // active
    buf[12] = mv.recovery; // recovery
    buf[13] = 0; // reserved
    let total = mv
        .total
        .map(|t| t as u16)
        .unwrap_or_else(|| (mv.startup as u16) + (mv.active as u16) + (mv.recovery as u16));
    buf[14..16].copy_from_slice(&total.to_le_bytes()); // total
    buf[16..18].copy_from_slice(&mv.damage.to_le_bytes()); // damage
    buf[18] = mv.hitstun; // hitstun
    buf[19] = mv.blockstun; // blockstun
    buf[20] = mv.hitstop; // hitstop
    buf[21] = 0; // reserved
    buf[22..26].copy_from_slice(&hit_windows_off.to_le_bytes()); // hit_windows_off
    buf[26..28].copy_from_slice(&hit_windows_len.to_le_bytes()); // hit_windows_len
    buf[28..30].copy_from_slice(&hurt_windows_off.to_le_bytes()); // hurt_windows_off (u16)
    buf[30..32].copy_from_slice(&hurt_windows_len.to_le_bytes()); // hurt_windows_len
    buf[32..34].copy_from_slice(&push_windows_off.to_le_bytes()); // push_windows_off (u16)
    buf[34..36].copy_from_slice(&push_windows_len.to_le_bytes()); // push_windows_len

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fspk_format::SHAPE_KIND_AABB;
    use crate::schema::{MeterGain, Pushback};

    fn make_test_rect() -> Rect {
        Rect { x: 10, y: 20, w: 50, h: 60 }
    }

    fn make_test_hitbox() -> FrameHitbox {
        FrameHitbox {
            frames: (5, 8),
            r#box: make_test_rect(),
        }
    }

    #[test]
    fn test_pack_shape() {
        let rect = make_test_rect();
        let shape = pack_shape(&rect);

        assert_eq!(shape.len(), SHAPE12_SIZE);
        assert_eq!(shape[0], SHAPE_KIND_AABB);
        assert_eq!(shape[1], 0); // flags

        // x=10 -> Q12.4 = 160 = 0x00A0
        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, 160);

        // y=20 -> Q12.4 = 320 = 0x0140
        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, 320);

        // w=50 -> Q12.4 = 800 = 0x0320
        let w = u16::from_le_bytes([shape[6], shape[7]]);
        assert_eq!(w, 800);

        // h=60 -> Q12.4 = 960 = 0x03C0
        let h = u16::from_le_bytes([shape[8], shape[9]]);
        assert_eq!(h, 960);
    }

    #[test]
    fn test_pack_hit_window() {
        let hb = make_test_hitbox();
        let hw = pack_hit_window(&hb, 100, 500, 12, 8, 10, 1);

        assert_eq!(hw.len(), HIT_WINDOW24_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end
        assert_eq!(hw[2], 1); // guard (mid)
    }

    #[test]
    fn test_pack_hurt_window() {
        let hb = make_test_hitbox();
        let hw = pack_hurt_window(&hb, 200);

        assert_eq!(hw.len(), HURT_WINDOW12_SIZE);
        assert_eq!(hw[0], 5); // frame_start
        assert_eq!(hw[1], 8); // frame_end

        let shape_off = u32::from_le_bytes([hw[2], hw[3], hw[4], hw[5]]);
        assert_eq!(shape_off, 200);
    }

    #[test]
    fn test_guard_type_encoding() {
        assert_eq!(guard_type_to_u8(&GuardType::High), 0);
        assert_eq!(guard_type_to_u8(&GuardType::Mid), 1);
        assert_eq!(guard_type_to_u8(&GuardType::Low), 2);
        assert_eq!(guard_type_to_u8(&GuardType::Unblockable), 3);
    }

    #[test]
    fn test_negative_coordinates() {
        let rect = Rect { x: -50, y: -100, w: 30, h: 40 };
        let shape = pack_shape(&rect);

        let x = i16::from_le_bytes([shape[2], shape[3]]);
        assert_eq!(x, -800); // -50 -> Q12.4 = -800

        let y = i16::from_le_bytes([shape[4], shape[5]]);
        assert_eq!(y, -1600); // -100 -> Q12.4 = -1600
    }
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 5: Extract moves.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/moves.rs`

**Step 1: Create moves.rs with pack_moves and build_asset_keys**

Extract from `zx_fspack.rs` lines 346-497:

```rust
//! Move packing and asset key generation.

use std::collections::HashMap;

use crate::codegen::fspk_format::KEY_NONE;
use crate::commands::CharacterData;
use crate::schema::State;

use super::packing::{guard_type_to_u8, pack_hit_window, pack_hurt_window, pack_move_record, pack_shape};
use super::types::{CancelLookup, PackedMoveData, StrRef, StringTable};
use super::utils::{checked_u16, checked_u32};

/// Pack all moves into binary sections.
///
/// Returns packed move data with all backing arrays.
///
/// The `anim_to_index` map provides indices into the MESH_KEYS/KEYFRAMES_KEYS arrays
/// for each animation name. If None, all moves use KEY_NONE for asset references.
///
/// The `cancel_lookup` provides cancel information for setting MoveRecord.flags.
/// If None, all flags are 0.
pub fn pack_moves(
    moves: &[State],
    anim_to_index: Option<&HashMap<String, u16>>,
    cancel_lookup: Option<&CancelLookup>,
) -> Result<PackedMoveData, String> {
    let mut packed = PackedMoveData {
        moves: Vec::new(),
        shapes: Vec::new(),
        hit_windows: Vec::new(),
        hurt_windows: Vec::new(),
        push_windows: Vec::new(),
    };

    for (idx, mv) in moves.iter().enumerate() {
        let move_id = checked_u16(idx, "move_id")?;

        // Look up animation index if map is provided
        let anim_index = anim_to_index
            .and_then(|map| {
                if mv.animation.is_empty() {
                    None
                } else {
                    map.get(&mv.animation).copied()
                }
            })
            .unwrap_or(KEY_NONE);

        // Track offsets before adding this move's data
        let hit_windows_off = checked_u32(packed.hit_windows.len(), "hit_windows_off")?;
        let hurt_windows_off = checked_u16(packed.hurt_windows.len(), "hurt_windows_off")?;
        let push_windows_off = checked_u16(packed.push_windows.len(), "push_windows_off")?;

        // Pack hitboxes -> shapes + hit_windows
        for hb in &mv.hitboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed.hit_windows.extend_from_slice(&pack_hit_window(
                hb,
                shape_off,
                mv.damage,
                mv.hitstun,
                mv.blockstun,
                mv.hitstop,
                guard_type_to_u8(&mv.guard),
            ));
        }

        // Pack hurtboxes -> shapes + hurt_windows
        for hb in &mv.hurtboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&hb.r#box));
            packed.hurt_windows.extend_from_slice(&pack_hurt_window(hb, shape_off));
        }

        // Pack pushboxes -> shapes + push_windows (same 12-byte format as hurt windows)
        for pb in &mv.pushboxes {
            let shape_off = checked_u32(packed.shapes.len(), "shape_off")?;
            packed.shapes.extend_from_slice(&pack_shape(&pb.r#box));
            packed.push_windows.extend_from_slice(&pack_hurt_window(pb, shape_off));
        }

        // Calculate lengths
        let hit_windows_len = checked_u16(mv.hitboxes.len(), "hit_windows_len")?;
        let hurt_windows_len = checked_u16(mv.hurtboxes.len(), "hurt_windows_len")?;
        let push_windows_len = checked_u16(mv.pushboxes.len(), "push_windows_len")?;

        // Cancel flags are now handled via tag_rules, so MoveRecord.flags is always 0
        let flags: u8 = 0;
        let _ = cancel_lookup; // Silence unused warning; used later for deny resolution

        // Pack move record - mesh_key and keyframes_key both use the same animation index
        packed.moves.extend_from_slice(&pack_move_record(
            move_id,
            anim_index, // mesh_key
            anim_index, // keyframes_key
            mv,
            hit_windows_off,
            hit_windows_len,
            hurt_windows_off,
            hurt_windows_len,
            push_windows_off,
            push_windows_len,
            flags,
        ));
    }

    Ok(packed)
}

/// Build asset key arrays from character data.
///
/// Returns two vectors of string references:
/// - `mesh_keys`: Keys for mesh assets, format: "{character_id}.{animation}"
/// - `keyframes_keys`: Keys for keyframes assets, format: "{animation}"
///
/// Keys are sorted deterministically by their string value to ensure
/// reproducible output. Duplicate animations are deduplicated.
pub fn build_asset_keys(
    char_data: &CharacterData,
    strings: &mut StringTable,
) -> Result<(Vec<StrRef>, Vec<StrRef>), String> {
    // Collect unique animation names
    let mut animations: Vec<&str> = char_data
        .moves
        .iter()
        .filter(|m| !m.animation.is_empty())
        .map(|m| m.animation.as_str())
        .collect();

    // Deduplicate and sort for determinism
    animations.sort();
    animations.dedup();

    let character_id = &char_data.character.id;

    // Build mesh keys: "{character_id}.{animation}"
    let mesh_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| {
            let mesh_key = format!("{}.{}", character_id, anim);
            strings.intern(&mesh_key)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Build keyframes keys: just the animation name
    let keyframes_keys: Vec<StrRef> = animations
        .iter()
        .map(|anim| strings.intern(anim))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((mesh_keys, keyframes_keys))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fspk_format::{HURT_WINDOW12_SIZE, STATE_RECORD_SIZE};
    use crate::schema::{CancelTable, Character, FrameHitbox, GuardType, MeterGain, Pushback, Rect, State};
    use std::collections::BTreeMap;

    fn make_test_character(id: &str) -> Character {
        use crate::schema::PropertyValue;
        let mut properties = BTreeMap::new();
        properties.insert("health".to_string(), PropertyValue::Number(1000.0));

        Character {
            id: id.to_string(),
            name: "Test Character".to_string(),
            properties,
            resources: vec![],
        }
    }

    fn make_test_move(input: &str, animation: &str) -> State {
        State {
            input: input.to_string(),
            name: format!("{} attack", input),
            tags: vec![],
            startup: 5,
            active: 3,
            recovery: 10,
            damage: 50,
            hitstun: 15,
            blockstun: 10,
            hitstop: 5,
            guard: GuardType::Mid,
            hitboxes: vec![],
            hurtboxes: vec![],
            pushback: Pushback { hit: 10, block: 5 },
            meter_gain: MeterGain { hit: 10, whiff: 5 },
            animation: animation.to_string(),
            ..Default::default()
        }
    }

    fn make_empty_cancel_table() -> CancelTable {
        CancelTable::default()
    }

    #[test]
    fn test_build_asset_keys_deterministic() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5H", "stand_heavy"),
                make_test_move("5L", "stand_light"),
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings1 = StringTable::new();
        let (mesh_keys1, kf_keys1) = build_asset_keys(&char_data, &mut strings1).unwrap();

        let char_data2 = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5M", "stand_medium"),
                make_test_move("5L", "stand_light"),
                make_test_move("5H", "stand_heavy"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings2 = StringTable::new();
        let (mesh_keys2, kf_keys2) = build_asset_keys(&char_data2, &mut strings2).unwrap();

        assert_eq!(mesh_keys1.len(), mesh_keys2.len());
        assert_eq!(kf_keys1.len(), kf_keys2.len());
        assert_eq!(strings1.into_bytes(), strings2.into_bytes());
    }

    #[test]
    fn test_build_asset_keys_deduplication() {
        let char_data = CharacterData {
            character: make_test_character("test_char"),
            moves: vec![
                make_test_move("5L", "stand_light"),
                make_test_move("2L", "stand_light"), // Same animation
                make_test_move("5M", "stand_medium"),
            ],
            cancel_table: make_empty_cancel_table(),
        };

        let mut strings = StringTable::new();
        let (mesh_keys, kf_keys) = build_asset_keys(&char_data, &mut strings).unwrap();

        assert_eq!(mesh_keys.len(), 2, "Duplicate animations should be deduplicated");
        assert_eq!(kf_keys.len(), 2);
    }

    #[test]
    fn test_pack_moves_empty() {
        let packed = pack_moves(&[], None, None).unwrap();
        assert_eq!(packed.moves.len(), 0);
        assert_eq!(packed.shapes.len(), 0);
        assert_eq!(packed.hit_windows.len(), 0);
        assert_eq!(packed.hurt_windows.len(), 0);
    }

    #[test]
    fn test_pack_moves_count_matches() {
        let moves = vec![
            make_test_move("5L", "stand_light"),
            make_test_move("5M", "stand_medium"),
        ];
        let packed = pack_moves(&moves, None, None).unwrap();

        let move_count = packed.moves.len() / STATE_RECORD_SIZE;
        assert_eq!(move_count, 2);
    }
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 6: Extract properties.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/properties.rs`

**Step 1: Create properties.rs with pack_character_props**

Extract from `zx_fspack.rs` lines 499-555:

```rust
//! Character properties packing.

use crate::codegen::fspk_format::{
    to_q24_8, write_u16_le, write_u32_le, write_u8, PROP_TYPE_BOOL, PROP_TYPE_Q24_8, PROP_TYPE_STR,
};
use crate::schema::Character;

use super::types::StringTable;
use super::utils::checked_u16;

/// Pack character properties into the CHARACTER_PROPS section.
///
/// Each property record is 12 bytes (CHARACTER_PROP12_SIZE):
/// - bytes 0-3: name offset (u32) into string pool
/// - bytes 4-5: name length (u16)
/// - bytes 6: value type (u8) - 0=Q24.8 number, 1=bool, 2=string ref
/// - byte 7: reserved/padding
/// - bytes 8-11: value (u32/i32 depending on type)
///
/// For string values, the value field contains (offset: u16, len: u16) packed into u32.
pub fn pack_character_props(
    character: &Character,
    strings: &mut StringTable,
) -> Result<Vec<u8>, String> {
    use crate::schema::PropertyValue;

    // CHARACTER_PROP12_SIZE = 12 bytes per property
    let mut data = Vec::with_capacity(character.properties.len() * 12);

    // BTreeMap iterates in sorted key order, ensuring deterministic output
    for (name, value) in &character.properties {
        // Write name reference (offset + length)
        let (name_off, name_len) = strings.intern(name)?;
        write_u32_le(&mut data, name_off);
        write_u16_le(&mut data, name_len);

        // Write type and value based on PropertyValue variant
        match value {
            PropertyValue::Number(n) => {
                write_u8(&mut data, PROP_TYPE_Q24_8);
                write_u8(&mut data, 0); // reserved
                // Convert f64 to Q24.8 fixed-point and write as i32
                let q24_8 = to_q24_8(*n);
                data.extend_from_slice(&q24_8.to_le_bytes());
            }
            PropertyValue::Bool(b) => {
                write_u8(&mut data, PROP_TYPE_BOOL);
                write_u8(&mut data, 0); // reserved
                // Write boolean as u32 (0 or 1)
                let val: u32 = if *b { 1 } else { 0 };
                write_u32_le(&mut data, val);
            }
            PropertyValue::String(s) => {
                write_u8(&mut data, PROP_TYPE_STR);
                write_u8(&mut data, 0); // reserved
                // Write string reference as u16 offset + u16 length pair
                let (str_off, str_len) = strings.intern(s)?;
                let str_off_u16 = checked_u16(str_off as usize, "string property value offset")?;
                write_u16_le(&mut data, str_off_u16);
                write_u16_le(&mut data, str_len);
            }
        }
    }

    Ok(data)
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 7: Extract sections.rs

**Files:**
- Create: `src-tauri/src/codegen/fspk/sections.rs`

**Step 1: Create sections.rs with section building helpers**

This file will contain helper functions for building the various optional sections (events, resources, tags, cancels). Extract the repetitive event arg packing logic into a reusable function.

```rust
//! Section building helpers for FSPK export.

use crate::codegen::fspk_format::{write_u16_le, write_u32_le, write_u8};
use crate::schema::{EventArgValue, EventEmit};

use super::types::{StrRef, StringTable};
use super::utils::{checked_u16, checked_u32, write_i64_le, write_range, write_strref, write_u64_le};

// Event argument type tags
pub const EVENT_ARG_TAG_BOOL: u8 = 0;
pub const EVENT_ARG_TAG_I64: u8 = 1;
pub const EVENT_ARG_TAG_F32: u8 = 2;
pub const EVENT_ARG_TAG_STRING: u8 = 3;

// Resource delta trigger types
pub const RESOURCE_DELTA_TRIGGER_ON_USE: u8 = 0;
pub const RESOURCE_DELTA_TRIGGER_ON_HIT: u8 = 1;
pub const RESOURCE_DELTA_TRIGGER_ON_BLOCK: u8 = 2;

/// Sentinel value for optional u16 fields
pub const OPT_U16_NONE: u16 = u16::MAX;

/// Pack event arguments into the event_args buffer.
///
/// Returns (args_off, args_len) for the packed arguments.
pub fn pack_event_args(
    args: &std::collections::BTreeMap<String, EventArgValue>,
    event_args_data: &mut Vec<u8>,
    strings: &mut StringTable,
) -> Result<(u32, u16), String> {
    let args_off = checked_u32(event_args_data.len(), "event_args_off")?;
    let args_len = checked_u16(args.len(), "event_args_len")?;

    for (k, v) in args {
        let key = strings.intern(k)?;
        write_strref(event_args_data, key);

        match v {
            EventArgValue::Bool(b) => {
                write_u8(event_args_data, EVENT_ARG_TAG_BOOL);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_u64_le(event_args_data, if *b { 1 } else { 0 });
            }
            EventArgValue::I64(i) => {
                write_u8(event_args_data, EVENT_ARG_TAG_I64);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_i64_le(event_args_data, *i);
            }
            EventArgValue::F32(f) => {
                write_u8(event_args_data, EVENT_ARG_TAG_F32);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                write_u64_le(event_args_data, f.to_bits() as u64);
            }
            EventArgValue::String(s) => {
                write_u8(event_args_data, EVENT_ARG_TAG_STRING);
                write_u8(event_args_data, 0);
                write_u16_le(event_args_data, 0);
                let vref = strings.intern(s)?;
                write_u32_le(event_args_data, vref.0);
                write_u16_le(event_args_data, vref.1);
                write_u16_le(event_args_data, 0);
            }
        }
    }

    Ok((args_off, args_len))
}

/// Pack event emits into the event_emits buffer.
///
/// Returns (emits_off, emits_len) for the packed emits.
pub fn pack_event_emits(
    events: &[EventEmit],
    event_emits_data: &mut Vec<u8>,
    event_args_data: &mut Vec<u8>,
    strings: &mut StringTable,
) -> Result<(u32, u16), String> {
    let emits_off = checked_u32(event_emits_data.len(), "event_emits_off")?;
    let emits_len = checked_u16(events.len(), "event_emits_len")?;

    for emit in events {
        let (args_off, args_len) = pack_event_args(&emit.args, event_args_data, strings)?;

        let id = strings.intern(&emit.id)?;
        write_strref(event_emits_data, id);
        write_range(event_emits_data, args_off, args_len);
    }

    Ok((emits_off, emits_len))
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 8: Extract export.rs (main orchestrator)

**Files:**
- Create: `src-tauri/src/codegen/fspk/export.rs`

**Step 1: Create export.rs with export_fspk function**

This is the main orchestrator - extract from `zx_fspack.rs` lines 594-1332. Due to the length, I'll provide the structure:

```rust
//! Main FSPK export function.

use std::collections::HashMap;

use crate::codegen::fspk_format::{
    write_u16_le, write_u32_le, write_u8, FLAGS_RESERVED, HEADER_SIZE, MAGIC,
    SECTION_CANCEL_DENIES, SECTION_CANCEL_TAG_RULES, SECTION_CHARACTER_PROPS,
    SECTION_EVENT_ARGS, SECTION_EVENT_EMITS, SECTION_HEADER_SIZE, SECTION_HIT_WINDOWS,
    SECTION_HURT_WINDOWS, SECTION_KEYFRAMES_KEYS, SECTION_MESH_KEYS, SECTION_MOVE_NOTIFIES,
    SECTION_MOVE_RESOURCE_COSTS, SECTION_MOVE_RESOURCE_DELTAS,
    SECTION_MOVE_RESOURCE_PRECONDITIONS, SECTION_PUSH_WINDOWS, SECTION_RESOURCE_DEFS,
    SECTION_SHAPES, SECTION_STATES, SECTION_STATE_EXTRAS, SECTION_STATE_TAGS,
    SECTION_STATE_TAG_RANGES, SECTION_STRING_TABLE, STATE_EXTRAS72_SIZE, STRREF_SIZE,
};
use crate::commands::CharacterData;

use super::moves::{build_asset_keys, pack_moves};
use super::properties::pack_character_props;
use super::sections::{
    pack_event_emits, OPT_U16_NONE, RESOURCE_DELTA_TRIGGER_ON_BLOCK,
    RESOURCE_DELTA_TRIGGER_ON_HIT, RESOURCE_DELTA_TRIGGER_ON_USE,
};
use super::types::{CancelLookup, StringTable};
use super::utils::{
    align_up, checked_u16, checked_u32, write_i32_le, write_range, write_section_header,
    write_strref,
};

/// Export character data to FSPK binary format.
///
/// Returns the packed binary data as a Vec<u8>.
#[allow(clippy::vec_init_then_push)] // Intentional: base sections first, optional sections conditionally added
pub fn export_fspk(char_data: &CharacterData) -> Result<Vec<u8>, String> {
    // [Full implementation extracted from zx_fspack.rs lines 598-1332]
    // The logic stays the same, just using the new module paths

    // ... (full implementation)
}

#[cfg(test)]
mod tests {
    // Integration tests for export_fspk
    // Extract from zx_fspack.rs lines 1622-2507
}
```

The full implementation would be copied from the original file with updated imports.

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 9: Rename zx_fspack_format.rs to fspk_format.rs

**Files:**
- Rename: `src-tauri/src/codegen/zx_fspack_format.rs` → `src-tauri/src/codegen/fspk_format.rs`

**Step 1: Rename the file**

Run: `git mv src-tauri/src/codegen/zx_fspack_format.rs src-tauri/src/codegen/fspk_format.rs`

**Step 2: Update module doc comment**

Change line 1-3 from:
```rust
//! ZX FSPK (Framesmith Pack) Binary Format Constants
//!
//! This module defines the binary format for exporting character data to the ZX runtime.
```

To:
```rust
//! FSPK (Framesmith Pack) Binary Format Constants
//!
//! This module defines the binary format for exporting character data.
```

**Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 10: Update codegen/mod.rs

**Files:**
- Modify: `src-tauri/src/codegen/mod.rs`

**Step 1: Update module declarations**

Change from:
```rust
mod json_blob;
mod zx_fspack;
pub mod zx_fspack_format;

pub use json_blob::{export_json_blob, export_json_blob_pretty};
pub use zx_fspack::export_zx_fspack;
```

To:
```rust
mod fspk;
mod json_blob;
pub mod fspk_format;

pub use fspk::export_fspk;
pub use json_blob::{export_json_blob, export_json_blob_pretty};
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 11: Update commands.rs

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Update import**

Change line 1 from:
```rust
use crate::codegen::{export_json_blob, export_json_blob_pretty, export_zx_fspack};
```

To:
```rust
use crate::codegen::{export_fspk, export_json_blob, export_json_blob_pretty};
```

**Step 2: Update adapter name in export_character (line 531)**

Change:
```rust
"zx-fspack" => {
    let bytes = export_zx_fspack(&char_data)?;
```

To:
```rust
"fspk" => {
    let bytes = export_fspk(&char_data)?;
```

**Step 3: Update get_character_fspk (line 623)**

Change:
```rust
let bytes = export_zx_fspack(&char_data)?;
```

To:
```rust
let bytes = export_fspk(&char_data)?;
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 12: Update frontend

**Files:**
- Modify: `src/lib/views/CharacterOverview.svelte`

**Step 1: Update adapter value (line 97)**

Change:
```svelte
if (exportAdapter === "zx-fspack") {
```

To:
```svelte
if (exportAdapter === "fspk") {
```

**Step 2: Update option value (line 195)**

Change:
```svelte
<option value="zx-fspack">ZX FSPK (Binary)</option>
```

To:
```svelte
<option value="fspk">FSPK (Binary)</option>
```

**Step 3: Update disabled check (line 201)**

Change:
```svelte
disabled={exportAdapter === "zx-fspack"}
```

To:
```svelte
disabled={exportAdapter === "fspk"}
```

**Step 4: Verify frontend compiles**

Run: `cd src-tauri && npm run check`

---

## Task 13: Delete old zx_fspack.rs

**Files:**
- Delete: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Remove the old file**

Run: `git rm src-tauri/src/codegen/zx_fspack.rs`

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`

---

## Task 14: Run Full Test Suite

**Step 1: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cd src-tauri && cargo clippy --all-targets`
Expected: No warnings

**Step 3: Run frontend checks**

Run: `npm run check`
Expected: No errors

---

## Task 15: Commit

**Step 1: Stage changes**

Run: `git add -A`

**Step 2: Commit**

```bash
git commit -m "$(cat <<'EOF'
refactor(codegen): rename zx_fspack to fspk and split into modules

- Rename zx_fspack.rs -> fspk/ subdirectory module
- Rename zx_fspack_format.rs -> fspk_format.rs
- Split 2,773-line monolith into 8 focused modules:
  - types.rs: StringTable, CancelLookup, PackedMoveData
  - utils.rs: Helper functions (checked_*, write_*)
  - packing.rs: Shape/hitbox/move record packing
  - moves.rs: pack_moves, build_asset_keys
  - properties.rs: pack_character_props
  - sections.rs: Event/resource section helpers
  - export.rs: Main export_fspk orchestrator
  - mod.rs: Public API
- Update frontend adapter name from "zx-fspack" to "fspk"

This improves LLM-friendliness by keeping each file under 400 lines
and removes the misleading "zx_" prefix since FSPK is engine-agnostic.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Verification

After completing all tasks:

1. `cargo clippy --all-targets` - No warnings
2. `cargo test` - All tests pass (including roundtrip tests with framesmith-fspack crate)
3. `npm run check` - No TypeScript/Svelte errors
4. Manual test: Open Framesmith, load a character, export as FSPK - should work
