# Cancel Condition Bitfield Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Convert cancel conditions from discrete enum to bitfield, enabling combinations like "hit+block but not whiff", and remove deprecated `chains` field.

**Architecture:** Replace `CancelCondition` enum (Always/Hit/Block/Whiff) with a `u8` bitfield where bit 0=hit, bit 1=block, bit 2=whiff. Remove `chains`, `special_cancels`, `super_cancels`, `jump_cancels` fields from `CancelTable`. All cancel logic flows through `tag_rules` with bitfield conditions.

**Tech Stack:** Rust (schema, codegen, runtime), TypeScript/Svelte (UI), JSON (data format)

---

## Overview

### Bitfield Definition

| Bit | Meaning | JSON syntax |
|-----|---------|-------------|
| 0 | on_hit | `"hit"` in array |
| 1 | on_block | `"block"` in array |
| 2 | on_whiff | `"whiff"` in array |

### Common Values

| Value | Binary | JSON | Meaning |
|-------|--------|------|---------|
| 7 | `0b111` | `"always"` or `["hit", "block", "whiff"]` | always |
| 3 | `0b011` | `["hit", "block"]` | hit + block (most common) |
| 1 | `0b001` | `["hit"]` or `"hit"` | hit only |
| 2 | `0b010` | `["block"]` or `"block"` | block only |
| 4 | `0b100` | `["whiff"]` or `"whiff"` | whiff only |
| 5 | `0b101` | `["hit", "whiff"]` | hit + whiff |
| 6 | `0b110` | `["block", "whiff"]` | block + whiff |

### JSON Format

The `on` field accepts either a string shorthand or an array:

```json
// Shorthand for common cases
{ "from": "normal", "to": "special", "on": "always" }
{ "from": "normal", "to": "special", "on": "hit" }

// Array for combinations
{ "from": "normal", "to": "special", "on": ["hit", "block"] }
{ "from": "normal", "to": "super", "on": ["hit"] }
```

### Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/schema/mod.rs` | Replace `CancelCondition` enum, remove `chains`/legacy fields from `CancelTable` |
| `src-tauri/src/codegen/zx_fspack.rs` | Update condition encoding, remove chain-related code |
| `crates/framesmith-fspack/src/view.rs` | Update condition decoding docs |
| `crates/framesmith-runtime/src/cancel.rs` | Update bitfield evaluation, remove chain cancel logic |
| `characters/test_char/cancel_table.json` | Update to new format |
| `src-tauri/tests/zx_fspack_roundtrip.rs` | Update tests |

---

## Task 1: Update Schema - CancelCondition to Bitfield

**Files:**
- Modify: `src-tauri/src/schema/mod.rs:372-443`

**Step 1.1: Replace CancelCondition enum with bitfield type**

Replace the existing `CancelCondition` enum (lines 372-381) with:

