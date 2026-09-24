# Schema Migration Notes

Status: active
Last reviewed: 2026-05-23

This document records authoring-data migrations that matter when updating an
older Framesmith project to the current schema.

## Current Canonical Shape

- `character.json` stores custom gameplay fields in `character.properties`.
- `character.resources[]` declares named resources such as heat, ammo, level,
  or install flags.
- State files may use `id` to distinguish resolved variants that share a
  gameplay `input`.
- `cancel_table.json` supports both legacy `rules[]` and tag-based
  `tag_rules[]` plus `deny`.
- The binary export adapter is named `fspk`; `zx-fspack` is accepted only as a
  legacy alias.

## Character Properties

Older projects may have fixed character fields at the top level. Move them into
`properties` so rules, UI, JSON export, and FSPK property export see the same
data.

Before:

```json
{
  "id": "glitch",
  "name": "GLITCH",
  "health": 10000,
  "walk_speed": 4.0
}
```

After:

```json
{
  "id": "glitch",
  "name": "GLITCH",
  "properties": {
    "health": 10000,
    "walk_speed": 4.0
  },
  "resources": []
}
```

## Cancel Tables

Legacy route rules can remain in `rules[]`. Prefer `tag_rules[]` for production
characters because tags survive state renames and variant expansion.

```json
{
  "rules": [
    { "from": "5L", "to": "5M", "on": ["hit", "block"] }
  ],
  "tag_rules": [
    { "from": "normal", "to": "special", "on": ["hit", "block"] }
  ],
  "deny": {
    "5H": ["5H"]
  }
}
```

## Variant States

Resolved variants are read-only editor snapshots. Author variants as overlay
JSON files and give resolved states stable `id` values when they share the same
gameplay `input`.

```json
{
  "id": "5H~level2",
  "input": "5H",
  "name": "Standing Heavy Level 2",
  "tags": ["5h", "variant"]
}
```

Do not serialize a fully resolved state back into an overlay patch. The current
State Editor, MCP `update_move`, and Tauri `save_move` path reject resolved
variant saves.

## Export Adapter Rename

Use:

```bash
cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter fspk --out ../exports/test_char.fspk
```

The `zx-fspack` adapter name remains as a compatibility alias for old scripts,
but new docs and automation should use `fspk`.

## Verification

After migration, run:

```bash
npm run check
npm run test:run
cargo test --manifest-path src-tauri/Cargo.toml --test pipeline_e2e
cargo test --manifest-path src-tauri/Cargo.toml --test export_fidelity_contract
```

For a release candidate, run the full checklist in
[`production-readiness-plan.md`](production-readiness-plan.md).
