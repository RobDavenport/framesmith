# Move → State + Tags + Cancel Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor framesmith from "Move" to "State" terminology, add flexible tag-based categorization, and implement tag-based cancel rules.

**Architecture:** States are the fundamental unit (attacks, reactions, neutral, system). Tags provide flexible categorization. Cancel rules use both tag patterns (normal→special) and explicit routes (rekkas, target combos). Runtime stays minimal; tags evaluated at cancel-check time.

**Tech Stack:** Rust (schema, runtime, fspack), TypeScript/Svelte (frontend), FSPK binary format

**Reference:** Design document at `C:\Users\rdave\.claude\plans\snug-churning-hummingbird.md`

---

## Phase 1: Schema + Tag Newtype

### Task 1.1: Add Tag Newtype and Error Type

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Write the failing test for Tag validation**

Add at the bottom of the file, before the closing of the module:

```rust
#[cfg(test)]
mod tag_tests {
    use super::*;

    #[test]
    fn tag_valid_lowercase() {
        let tag = Tag::new("normal").unwrap();
        assert_eq!(tag.as_str(), "normal");
    }

    #[test]
    fn tag_valid_with_underscore() {
        let tag = Tag::new("on_hit").unwrap();
        assert_eq!(tag.as_str(), "on_hit");
    }

    #[test]
    fn tag_valid_with_numbers() {
        let tag = Tag::new("rekka1").unwrap();
        assert_eq!(tag.as_str(), "rekka1");
    }

    #[test]
    fn tag_rejects_empty() {
        assert!(Tag::new("").is_err());
    }

    #[test]
    fn tag_rejects_uppercase() {
        assert!(Tag::new("Normal").is_err());
    }

    #[test]
    fn tag_rejects_spaces() {
        assert!(Tag::new("on hit").is_err());
    }

    #[test]
    fn tag_rejects_special_chars() {
        assert!(Tag::new("normal!").is_err());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test tag_tests`
Expected: FAIL - `Tag` type not found

**Step 3: Implement Tag newtype**

Add after the imports at the top of `src-tauri/src/schema/mod.rs`:

```rust
/// Error type for invalid tags
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    Empty,
    InvalidChars,
}

impl std::fmt::Display for TagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagError::Empty => write!(f, "tag cannot be empty"),
            TagError::InvalidChars => write!(f, "tag must be lowercase alphanumeric with underscores"),
        }
    }
}

impl std::error::Error for TagError {}

/// Validated tag for state categorization.
///
/// Tags are lowercase alphanumeric strings with underscores.
/// Games use tags for cancel rules and filtering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag, validating the format.
    pub fn new(s: impl Into<String>) -> Result<Self, TagError> {
        let s = s.into();
        if s.is_empty() {
            return Err(TagError::Empty);
        }
        if !s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(TagError::InvalidChars);
        }
        Ok(Tag(s))
    }

    /// Get the tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Tag {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Tag::new(s).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Tag {
    fn schema_name() -> String {
        "Tag".to_string()
    }

    fn json_schema(gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = gen.subschema_for::<String>();
        if let schemars::Schema::Object(ref mut obj) = schema {
            obj.metadata().description = Some("Lowercase alphanumeric tag with underscores".to_string());
        }
        schema
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test tag_tests`
Expected: PASS (all 7 tests)

**Step 5: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): add Tag newtype with validation

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 1.2: Add tags field to Move struct

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Write the failing test**

Add to the existing tests section:

```rust
#[test]
fn move_with_tags_deserializes() {
    let json = r#"{
      "input": "5L",
      "name": "Light",
      "tags": ["normal", "light"],
      "startup": 5,
      "active": 2,
      "recovery": 10,
      "damage": 500,
      "hitstun": 15,
      "blockstun": 10,
      "hitstop": 10,
      "guard": "mid",
      "hitboxes": [],
      "hurtboxes": [],
      "pushback": { "hit": 5, "block": 8 },
      "meter_gain": { "hit": 100, "whiff": 20 },
      "animation": "5L"
    }"#;

    let mv: Move = serde_json::from_str(json).expect("move should parse");
    assert_eq!(mv.tags.len(), 2);
    assert_eq!(mv.tags[0].as_str(), "normal");
    assert_eq!(mv.tags[1].as_str(), "light");
}

#[test]
fn move_without_tags_deserializes_empty() {
    let json = r#"{
      "input": "5L",
      "name": "Light",
      "startup": 5,
      "active": 2,
      "recovery": 10,
      "damage": 500,
      "hitstun": 15,
      "blockstun": 10,
      "hitstop": 10,
      "guard": "mid",
      "hitboxes": [],
      "hurtboxes": [],
      "pushback": { "hit": 5, "block": 8 },
      "meter_gain": { "hit": 100, "whiff": 20 },
      "animation": "5L"
    }"#;

    let mv: Move = serde_json::from_str(json).expect("move should parse");
    assert!(mv.tags.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test move_with_tags`