```rust
/// Bit flags for cancel conditions
pub mod cancel_flags {
    pub const HIT: u8 = 0b001;
    pub const BLOCK: u8 = 0b010;
    pub const WHIFF: u8 = 0b100;
    pub const ALWAYS: u8 = 0b111;
}

/// Cancel condition as a bitfield.
///
/// Serializes as either a string shorthand ("always", "hit", "block", "whiff")
/// or an array of conditions (["hit", "block"]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CancelCondition(pub u8);

impl CancelCondition {
    pub const ALWAYS: Self = Self(cancel_flags::ALWAYS);
    pub const HIT: Self = Self(cancel_flags::HIT);
    pub const BLOCK: Self = Self(cancel_flags::BLOCK);
    pub const WHIFF: Self = Self(cancel_flags::WHIFF);

    /// Check if this condition matches the given hit/block state.
    pub fn matches(&self, hit_confirmed: bool, block_confirmed: bool) -> bool {
        if hit_confirmed && (self.0 & cancel_flags::HIT != 0) {
            return true;
        }
        if block_confirmed && (self.0 & cancel_flags::BLOCK != 0) {
            return true;
        }
        if !hit_confirmed && !block_confirmed && (self.0 & cancel_flags::WHIFF != 0) {
            return true;
        }
        false
    }

    /// Convert to the binary format value (same as inner u8).
    pub fn to_binary(&self) -> u8 {
        self.0
    }
}

impl schemars::JsonSchema for CancelCondition {
    fn schema_name() -> String {
        "CancelCondition".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{Schema, SchemaObject, InstanceType, SingleOrVec};

        // Accept string or array of strings
        let mut schema = SchemaObject::default();
        schema.instance_type = Some(SingleOrVec::Vec(vec![
            InstanceType::String,
            InstanceType::Array,
        ]));
        Schema::Object(schema)
    }
}

impl Serialize for CancelCondition {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            cancel_flags::ALWAYS => serializer.serialize_str("always"),
            cancel_flags::HIT => serializer.serialize_str("hit"),
            cancel_flags::BLOCK => serializer.serialize_str("block"),
            cancel_flags::WHIFF => serializer.serialize_str("whiff"),
            bits => {
                // Serialize as array for combinations
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(None)?;
                if bits & cancel_flags::HIT != 0 {
                    seq.serialize_element("hit")?;
                }
                if bits & cancel_flags::BLOCK != 0 {
                    seq.serialize_element("block")?;
                }
                if bits & cancel_flags::WHIFF != 0 {
                    seq.serialize_element("whiff")?;
                }
                seq.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for CancelCondition {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, Visitor, SeqAccess};

        struct CancelConditionVisitor;

        impl<'de> Visitor<'de> for CancelConditionVisitor {
            type Value = CancelCondition;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string ('always', 'hit', 'block', 'whiff') or array of conditions")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "always" => Ok(CancelCondition(cancel_flags::ALWAYS)),
                    "hit" => Ok(CancelCondition(cancel_flags::HIT)),
                    "block" => Ok(CancelCondition(cancel_flags::BLOCK)),
                    "whiff" => Ok(CancelCondition(cancel_flags::WHIFF)),
                    _ => Err(de::Error::unknown_variant(v, &["always", "hit", "block", "whiff"])),
                }
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut bits: u8 = 0;
                while let Some(s) = seq.next_element::<&str>()? {
                    match s {
                        "hit" => bits |= cancel_flags::HIT,
                        "block" => bits |= cancel_flags::BLOCK,
                        "whiff" => bits |= cancel_flags::WHIFF,
                        _ => return Err(de::Error::unknown_variant(s, &["hit", "block", "whiff"])),
                    }
                }
                if bits == 0 {
                    return Err(de::Error::custom("cancel condition array cannot be empty"));
                }
                Ok(CancelCondition(bits))
            }
        }

        deserializer.deserialize_any(CancelConditionVisitor)
    }
}
```

**Step 1.2: Simplify CancelTable - remove deprecated fields**

Replace `CancelTable` struct (lines 424-443) with:

```rust
/// Cancel table defining all state relationships
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct CancelTable {
    /// Tag-based cancel rules (general patterns)
    #[serde(default)]
    pub tag_rules: Vec<CancelTagRule>,
    /// Explicit deny overrides
    #[serde(default)]
    pub deny: std::collections::HashMap<String, Vec<String>>,
}
```

**Step 1.3: Run cargo check**

Run: `cd src-tauri && cargo check`
Expected: Compilation errors in codegen (expected - we'll fix those next)

**Step 1.4: Commit schema changes**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): convert CancelCondition to bitfield, remove chains

- CancelCondition now uses u8 bitfield (hit=0b001, block=0b010, whiff=0b100)
- Supports JSON as string shorthand or array: \"hit\", [\"hit\", \"block\"]
- Remove chains, special_cancels, super_cancels, jump_cancels from CancelTable
- All cancel logic now flows through tag_rules

