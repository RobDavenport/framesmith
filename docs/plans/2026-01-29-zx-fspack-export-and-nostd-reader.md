# ZX FSPK Export + no_std Reader Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a new Framesmith export adapter that emits a compact, zero-copy-friendly `FSPK` binary pack for Nethercore ZX, and ship a `no_std` Rust reader crate that ZX games compile into their `.wasm` to parse the pack and resolve stable asset-key IDs to runtime handles.

**Architecture:** Export produces a stable, sectioned `FSPK` container (little-endian) with packed gameplay records (IDs + offset/len into backing arrays) and an asset-key string table for init-time `rom_*` handle creation. Runtime crate parses `FSPK` with bounds checks (unaligned-safe) and exposes borrow-only “views” over the underlying bytes.

**Tech Stack:** Rust (Tauri backend exporter), `no_std` Rust crate (`alloc` optional), existing Framesmith schema (`src-tauri/src/schema`), existing export pipeline (`src-tauri/src/commands.rs`, `src-tauri/src/codegen`).

---

## Non-Goals (YAGNI)

- No UI changes required (export can be called via existing `export_character` command).
- No compression, encryption, or streaming/paging.
- No animation/mesh baking; pack stores asset key strings only.
- No engine runtime integration code beyond the `no_std` reader crate.

## Constraints / Assumptions

- Nethercore ZX requires GPU assets to be loaded during `init()`, but `rom_data` can be called anytime.
- Roster is fixed at compile time; game loads all packs/assets during init.
- Coordinate encoding uses Q12.4 fixed-point (1/16 px) with `i16` for x/y and `u16` for sizes.
- Frame ranges per move are typical, so `u8` frames are sufficient.

## `FSPK` Format (v1)

### Container header

All values little-endian.

- `magic[4] = "FSPK"`
- `version: u16 = 1`
- `flags: u16 = 0`
- `total_len: u32`
- `section_count: u32`
- `sections[section_count]`: `SectionHeader` (fixed 16 bytes)

```rust
// on-disk
struct SectionHeader {
  kind: u32,
  off: u32,
  len: u32,
  align: u32,
}
```

### Section kinds

Reserve numeric IDs (stable, do not reuse):

- `1 = STRING_TABLE`
- `2 = MESH_KEYS`
- `3 = KEYFRAMES_KEYS`
- `4 = MOVES`
- `5 = HIT_WINDOWS`
- `6 = HURT_WINDOWS`
- `7 = SHAPES`
- `8 = CANCELS_U16`

### String table

`STRING_TABLE` is a raw UTF-8 byte blob. Strings are referenced by `(off: u32, len: u16)`.

### Asset key sections

Asset keys are stable IDs into key tables; at runtime they map to host handles.

```rust
// on-disk
struct StrRef {
  off: u32,
  len: u16,
  _pad: u16,
}
```

- `MESH_KEYS`: `StrRef[]` where the string is the ROM key passed to `rom_mesh`.
- `KEYFRAMES_KEYS`: `StrRef[]` where the string is the ROM key passed to `rom_keyframes`.

### Shape encoding (fixed 12 bytes)

```rust
// on-disk
struct Shape12 {
  kind: u8,   // 0=aabb, 1=rect, 2=circle, 3=capsule
  flags: u8,  // reserved
  a: i16,
  b: i16,
  c: i16,
  d: i16,
  e: i16,
}
```

All numeric fields are Q12.4 except where noted.

Interpretation:

- aabb: x=a, y=b, w=u16(c), h=u16(d)
- rect: x=a, y=b, w=u16(c), h=u16(d), angle=e (Q8.8 degrees)
- circle: x=a, y=b, r=u16(c)
- capsule: x1=a, y1=b, x2=c, y2=d, r=u16(e)

### Hit window encoding (24 bytes)

```rust
// on-disk
struct HitWindow24 {
  start_f: u8,
  end_f: u8,
  guard: u8,
  _r0: u8,
  dmg: u16,
  chip: u16, // 0 = none
  hitstun: u8,
  blockstun: u8,
  hitstop: u8,
  _r1: u8,
  shapes_off: u32,
  shapes_len: u16,
  cancels_off: u32,  // offset into CANCELS_U16
  cancels_len: u16,
}
```

### Hurt window encoding (12 bytes)

```rust
// on-disk
struct HurtWindow12 {
  start_f: u8,
  end_f: u8,
  hurt_flags: u16,
  shapes_off: u32,
  shapes_len: u16,
  _r0: u16,
}
```