Expected: FAIL - no field `tags` on type `Move`

**Step 3: Add tags field to Move struct**

In the `Move` struct definition, add after the `name` field:

```rust
#[serde(default)]
pub tags: Vec<Tag>,
```

And in `impl Default for Move`, add:

```rust
tags: Vec::new(),
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test move_with_tags && cargo test move_without_tags`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): add tags field to Move struct

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 1.3: Update frontend types.ts

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: Add Tag type and update Move interface**

Find the `Move` interface and add:

```typescript
// Add near the top with other type definitions
export type Tag = string; // Validated on backend, just string on frontend

// In the Move interface, add after 'name':
export interface Move {
  input: string;
  name: string;
  tags: Tag[];  // ADD THIS LINE
  // ... rest of existing fields
}
```

**Step 2: Verify TypeScript compiles**

Run: `cd src && npx tsc --noEmit`
Expected: No errors (or only pre-existing errors unrelated to tags)

**Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(frontend): add tags field to Move type

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 1.4: Verify existing moves load with empty tags

**Step 1: Run the full test suite**

Run: `cd src-tauri && cargo test`
Expected: All existing tests pass

**Step 2: Manual verification**

Run: `npm run tauri dev`
- Open an existing project
- Verify moves load without errors
- Check browser console for any deserialization errors

**Step 3: Commit checkpoint**

```bash
git add -A
git commit -m "checkpoint: phase 1 complete - tags field added

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: FSPK Tag Support

### Task 2.1: Add section constants for tags

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add section constants**

Find the section constant definitions and add:

```rust
/// Section containing tag range pointers (parallel to STATES)
pub const SECTION_STATE_TAG_RANGES: u32 = 17;
/// Section containing tag StrRefs
pub const SECTION_STATE_TAGS: u32 = 18;
```

Also add size constant:

```rust
/// Size of a state tag range entry (off: u32, count: u16, pad: u16)
pub const STATE_TAG_RANGE_SIZE: usize = 8;
```

**Step 2: Verify it compiles**

Run: `cd crates/framesmith-fspack && cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add section constants for state tags

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 2.2: Implement StateTagRangesView reader

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod state_tag_tests {
    use super::*;

    #[test]
    fn state_tag_range_view_returns_none_for_missing_section() {
        // Minimal valid pack with no tag sections
        let data = build_minimal_pack_without_tags();
        let pack = PackView::parse(&data).unwrap();
        assert!(pack.state_tag_ranges().is_none());
    }
}
```

**Step 2: Implement StateTagRangesView**

```rust
/// View into STATE_TAG_RANGES section
pub struct StateTagRangesView<'a> {
    data: &'a [u8],
}

impl<'a> StateTagRangesView<'a> {
    /// Get the tag range (offset, count) for a state by index
    pub fn get(&self, index: usize) -> Option<(u32, u16)> {
        let offset = index * STATE_TAG_RANGE_SIZE;
        if offset + STATE_TAG_RANGE_SIZE > self.data.len() {
            return None;
        }
        let slice = &self.data[offset..offset + STATE_TAG_RANGE_SIZE];
        let off = read_u32(slice, 0);
        let count = read_u16(slice, 4);
        Some((off, count))
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.data.len() / STATE_TAG_RANGE_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a> PackView<'a> {
    /// Get the state tag ranges section
    pub fn state_tag_ranges(&self) -> Option<StateTagRangesView<'a>> {
        self.section(SECTION_STATE_TAG_RANGES)
            .map(|data| StateTagRangesView { data })
    }

    /// Get tags for a state by index
    pub fn state_tags(&self, state_idx: usize) -> Option<impl Iterator<Item = &'a str>> {
        let ranges = self.state_tag_ranges()?;
        let (off, count) = ranges.get(state_idx)?;
        let tags_section = self.section(SECTION_STATE_TAGS)?;

        Some((0..count).filter_map(move |i| {
            let tag_offset = (off as usize) + (i as usize) * 8; // StrRef is 8 bytes
            if tag_offset + 8 > tags_section.len() {
                return None;
            }
            let str_off = read_u32(tags_section, tag_offset);
            let str_len = read_u16(tags_section, tag_offset + 4);
            self.string(str_off, str_len)
        }))
    }
}
```

**Step 3: Run test to verify**

Run: `cd crates/framesmith-fspack && cargo test state_tag`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add StateTagRangesView reader

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 2.3: Update zx_fspack.rs encoder to write tags

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Add tag encoding to pack generation**

Find the pack generation function and add after moves are written:

```rust
// Write STATE_TAG_RANGES section (parallel to moves)
let mut tag_ranges: Vec<u8> = Vec::new();
let mut tag_strrefs: Vec<u8> = Vec::new();