BREAKING CHANGE: chains field removed from cancel_table.json

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Update FSPK Codegen

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs:325-358, 445-465, 680-720, 1167-1172`

**Step 2.1: Remove CancelLookup chain-related fields**

Find `CancelLookup` struct (~line 327) and remove chain-related fields. Replace with:

```rust
/// Precomputed lookup tables for cancel encoding.
///
/// Contains HashSets for each cancel type, keyed by move input notation.
pub struct CancelLookup<'a> {
    /// Moves that can cancel into specials (from legacy special_cancels, kept for flag compat)
    pub specials: std::collections::HashSet<&'a str>,
    /// Moves that can cancel into supers
    pub supers: std::collections::HashSet<&'a str>,
    /// Moves that can jump cancel
    pub jumps: std::collections::HashSet<&'a str>,
    /// Map from input notation to move index
    pub input_to_index: std::collections::HashMap<&'a str, u16>,
}
```

**Step 2.2: Remove PackedMoveData cancel fields**

Find `PackedMoveData` struct and remove:
- `cancels: Vec<u8>`
- `cancel_info: Vec<(u32, u16)>`

**Step 2.3: Remove chain cancel flag logic**

Find the move packing loop that sets `CANCEL_FLAG_CHAIN` (~line 448) and remove:

```rust
// DELETE this block:
if lookup.chains.contains(input) {
    flags |= super::zx_fspack_format::CANCEL_FLAG_CHAIN;
}
```

Also remove the chain cancel routes packing (~lines 461-475):

```rust
// DELETE this block:
// Pack chain cancel routes into CANCELS_U16 section
if let Some(targets) = lookup.chain_routes.get(input) {
    // ... entire block
}
```

**Step 2.4: Update CancelLookup construction**

Find where `CancelLookup` is constructed (~line 680) and simplify:

```rust
// Build cancel lookup from cancel_table
let cancel_lookup = CancelLookup {
    specials: std::collections::HashSet::new(), // No longer used
    supers: std::collections::HashSet::new(),
    jumps: std::collections::HashSet::new(),
    input_to_index: {
        let mut map = std::collections::HashMap::new();
        for (i, mv) in char_data.states.iter().enumerate() {
            map.insert(mv.input.as_str(), i as u16);
        }
        map
    },
};
```

**Step 2.5: Update condition encoding**

Find the condition encoding (~line 1167) and replace:

```rust
// condition (1 byte) - now a bitfield
let condition: u8 = rule.on.to_binary();
write_u8(&mut cancel_tag_rules_data, condition);
```

**Step 2.6: Remove CANCELS_U16 section emission**

Find where `SECTION_CANCELS_U16` is added to sections (~line 1252) and remove it, since chains no longer exist:

```rust
// DELETE:
sections.push(SectionData {
    kind: SECTION_CANCELS_U16,
    align: 2,
    bytes: packed.cancels,
});
```

**Step 2.7: Run cargo check**

Run: `cd src-tauri && cargo check`
Expected: May have errors in tests (expected - we'll fix those in Task 4)

**Step 2.8: Commit codegen changes**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat(codegen): update FSPK export for bitfield conditions

- Remove chain cancel encoding (CANCELS_U16 section no longer emitted)
- CancelCondition now written as raw bitfield u8
- Simplify CancelLookup to only track input->index mapping

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Update Runtime Cancel Evaluation

**Files:**
- Modify: `crates/framesmith-runtime/src/cancel.rs:38-130`

**Step 3.1: Remove chain cancel checking**

Delete the entire chain cancels block (lines 58-76):

```rust
// DELETE this entire block:
// 2. Check explicit chain cancels from move extras (rekkas, target combos)
if let Some(extras) = pack.state_extras() {
    // ... entire block
}
```

**Step 3.2: Update condition evaluation to use bitfield**

Replace the condition matching block (~lines 99-109) with:

```rust
// Check condition bitfield
// bit 0 = hit, bit 1 = block, bit 2 = whiff
let condition = rule.condition();
let condition_met = if state.hit_confirmed {
    condition & 0b001 != 0  // HIT bit
} else if state.block_confirmed {
    condition & 0b010 != 0  // BLOCK bit
} else {
    condition & 0b100 != 0  // WHIFF bit
};
if !condition_met {
    continue;
}
```

**Step 3.3: Update function doc comment**

Update the doc comment for `can_cancel_to` to reflect the new priority:

```rust
/// Check if a cancel from current state to target move is valid.
///
/// This checks (in priority order):
/// 1. Explicit denies - block specific cancels
/// 2. Tag-based rules (patterns like "normal->special on hit+block")
///
/// Resource preconditions are checked for tag rules.
```

**Step 3.4: Remove available_cancels functions or simplify**

The `available_cancels` and `available_cancels_buf` functions rely on the chain system. Either:
- Remove them entirely, OR
- Reimplement to iterate tag_rules (more complex)

For now, remove them since they're behind `#[cfg(feature = "alloc")]` and can be re-added later if needed.