### Move record (v1)

We keep a fixed-size record with offsets into window arrays.

```rust
// on-disk
struct MoveRecord {
  move_id: u16,          // index
  mesh_key: u16,         // MeshKeyId
  keyframes_key: u16,    // KeyframesKeyId
  move_type: u8,
  trigger: u8,
  guard: u8,
  flags: u8,
  startup: u8,
  active: u8,
  recovery: u8,
  _r0: u8,
  total: u16,
  damage: u16,
  hitstun: u8,
  blockstun: u8,
  hitstop: u8,
  _r1: u8,
  hit_windows_off: u32,
  hit_windows_len: u16,
  hurt_windows_off: u32,
  hurt_windows_len: u16,
}
```

Notes:

- `move_id` is the index in the `MOVES` array (redundant but helps validation).
- `mesh_key` and `keyframes_key` may be 0xFFFF for “none” (define sentinel).

---

# Implementation Tasks

### Task 1: Write format doc + constants

**Files:**
- Create: `src-tauri/src/codegen/zx_fspack_format.rs`

**Step 1: Add on-disk structs as docs + constants**

Write Rust constants for section kinds, version, sentinels, and helper functions for little-endian writes.

**Step 2: Add a small unit test for header/section sizing**

Add a `#[cfg(test)]` test ensuring the “on-disk sizes” are as expected (use manual `const` sizes, not `repr(C)` layout).

**Step 3: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack_format.rs
git commit -m "feat: define zx fspack v1 format"
```

### Task 2: Create the `no_std` reader crate skeleton

**Files:**
- Create: `crates/framesmith-fspack/Cargo.toml`
- Create: `crates/framesmith-fspack/src/lib.rs`

**Step 1: Create crate with `#![no_std]`**

`lib.rs` exports:

```rust
#![no_std]

pub mod error;
pub mod view;
pub mod bytes;
```

Add `alloc` feature gate only if needed; default should be allocation-free.

**Step 2: Add minimal parse API surface**

```rust
pub struct PackView<'a> { /* ... */ }

impl<'a> PackView<'a> {
  pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> { /* stub */ }
}
```

**Step 3: Add crate unit test (std-only)**

Use `#[cfg(test)] extern crate std;` and test that `parse(&[])` fails with a stable error.

**Step 4: Run tests**

Run: `cargo test -p framesmith-fspack` (or workspace equivalent).
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/framesmith-fspack
git commit -m "feat: add no_std framesmith fspack reader crate"
```

### Task 3: Implement bounds-checked container parsing in reader crate

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`
- Create: `crates/framesmith-fspack/src/error.rs`
- Create: `crates/framesmith-fspack/src/bytes.rs`

**Step 1: Write failing tests for header parsing**

Create tests for:

- wrong magic
- unsupported version
- section table out of bounds
- overlapping / out-of-bounds section ranges

**Step 2: Implement little-endian readers**

Implement `read_u16_le`, `read_u32_le`, etc with bounds checks.

**Step 3: Implement `PackView::parse`**

Parse header + sections, expose `get_section(kind) -> Option<&[u8]>`.

**Step 4: Run tests**

Run: `cargo test -p framesmith-fspack`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/framesmith-fspack/src
git commit -m "feat: parse fspk container with bounds checks"
```

### Task 4: Expose typed views (string table, keys, moves)

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Write failing tests with a tiny hand-built pack**

Construct a minimal in-memory `FSPK` buffer in the test (manual bytes), containing:

- STRING_TABLE: `"glitch.5h"`
- MESH_KEYS: one StrRef
- MOVES: one MoveRecord referencing that mesh key

Test:

- `pack.string(mesh_key)` returns correct `&str`
- move record fields are readable

**Step 2: Implement typed accessors**

- `PackView::string_table()`
- `PackView::mesh_keys()` returns `MeshKeysView`
- `PackView::moves()` returns `MovesView`
- `MovesView::get(MoveId)`

All must be unaligned-safe (read via byte slices, not `transmute`).

**Step 3: Run tests**

Run: `cargo test -p framesmith-fspack`
Expected: PASS.

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat: add typed views for fspack keys and moves"
```

### Task 5: Add exporter adapter module stub

**Files:**
- Create: `src-tauri/src/codegen/zx_fspack.rs`
- Modify: `src-tauri/src/codegen/mod.rs`

**Step 1: Add stub export function**