for mv in &char_data.moves {
    let tag_offset = tag_strrefs.len() as u32;
    let tag_count = mv.tags.len() as u16;

    // Write range entry
    tag_ranges.extend_from_slice(&tag_offset.to_le_bytes());
    tag_ranges.extend_from_slice(&tag_count.to_le_bytes());
    tag_ranges.extend_from_slice(&0u16.to_le_bytes()); // padding

    // Write tag StrRefs
    for tag in &mv.tags {
        let (str_off, str_len) = string_table.intern(tag.as_str());
        tag_strrefs.extend_from_slice(&str_off.to_le_bytes());
        tag_strrefs.extend_from_slice(&str_len.to_le_bytes());
        tag_strrefs.extend_from_slice(&0u16.to_le_bytes()); // padding
    }
}

// Add sections if there are any tags
if !tag_ranges.is_empty() {
    sections.push(Section {
        kind: SECTION_STATE_TAG_RANGES,
        data: tag_ranges,
        align: 4,
    });
    sections.push(Section {
        kind: SECTION_STATE_TAGS,
        data: tag_strrefs,
        align: 4,
    });
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat(codegen): encode state tags in FSPK

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 2.4: Add roundtrip test for tags

**Files:**
- Modify: `src-tauri/tests/zx_fspack_roundtrip.rs`

**Step 1: Write the roundtrip test**

```rust
#[test]
fn tags_survive_roundtrip() {
    let mut char_data = create_test_character();
    char_data.moves[0].tags = vec![
        Tag::new("normal").unwrap(),
        Tag::new("light").unwrap(),
    ];

    let pack_bytes = export_zx_fspack(&char_data).unwrap();
    let pack = PackView::parse(&pack_bytes).unwrap();

    let tags: Vec<&str> = pack.state_tags(0).unwrap().collect();
    assert_eq!(tags, vec!["normal", "light"]);
}

#[test]
fn empty_tags_roundtrip() {
    let char_data = create_test_character(); // moves have no tags

    let pack_bytes = export_zx_fspack(&char_data).unwrap();
    let pack = PackView::parse(&pack_bytes).unwrap();

    // Should return empty iterator, not None
    let tags: Vec<&str> = pack.state_tags(0).unwrap_or_default().collect();
    assert!(tags.is_empty());
}
```

**Step 2: Run the test**

Run: `cd src-tauri && cargo test tags_survive_roundtrip`
Expected: PASS

**Step 3: Commit checkpoint**

```bash
git add -A
git commit -m "checkpoint: phase 2 complete - FSPK tag support

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: Tag-Based Cancel Rules

### Task 3.1: Update CancelTable schema

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Write the test**

```rust
#[test]
fn cancel_table_with_tag_rules_deserializes() {
    let json = r#"{
      "tag_rules": [
        { "from": "normal", "to": "special", "on": "hit" },
        { "from": "hitstun", "to": "burst" }
      ],
      "chains": { "5L": ["5M", "5H"] },
      "deny": { "2H": ["jump"] }
    }"#;

    let ct: CancelTable = serde_json::from_str(json).expect("should parse");
    assert_eq!(ct.tag_rules.len(), 2);
    assert_eq!(ct.tag_rules[0].from, "normal");
    assert_eq!(ct.deny.get("2H"), Some(&vec!["jump".to_string()]));
}
```

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test cancel_table_with_tag_rules`
Expected: FAIL - no field `tag_rules`

**Step 3: Add CancelTagRule and update CancelTable**

```rust
/// Condition for when a cancel rule applies
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum CancelCondition {
    #[default]
    Always,
    Hit,
    Block,
    Whiff,
}

/// Tag-based cancel rule
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CancelTagRule {
    /// Source state must have this tag (or "any")
    pub from: String,
    /// Target state must have this tag (or "any")
    pub to: String,
    /// When the cancel is allowed
    #[serde(default)]
    pub on: CancelCondition,
    /// Minimum frame to allow cancel (0 = no minimum)
    #[serde(default)]
    pub after_frame: u8,
    /// Maximum frame to allow cancel (255 = no maximum)
    #[serde(default = "default_max_frame")]
    pub before_frame: u8,
}

fn default_max_frame() -> u8 { 255 }

/// Cancel table defining all state relationships
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, Default)]
pub struct CancelTable {
    /// Tag-based cancel rules (general patterns)
    #[serde(default)]
    pub tag_rules: Vec<CancelTagRule>,
    /// Explicit chain routes (target combos, rekkas)
    #[serde(default)]
    pub chains: std::collections::HashMap<String, Vec<String>>,
    /// Explicit deny overrides
    #[serde(default)]
    pub deny: std::collections::HashMap<String, Vec<String>>,
    // Legacy fields for backward compat during migration
    #[serde(default)]
    pub special_cancels: Vec<String>,
    #[serde(default)]
    pub super_cancels: Vec<String>,
    #[serde(default)]
    pub jump_cancels: Vec<String>,
}
```

**Step 4: Run test to verify it passes**

