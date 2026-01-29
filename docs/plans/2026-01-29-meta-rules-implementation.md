# Meta Rules System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a rule engine that applies project/character default values and validation constraints to moves, based on `framesmith.rules.json` + optional `characters/<id>/rules.json`.

**Architecture:** Parse rules JSON into Rust structs, resolve each move by applying `apply` rules in order (without overriding explicit move values), then run `validate` rules on the resolved move and surface errors/warnings.

**Tech Stack:** Rust (`serde`, `serde_json`), existing Tauri commands (`src-tauri/src/commands.rs`) and validation plumbing (`src-tauri/src/mcp/validation.rs`).

---

## Notes / MVP Scope

- This plan implements the rules engine as described in `docs/plans/2026-01-29-meta-rules-design.md`.
- Move fields are currently strongly typed; to preserve "explicit values always win", the engine treats JSON defaults as "unset" (e.g. `0`, `""`, empty arrays/objects). Rules may override earlier rules, but not explicit move values.
- Apply/validate semantics are implemented generically via `serde_json::Value` walking, so rules can target nested fields like `pushback.hit` and `meter_gain.whiff`.

---

### Task 1: Add Rules Module + JSON Schema Types

**Files:**
- Create: `src-tauri/src/rules/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Steps:**
1. Create `crate::rules` module and export it from `src-tauri/src/lib.rs`.
2. Implement `RulesFile`, `ApplyRule`, `ValidateRule`, `MatchSpec`, `Severity`.
3. Add serde helpers so match fields can be a single value or an array (OR semantics per field).
4. Add a loader helper that reads a rules JSON file from disk and returns `Option<RulesFile>`.

**Verification:**
- Run: `cd src-tauri && cargo test`
  Expected: PASS

---

### Task 2: Implement Match Evaluation (Including Input Glob)

**Files:**
- Modify: `src-tauri/src/rules/mod.rs`

**Steps:**
1. Implement a minimal glob matcher supporting `*` and `?` with all other characters treated literally (so patterns like `[*]*` work).
2. Implement `matches_move(&MatchSpec, &Move) -> bool` with AND semantics across fields.
3. Implement derived `button` matcher by extracting the last alphabetic character from `Move.input` (e.g. `5L`, `j.H`, `236P`).
4. Implement `tags` matcher (requires `Move.tags: Vec<String>`; if missing in file, defaults empty).

**Verification:**
- Run: `cd src-tauri && cargo test rules::tests::test_glob_*`
  Expected: PASS

---

### Task 3: Apply Engine (Defaults That Don’t Overwrite Explicit Fields)

**Files:**
- Modify: `src-tauri/src/rules/mod.rs`

**Steps:**
1. Add helpers to convert `Move` to/from `serde_json::Value`.
2. Implement `apply_rules_to_move(project, character, mv) -> Result<Move, RulesError>`.
3. Implement rule stacking: later rules overwrite earlier rules, but only for fields that are "unset" in the original move value.
4. Implement project+character merge behavior: character rules replace project rules with identical `match` specs (apply + validate separately).

**Verification:**
- Run: `cd src-tauri && cargo test rules::tests::test_apply_*`
  Expected: PASS

---

### Task 4: Validate Engine (Errors + Warnings)

**Files:**
- Modify: `src-tauri/src/rules/mod.rs`
- Modify: `src-tauri/src/mcp/validation.rs`

**Steps:**
1. Add `ValidationSeverity` and `ValidationIssue { field, message, severity }`.
2. Keep the existing `validate_move(&Move)` behavior for schema sanity checks, but return issues with severity `error`.
3. Implement rule-driven constraints (`min`, `max`, `exists`, `equals`, `in`) over resolved move JSON values.

**Verification:**
- Run: `cd src-tauri && cargo test`
  Expected: PASS

---

### Task 5: Integrate Rules Into Load/Export

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Steps:**
1. In `load_character`, load project rules from `<parent-of-characters-dir>/framesmith.rules.json` (if present) and character rules from `characters/<id>/rules.json`.
2. Resolve each move using apply rules before returning `CharacterData`.
3. In `export_character`, run rule-driven validation and block export if any `error` issues exist (include messages in the error string).

**Verification:**
- Run: `cd src-tauri && cargo test`
  Expected: PASS

---

### Task 6: Add Example Rules File (Optional, For Manual Testing)

**Files:**
- Create: `framesmith.rules.json` (if missing)

**Steps:**
1. Add a minimal example `framesmith.rules.json` that sets a default `hitstop` and warns on missing `animation`.

**Verification:**
- Run app / MCP tool `get_character` and confirm defaults apply.