**Step 3.5: Run cargo check in runtime crate**

Run: `cd crates/framesmith-runtime && cargo check`
Expected: PASS

**Step 3.6: Commit runtime changes**

```bash
git add crates/framesmith-runtime/src/cancel.rs
git commit -m "feat(runtime): update cancel evaluation for bitfield conditions

- Remove chain cancel checking (chains deprecated)
- Condition now evaluated as bitfield: hit=0b001, block=0b010, whiff=0b100
- Simplify to: deny check -> tag rules only

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Update Test Data

**Files:**
- Modify: `characters/test_char/cancel_table.json`

**Step 4.1: Rewrite cancel_table.json with new format**

Replace the entire file with:

```json
{
  "tag_rules": [
    { "from": "system", "to": "any", "on": "always" },
    { "from": "movement", "to": "any", "on": "always" },
    { "from": "normal", "to": "special", "on": ["hit", "block"] },
    { "from": "normal", "to": "super", "on": ["hit", "block"] },
    { "from": "special", "to": "super", "on": ["hit", "block"] },

    { "from": "5L", "to": "5L", "on": ["hit", "block"] },
    { "from": "5L", "to": "5M", "on": ["hit", "block"] },
    { "from": "5L", "to": "2L", "on": ["hit", "block"] },
    { "from": "5M", "to": "5H", "on": ["hit", "block"] },
    { "from": "2L", "to": "2L", "on": ["hit", "block"] },
    { "from": "2L", "to": "5M", "on": ["hit", "block"] },
    { "from": "j.L", "to": "j.L", "on": ["hit", "block"] },

    { "from": "236K", "to": "236K~K", "on": ["hit", "block"] },
    { "from": "236K~K", "to": "236K~K~K", "on": ["hit", "block"] }
  ],
  "deny": {}
}
```

**Step 4.2: Add self-referential tags to states that need them**

For each state that appears in `from` or `to` as a specific name (not a category tag), add a matching tag to that state's JSON.

Modify `characters/test_char/states/5L.json` - add `"5L"` to tags:
```json
"tags": ["normal", "starter", "poke", "5L"],
```

Modify `characters/test_char/states/5M.json` - add tag:
```json
"tags": ["normal", "5M"],
```

Modify `characters/test_char/states/5H.json` - add tag:
```json
"tags": ["normal", "5H"],
```

Modify `characters/test_char/states/2L.json` - add tag:
```json
"tags": ["normal", "2L"],
```

Modify `characters/test_char/states/j.L.json` - add tag:
```json
"tags": ["normal", "aerial", "j.L"],
```

Modify `characters/test_char/states/236K.json` - add tag:
```json
"tags": ["special", "236K"],
```

Modify `characters/test_char/states/236K~K.json` - add tag:
```json
"tags": ["special", "236K~K"],
```

Modify `characters/test_char/states/236K~K~K.json` - add tag:
```json
"tags": ["special", "236K~K~K"],
```

**Step 4.3: Commit test data changes**

```bash
git add characters/test_char/
git commit -m "chore(test_char): update to bitfield cancel format