Run: `cd src-tauri && cargo test cancel_table_with_tag_rules`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/schema/mod.rs
git commit -m "feat(schema): add tag_rules and deny to CancelTable

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 3.2: Add FSPK sections for cancel rules

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`

**Step 1: Add section constants**

```rust
/// Section containing tag-based cancel rules
pub const SECTION_CANCEL_TAG_RULES: u32 = 19;
/// Section containing explicit deny pairs
pub const SECTION_CANCEL_DENIES: u32 = 20;

/// Size of a cancel tag rule entry (24 bytes)
pub const CANCEL_TAG_RULE_SIZE: usize = 24;
/// Size of a cancel deny entry (4 bytes: from u16, to u16)
pub const CANCEL_DENY_SIZE: usize = 4;
```

**Step 2: Implement CancelTagRuleView**

```rust
/// View into a single cancel tag rule
pub struct CancelTagRuleView<'a> {
    data: &'a [u8],
    pack: &'a PackView<'a>,
}

impl<'a> CancelTagRuleView<'a> {
    pub fn from_tag(&self) -> Option<&'a str> {
        let off = read_u32(self.data, 0);
        let len = read_u16(self.data, 4);
        if off == 0xFFFFFFFF { return None; } // "any"
        self.pack.string(off, len)
    }

    pub fn to_tag(&self) -> Option<&'a str> {
        let off = read_u32(self.data, 8);
        let len = read_u16(self.data, 12);
        if off == 0xFFFFFFFF { return None; } // "any"
        self.pack.string(off, len)
    }

    pub fn condition(&self) -> u8 {
        read_u8(self.data, 16)
    }

    pub fn min_frame(&self) -> u8 {
        read_u8(self.data, 17)
    }

    pub fn max_frame(&self) -> u8 {
        read_u8(self.data, 18)
    }

    pub fn flags(&self) -> u8 {
        read_u8(self.data, 19)
    }
}

/// View into cancel tag rules section
pub struct CancelTagRulesView<'a> {
    data: &'a [u8],
    pack: &'a PackView<'a>,
}

impl<'a> CancelTagRulesView<'a> {
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

    pub fn len(&self) -> usize {
        self.data.len() / CANCEL_TAG_RULE_SIZE
    }

    pub fn iter(&self) -> impl Iterator<Item = CancelTagRuleView<'a>> {
        (0..self.len()).filter_map(move |i| self.get(i))
    }
}

impl<'a> PackView<'a> {
    pub fn cancel_tag_rules(&self) -> Option<CancelTagRulesView<'a>> {
        self.section(SECTION_CANCEL_TAG_RULES)
            .map(|data| CancelTagRulesView { data, pack: self })
    }

    pub fn cancel_denies(&self) -> Option<&'a [u8]> {
        self.section(SECTION_CANCEL_DENIES)
    }

    pub fn has_cancel_deny(&self, from: u16, to: u16) -> bool {
        let Some(denies) = self.cancel_denies() else { return false };
        let count = denies.len() / CANCEL_DENY_SIZE;
        for i in 0..count {
            let off = i * CANCEL_DENY_SIZE;
            let deny_from = read_u16(denies, off);
            let deny_to = read_u16(denies, off + 2);
            if deny_from == from && deny_to == to {
                return true;
            }
        }
        false
    }
}
```

**Step 3: Verify it compiles**

Run: `cd crates/framesmith-fspack && cargo check`
Expected: Compiles

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/src/view.rs
git commit -m "feat(fspack): add cancel tag rules and denies views

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 3.3: Update encoder to write cancel rules

**Files:**
- Modify: `src-tauri/src/codegen/zx_fspack.rs`

**Step 1: Add cancel rule encoding**

```rust
// Encode cancel tag rules
let mut cancel_rules_data: Vec<u8> = Vec::new();
for rule in &char_data.cancel_table.tag_rules {
    // from_tag StrRef
    let (from_off, from_len) = if rule.from == "any" {
        (0xFFFFFFFFu32, 0u16)
    } else {
        string_table.intern(&rule.from)
    };
    cancel_rules_data.extend_from_slice(&from_off.to_le_bytes());
    cancel_rules_data.extend_from_slice(&from_len.to_le_bytes());
    cancel_rules_data.extend_from_slice(&0u16.to_le_bytes()); // pad

    // to_tag StrRef
    let (to_off, to_len) = if rule.to == "any" {
        (0xFFFFFFFFu32, 0u16)
    } else {
        string_table.intern(&rule.to)
    };
    cancel_rules_data.extend_from_slice(&to_off.to_le_bytes());
    cancel_rules_data.extend_from_slice(&to_len.to_le_bytes());
    cancel_rules_data.extend_from_slice(&0u16.to_le_bytes()); // pad

    // condition, min_frame, max_frame, flags
    let condition: u8 = match rule.on {
        CancelCondition::Always => 0,
        CancelCondition::Hit => 1,
        CancelCondition::Block => 2,
        CancelCondition::Whiff => 3,
    };
    cancel_rules_data.push(condition);
    cancel_rules_data.push(rule.after_frame);
    cancel_rules_data.push(rule.before_frame);
    cancel_rules_data.push(0); // flags

    // padding to 24 bytes
    cancel_rules_data.extend_from_slice(&0u32.to_le_bytes());
}