```rust
pub fn export_zx_fspack(_char_data: &crate::commands::CharacterData) -> Result<Vec<u8>, String> {
  Err("zx-fspack not implemented".to_string())
}
```

**Step 2: Wire module export in `codegen/mod.rs`**

**Step 3: Add unit test that function is callable**

Optional: a smoke test in `src-tauri/src/codegen/zx_fspack.rs`.

**Step 4: Run Rust tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/codegen
git commit -m "feat: add zx-fspack exporter stub"
```

### Task 6: Implement exporter: string table + keys

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Write failing exporter test for deterministic string table**

Create a minimal `CharacterData` (or load `characters/glitch`) and assert:

- string table includes move animation names used as keyframes keys
- keys are deduped and deterministic (sorted)

**Step 2: Implement key extraction**

For v1:

- Treat `Move.animation` as `KeyframesKey`
- Treat `Move.input` as `MeshKey` via a deterministic naming convention: `{character.id}.{move.input}` (or `{character.id}.{move.animation}` if preferred)

NOTE: If naming convention is not final, implement both behind a small function to change later.

**Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat: build fspack string table and asset key tables"
```

### Task 7: Implement exporter: pack moves and backing arrays

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Write failing exporter test for packed move record counts**

Assert:

- move count matches input moves
- `MOVES` section length is `count * MoveRecordSize`
- references (offset/len) are within backing sections

**Step 2: Implement packing helpers**

- Quantize rect hitboxes/hurtboxes into Shape12 using Q12.4
- For now, convert legacy rect hitboxes/hurtboxes only (ignore v2 shaped hitboxes until later)
- Build:
  - `SHAPES` array
  - `HIT_WINDOWS` from `Move.hitboxes` (each rect frame range becomes one `HitWindow24` with one shape)
  - `HURT_WINDOWS` from `Move.hurtboxes` (each rect frame range becomes one `HurtWindow12` with one shape)
  - `CANCELS_U16` from cancel table (optional v1: store move-level cancels empty; defer per-hit cancels)

**Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat: pack moves hit/hurt windows and shapes for zx fspack"
```

### Task 8: Wire exporter into `export_character` command

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Add adapter match arm**

- Add `"zx-fspack" => export_zx_fspack(&char_data)`
- For this adapter, write bytes to `output_path` (binary) instead of string.

Implementation approach:

- Refactor `export_character` to support both `String` and `Vec<u8>` outputs (e.g. an enum).

**Step 2: Add integration test**

Prefer a unit test that calls the exporter directly and validates header/sections using the reader crate.

**Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat: add zx-fspack adapter to export_character"
```

### Task 9: Cross-check exporter output with reader crate

**Files:**
- Add: `src-tauri/tests/zx_fspack_roundtrip.rs` (or module tests)

**Step 1: Write failing roundtrip test**

- Load `characters/glitch`
- Export to bytes via exporter
- Parse via `framesmith_fspack::PackView::parse`
- Assert:
  - move count
  - at least one keyframes key exists if animations are present

**Step 2: Implement anything needed to pass**

Fix section alignments, offsets, sizes.

**Step 3: Run tests**

Run: `cd src-tauri && cargo test`
Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/tests/zx_fspack_roundtrip.rs
git commit -m "test: roundtrip zx fspack export through no_std reader"
```

### Task 10: Document how ZX games use it

**Files:**
- Modify: `docs/design.md`
- Create: `docs/README.md` section or `docs/zx-fspack.md`

**Step 1: Add usage snippet (init-time handle resolution)**

Include pseudo-code:

- `rom_data_len` + allocate buffer
- `rom_data` into buffer
- `PackView::parse(&buffer)`
- iterate mesh/keyframes keys and call `rom_mesh` / `rom_keyframes`
- store handles in arrays indexed by key IDs

**Step 2: Commit**

```bash
git add docs
git commit -m "docs: describe zx fspack export and runtime handle mapping"
```

---

## Verification Checklist (before merging)

- `cd src-tauri && cargo test` passes
- `cargo test -p framesmith-fspack` passes
- Export `zx-fspack` for `characters/glitch` produces a non-empty file
- Reader crate parses that file and can resolve at least one asset key string

## Follow-ups (separate plans)

- Pack cancel routes (`cancel_table.json`) into `CANCELS_U16` and expose `MoveView::cancels()`.
- Add v2 support: pack `hits` and `advanced_hurtboxes` with shaped hitboxes.
- Add optional compression per section if needed.