- Convert cancel_table.json to use tag_rules only
- Add self-referential tags to states for explicit cancel routes
- Use [\"hit\", \"block\"] for combo-able cancels

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Update/Fix Tests

**Files:**
- Modify: `src-tauri/tests/zx_fspack_roundtrip.rs`

**Step 5.1: Remove chain-related tests**

Find and remove these tests:
- `test_cancel_flags_exported`
- `test_chain_cancel_routes_exported`
- `test_move_extras_cancel_offsets`

Or rewrite them to test the new tag_rules-only behavior.

**Step 5.2: Add new condition bitfield test**

Add a test for the new bitfield serialization:

```rust
#[test]
fn test_cancel_condition_bitfield_roundtrip() {
    use crate::schema::{CancelCondition, CancelTagRule, CancelTable};

    // Test string shorthand
    let json = r#"{"from": "normal", "to": "special", "on": "hit"}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, 0b001);

    // Test array format
    let json = r#"{"from": "normal", "to": "special", "on": ["hit", "block"]}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, 0b011);

    // Test "always" shorthand
    let json = r#"{"from": "any", "to": "any", "on": "always"}"#;
    let rule: CancelTagRule = serde_json::from_str(json).unwrap();
    assert_eq!(rule.on.0, 0b111);

    // Test roundtrip serialization
    let table = CancelTable {
        tag_rules: vec![
            CancelTagRule {
                from: "normal".to_string(),
                to: "special".to_string(),
                on: CancelCondition(0b011), // hit + block
                after_frame: 0,
                before_frame: 255,
            },
        ],
        deny: Default::default(),
    };

    let json = serde_json::to_string(&table).unwrap();
    let parsed: CancelTable = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.tag_rules[0].on.0, 0b011);
}
```

**Step 5.3: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: PASS

**Step 5.4: Commit test updates**

```bash
git add src-tauri/tests/
git commit -m "test: update tests for bitfield cancel conditions

- Remove chain-related tests (chains deprecated)
- Add test_cancel_condition_bitfield_roundtrip

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Update FSPK View Documentation

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs:1996-2005`

**Step 6.1: Update CancelTagRuleView docs**

Update the doc comment for `CancelTagRuleView::condition()`:

```rust
/// Get the condition bitfield.
///
/// Bits: 0=hit, 1=block, 2=whiff
/// Common values: 7=always, 3=hit+block, 1=hit, 2=block, 4=whiff
pub fn condition(&self) -> u8 {
    read_u8(self.data, 16).unwrap_or(0)
}
```

**Step 6.2: Run cargo check**

Run: `cd crates/framesmith-fspack && cargo check`
Expected: PASS

**Step 6.3: Commit docs update**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "docs(fspack): document condition bitfield format

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 7: Final Verification

**Step 7.1: Run full test suite**

```bash
cd src-tauri && cargo test
cd crates/framesmith-runtime && cargo test
cd crates/framesmith-fspack && cargo test
```

Expected: All tests PASS

**Step 7.2: Run clippy**

```bash
cd src-tauri && cargo clippy --all-targets
```

Expected: No warnings

**Step 7.3: Test export with test_char**

Run the Tauri dev server and verify test_char exports correctly:

```bash
npm run tauri dev
```

Then in the app, open test_char and export to FSPK. Verify no errors.

**Step 7.4: Final commit if any fixups needed**

```bash
git add -A
git commit -m "chore: final cleanup for cancel bitfield migration

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Schema: CancelCondition bitfield | Medium |
| 2 | Codegen: Remove chains, update encoding | Medium |
| 3 | Runtime: Bitfield evaluation | Low |
| 4 | Test data: Update cancel_table.json | Low |
| 5 | Tests: Update/remove chain tests | Medium |
| 6 | Docs: Update FSPK view docs | Low |
| 7 | Final verification | Low |