if !cancel_rules_data.is_empty() {
    sections.push(Section {
        kind: SECTION_CANCEL_TAG_RULES,
        data: cancel_rules_data,
        align: 4,
    });
}

// Encode cancel denies
let mut denies_data: Vec<u8> = Vec::new();
for (from_input, deny_list) in &char_data.cancel_table.deny {
    let from_idx = move_input_to_index.get(from_input.as_str());
    for to_input in deny_list {
        let to_idx = move_input_to_index.get(to_input.as_str());
        if let (Some(&from), Some(&to)) = (from_idx, to_idx) {
            denies_data.extend_from_slice(&(from as u16).to_le_bytes());
            denies_data.extend_from_slice(&(to as u16).to_le_bytes());
        }
    }
}

if !denies_data.is_empty() {
    sections.push(Section {
        kind: SECTION_CANCEL_DENIES,
        data: denies_data,
        align: 4,
    });
}
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src-tauri/src/codegen/zx_fspack.rs
git commit -m "feat(codegen): encode cancel tag rules and denies in FSPK

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 3.4: Update runtime can_cancel_to() with tag rules

**Files:**
- Modify: `crates/framesmith-runtime/src/cancel.rs`

**Step 1: Write the test**

```rust
#[cfg(test)]
mod tag_cancel_tests {
    use super::*;

    #[test]
    fn tag_rule_allows_normal_to_special_on_hit() {
        // Setup pack with:
        // - state 0 tagged "normal"
        // - state 1 tagged "special"
        // - tag rule: from="normal", to="special", on=hit
        let pack = build_test_pack_with_tag_rule("normal", "special", 1); // 1 = on_hit

        let mut state = CharacterState::default();
        state.current_move = 0;
        state.hit_confirmed = true;

        assert!(can_cancel_to(&state, &pack, 1));
    }

    #[test]
    fn tag_rule_denies_normal_to_special_on_whiff() {
        let pack = build_test_pack_with_tag_rule("normal", "special", 1); // on_hit only

        let mut state = CharacterState::default();
        state.current_move = 0;
        state.hit_confirmed = false; // whiffed

        assert!(!can_cancel_to(&state, &pack, 1));
    }

    #[test]
    fn explicit_deny_overrides_tag_rule() {
        let pack = build_test_pack_with_deny(0, 1); // deny 0->1

        let mut state = CharacterState::default();
        state.current_move = 0;
        state.hit_confirmed = true;

        assert!(!can_cancel_to(&state, &pack, 1));
    }
}
```

**Step 2: Implement tag rule evaluation in can_cancel_to**

```rust
/// Check if source state has a specific tag
fn state_has_tag(pack: &PackView, state_idx: u16, tag: &str) -> bool {
    pack.state_tags(state_idx as usize)
        .map(|tags| tags.any(|t| t == tag))
        .unwrap_or(false)
}

/// Context for cancel checking
pub struct CancelContext {
    pub frame: u8,
    pub hit_confirmed: bool,
    pub block_confirmed: bool,
}

impl CancelContext {
    fn matches_condition(&self, condition: u8) -> bool {
        match condition {
            0 => true, // always
            1 => self.hit_confirmed, // on_hit
            2 => self.block_confirmed, // on_block
            3 => !self.hit_confirmed && !self.block_confirmed, // on_whiff
            _ => false,
        }
    }
}

pub fn can_cancel_to(state: &CharacterState, pack: &PackView, target: u16) -> bool {
    let from = state.current_move;

    // 1. Explicit deny always wins
    if pack.has_cancel_deny(from, target) {
        return false;
    }

    // 2. Check explicit chain routes (existing logic)
    if let Some(extras) = pack.move_extras() {
        if let Some(extra) = extras.get(from as usize) {
            let (off, len) = extra.cancels();
            if let Some(cancels) = pack.cancels() {
                for i in 0..len {
                    let cancel_target = read_u16(cancels, (off as usize) + (i as usize) * 2);
                    if cancel_target == target {
                        return true;
                    }
                }
            }
        }
    }

    // 3. Check tag rules
    let ctx = CancelContext {
        frame: state.frame,
        hit_confirmed: state.hit_confirmed,
        block_confirmed: state.block_confirmed,
    };

    if let Some(rules) = pack.cancel_tag_rules() {
        for rule in rules.iter() {
            // Check from tag matches (None means "any")
            let from_matches = rule.from_tag()
                .map(|t| state_has_tag(pack, from, t))
                .unwrap_or(true);

            if !from_matches { continue; }

            // Check to tag matches
            let to_matches = rule.to_tag()
                .map(|t| state_has_tag(pack, target, t))
                .unwrap_or(true);

            if !to_matches { continue; }

            // Check condition
            if !ctx.matches_condition(rule.condition()) { continue; }

            // Check frame range
            if ctx.frame < rule.min_frame() { continue; }
            if ctx.frame > rule.max_frame() { continue; }

            return true;
        }
    }

    false
}
```

**Step 3: Run tests**

Run: `cd crates/framesmith-runtime && cargo test tag_cancel`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/framesmith-runtime/src/cancel.rs
git commit -m "feat(runtime): implement tag-based cancel rule evaluation

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 3.5: Add integration test for cancel rules

**Files:**
- Modify: `src-tauri/tests/zx_fspack_roundtrip.rs`

**Step 1: Write integration test**

```rust
#[test]
fn cancel_tag_rules_roundtrip() {
    let mut char_data = create_test_character();

    // Add tags to moves
    char_data.moves[0].tags = vec![Tag::new("normal").unwrap()];
    char_data.moves[1].tags = vec![Tag::new("special").unwrap()];

    // Add tag rule
    char_data.cancel_table.tag_rules = vec![
        CancelTagRule {
            from: "normal".to_string(),
            to: "special".to_string(),
            on: CancelCondition::Hit,
            after_frame: 0,
            before_frame: 255,
        },
    ];

    let pack_bytes = export_zx_fspack(&char_data).unwrap();
    let pack = PackView::parse(&pack_bytes).unwrap();

    // Verify rule exists
    let rules = pack.cancel_tag_rules().unwrap();
    assert_eq!(rules.len(), 1);

    let rule = rules.get(0).unwrap();
    assert_eq!(rule.from_tag(), Some("normal"));
    assert_eq!(rule.to_tag(), Some("special"));
    assert_eq!(rule.condition(), 1); // on_hit
}
```

**Step 2: Run test**

Run: `cd src-tauri && cargo test cancel_tag_rules_roundtrip`
Expected: PASS

**Step 3: Commit checkpoint**

```bash
git add -A
git commit -m "checkpoint: phase 3 complete - tag-based cancel rules

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: Runtime Changes (instance_duration)

### Task 4.1: Add instance_duration to CharacterState

**Files:**
- Modify: `crates/framesmith-runtime/src/state.rs`

**Step 1: Update the size test to expect 23 bytes**

```rust
#[test]
fn character_state_size_is_small() {
    // Updated: now 23 bytes with instance_duration
    assert_eq!(core::mem::size_of::<CharacterState>(), 23);
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test character_state_size`
Expected: FAIL - expected 23, got 22

**Step 3: Add instance_duration field**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CharacterState {
    /// Current move index (0 = idle by convention).
    pub current_move: u16,
    /// Current frame within the move (0-indexed).
    pub frame: u8,
    /// Instance-specific duration override. 0 = use state's default total().
    pub instance_duration: u8,
    /// Move connected with a hit (opens on-hit cancel windows).
    pub hit_confirmed: bool,
    /// Move was blocked (opens on-block cancel windows).
    pub block_confirmed: bool,
    /// Resource pool values (meter, heat, ammo, etc.).
    pub resources: [u16; MAX_RESOURCES],
}
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test character_state_size`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/src/state.rs
git commit -m "feat(runtime): add instance_duration to CharacterState

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 4.2: Update next_frame to respect instance_duration

**Files:**
- Modify: `crates/framesmith-runtime/src/frame.rs`

**Step 1: Write the test**

```rust
#[test]
fn instance_duration_overrides_state_total() {
    let pack = build_test_pack_with_move_total(20); // state has total=20

    let mut state = CharacterState::default();
    state.current_move = 0;
    state.instance_duration = 10; // override to 10 frames
    state.frame = 9;

    let result = next_frame(&state, &pack, &FrameInput::default());

    // Frame 9 -> 10, but duration is 10, so move_ended should be true
    assert!(result.move_ended);
}

#[test]
fn zero_instance_duration_uses_state_total() {
    let pack = build_test_pack_with_move_total(20);

    let mut state = CharacterState::default();
    state.current_move = 0;
    state.instance_duration = 0; // use default
    state.frame = 19;

    let result = next_frame(&state, &pack, &FrameInput::default());

    // Frame 19 -> 20, state total is 20, so move_ended should be true
    assert!(result.move_ended);
}
```

**Step 2: Run test to verify it fails**

Run: `cd crates/framesmith-runtime && cargo test instance_duration`
Expected: FAIL - instance_duration not considered

**Step 3: Update next_frame logic**

In the `next_frame` function, update the move_ended calculation:

```rust
// Calculate effective duration
let effective_duration = if state.instance_duration > 0 {
    state.instance_duration
} else {
    move_data.total() as u8
};

let move_ended = new_state.frame >= effective_duration;
```

**Step 4: Run test to verify it passes**

Run: `cd crates/framesmith-runtime && cargo test instance_duration`
Expected: PASS

**Step 5: Commit checkpoint**

```bash
git add -A
git commit -m "checkpoint: phase 4 complete - instance_duration support

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 5: The Big Rename

### Task 5.1: Rename in Schema (Move → State)

**Files:**
- Modify: `src-tauri/src/schema/mod.rs`

**Step 1: Global find/replace in schema file**

Replace:
- `pub struct Move` → `pub struct State`
- `impl Default for Move` → `impl Default for State`
- `Move {` → `State {` (in default impl)
- `fn move_` → `fn state_` (any helper functions)
- Update doc comments: "Move" → "State", "move" → "state"

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Many errors in other files referencing `Move`

**Step 3: Fix commands.rs references**

Update imports and usages in `src-tauri/src/commands.rs`

**Step 4: Fix codegen references**

Update `src-tauri/src/codegen/zx_fspack.rs`

**Step 5: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

**Step 6: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All pass

**Step 7: Commit**

```bash
git add src-tauri/
git commit -m "refactor(schema): rename Move to State

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 5.2: Rename in Runtime

**Files:**
- Modify: `crates/framesmith-runtime/src/state.rs`
- Modify: `crates/framesmith-runtime/src/frame.rs`
- Modify: `crates/framesmith-runtime/src/cancel.rs`
- Modify: `crates/framesmith-runtime/src/resource.rs`

**Step 1: Rename CharacterState fields**

```rust
pub struct CharacterState {
    pub current_state: u16,  // was current_move
    // ... rest unchanged
}

pub struct FrameInput {
    pub requested_state: Option<u16>,  // was requested_move
}
```

**Step 2: Update all references in runtime crate**

Global find/replace:
- `current_move` → `current_state`
- `requested_move` → `requested_state`

**Step 3: Verify it compiles**

Run: `cd crates/framesmith-runtime && cargo check`
Expected: Compiles

**Step 4: Run tests**

Run: `cd crates/framesmith-runtime && cargo test`
Expected: All pass

**Step 5: Commit**

```bash
git add crates/framesmith-runtime/
git commit -m "refactor(runtime): rename move to state in CharacterState

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 5.3: Rename in FSPK

**Files:**
- Modify: `crates/framesmith-fspack/src/view.rs`
- Modify: `crates/framesmith-fspack/src/lib.rs`

**Step 1: Rename view types**

- `MoveView` → `StateView`
- `MovesView` → `StatesView`
- `MoveExtrasRecordView` → `StateExtrasRecordView`
- `MoveExtrasView` → `StateExtrasView`
- `SECTION_MOVES` → `SECTION_STATES` (constant name only, value stays 4)

**Step 2: Update lib.rs re-exports**

**Step 3: Verify it compiles**

Run: `cd crates/framesmith-fspack && cargo check`
Expected: Compiles

**Step 4: Commit**

```bash
git add crates/framesmith-fspack/
git commit -m "refactor(fspack): rename Move views to State views

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 5.4: Rename in Frontend

**Files:**
- Modify: `src/lib/types.ts`
- Rename: `src/lib/views/MoveEditor.svelte` → `src/lib/views/StateEditor.svelte`
- Modify: `src/lib/stores/character.svelte.ts`
- Modify: `src/lib/views/FrameDataTable.svelte`
- Modify: `src/lib/training/MoveResolver.ts` → `StateResolver.ts`

**Step 1: Update types.ts**

```typescript
// Rename interface
export interface State {  // was Move
  input: string;
  name: string;
  tags: Tag[];
  // ...
}
```

**Step 2: Rename MoveEditor.svelte**

```bash
git mv src/lib/views/MoveEditor.svelte src/lib/views/StateEditor.svelte
```

**Step 3: Update imports throughout frontend**

**Step 4: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 5: Commit**

```bash
git add src/
git commit -m "refactor(frontend): rename Move to State

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 5.5: Rename in WASM wrapper

**Files:**
- Modify: `crates/framesmith-runtime-wasm/src/lib.rs`

**Step 1: Update field names**

```rust
pub struct CharacterState {
    pub current_state: u32,  // was current_move
    // ...
}
```

**Step 2: Verify it compiles**

Run: `cd crates/framesmith-runtime-wasm && cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add crates/framesmith-runtime-wasm/
git commit -m "refactor(wasm): rename move to state

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 5.6: Update all tests

**Step 1: Find and update all test files**

Run: `grep -r "current_move\|Move\b" --include="*.rs" --include="*.ts" crates/ src-tauri/ src/`

Update each occurrence.

**Step 2: Run full test suite**

Run: `cargo test && npm test`
Expected: All pass

**Step 3: Commit checkpoint**

```bash
git add -A
git commit -m "checkpoint: phase 5 complete - the big rename

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 6: Migration Script

### Task 6.1: Create migration script

**Files:**
- Create: `scripts/migrate-project.ts`

**Step 1: Write the migration script**

```typescript
#!/usr/bin/env npx ts-node

import * as fs from 'fs';
import * as path from 'path';

interface OldState {
  type?: string;
  tags?: string[];
  [key: string]: unknown;
}

interface OldCancelTable {
  chains?: Record<string, string[]>;
  special_cancels?: string[];
  super_cancels?: string[];
  jump_cancels?: string[];
  tag_rules?: unknown[];
  deny?: Record<string, string[]>;
}

function migrateStateFile(filePath: string): void {
  const content = fs.readFileSync(filePath, 'utf-8');
  const state: OldState = JSON.parse(content);

  // Convert type to tag
  if (state.type && !state.tags?.includes(state.type)) {
    state.tags = state.tags || [];
    state.tags.push(state.type);
  }
  delete state.type;

  // Ensure tags exists
  state.tags = state.tags || [];

  fs.writeFileSync(filePath, JSON.stringify(state, null, 2));
  console.log(`Migrated: ${filePath}`);
}

function migrateCancelTable(filePath: string): void {
  const content = fs.readFileSync(filePath, 'utf-8');
  const table: OldCancelTable = JSON.parse(content);

  // Convert legacy cancel lists to tag rules
  table.tag_rules = table.tag_rules || [];

  // Note: Full conversion requires knowing which states have which tags
  // This is a simplified migration that preserves legacy fields
  // Users should manually update to tag_rules

  fs.writeFileSync(filePath, JSON.stringify(table, null, 2));
  console.log(`Migrated: ${filePath}`);
}

function migrateProject(projectPath: string): void {
  const charactersDir = path.join(projectPath, 'characters');

  for (const charDir of fs.readdirSync(charactersDir)) {
    const charPath = path.join(charactersDir, charDir);
    if (!fs.statSync(charPath).isDirectory()) continue;

    // Migrate moves (states)
    const movesDir = path.join(charPath, 'moves');
    if (fs.existsSync(movesDir)) {
      for (const file of fs.readdirSync(movesDir)) {
        if (file.endsWith('.json')) {
          migrateStateFile(path.join(movesDir, file));
        }
      }
    }

    // Migrate cancel table
    const cancelTablePath = path.join(charPath, 'cancel_table.json');
    if (fs.existsSync(cancelTablePath)) {
      migrateCancelTable(cancelTablePath);
    }
  }

  console.log('Migration complete!');
}

// Run
const projectPath = process.argv[2] || '.';
migrateProject(projectPath);
```

**Step 2: Make it executable**

```bash
chmod +x scripts/migrate-project.ts
```

**Step 3: Test on a sample project**

Run: `npx ts-node scripts/migrate-project.ts ./test-project`
Expected: Files migrated without errors

**Step 4: Commit**

```bash
git add scripts/migrate-project.ts
git commit -m "feat: add project migration script

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 7: Cleanup

### Task 7.1: Update documentation

**Files:**
- Modify: `docs/zx-fspack.md`
- Modify: `CLAUDE.md`

**Step 1: Update zx-fspack.md**

- Rename "MoveRecord" to "StateRecord" in docs
- Add sections 17-20 documentation
- Update examples to use "state" terminology

**Step 2: Update CLAUDE.md**

- Change "move" to "state" in terminology
- Update "moves/" to "states/" if directory renamed

**Step 3: Commit**

```bash
git add docs/ CLAUDE.md
git commit -m "docs: update terminology Move to State

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task 7.2: Final verification

**Step 1: Run full test suite**

```bash
cd src-tauri && cargo test
cd crates/framesmith-runtime && cargo test
cd crates/framesmith-fspack && cargo test
npm test
```
Expected: All pass

**Step 2: Run the app**

```bash
npm run tauri dev
```
- Create a new state with tags
- Verify cancel rules work
- Export to FSPK and verify

**Step 3: Grep for remaining "move" references**

```bash
grep -ri "move" --include="*.rs" --include="*.ts" --include="*.svelte" | grep -v "node_modules" | grep -v "target"
```
Review each match - some may be intentional (e.g., "movement")

**Step 4: Final commit**

```bash
git add -A
git commit -m "checkpoint: phase 7 complete - Move to State refactor done

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Verification Checklist

After completing all phases, verify:

- [ ] `cargo test` passes in all crates
- [ ] `npm test` passes
- [ ] App launches without errors
- [ ] Can create states with tags
- [ ] Tags display in frame data table
- [ ] Cancel rules with tags work (normal→special on hit)
- [ ] Explicit chains work (rekka sequences)
- [ ] Explicit deny works
- [ ] FSPK export includes tags and cancel rules
- [ ] FSPK import preserves tags and cancel rules
- [ ] Migration script works on sample project
- [ ] No remaining "Move" types in API surface
