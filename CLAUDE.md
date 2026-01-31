# Claude Guidelines for Framesmith

**Status:** Active
**Last reviewed:** 2026-02-01

## Project Context

Framesmith is an engine-agnostic fighting game character authoring tool. It edits characters as JSON on disk (git-friendly, one state per file) and exports to runtime-friendly formats.

## Tech Stack

- **Desktop framework:** Tauri (Rust backend + web frontend)
- **Frontend:** Svelte + TypeScript
- **3D rendering:** Threlte / Three.js (dependency; preview UI is not yet implemented)
- **Data format:** JSON (directory-based, one file per state)

## Project + Character Data Layout

A Framesmith project is a folder containing `framesmith.rules.json` and a `characters/` directory.

Each character is stored as a directory inside `characters/`:

```
<project>/
  framesmith.rules.json
  characters/
    test_char/
      character.json         # Properties (health, speed, etc.)
      cancel_table.json      # Cancel relationships (explicit and tag-based rules)
      states/
        5L.json              # One file per state
        236P.json
      rules.json             # Optional character-level rules overrides
```

Notes:

- Some projects may also contain `assets.json` / `hurtboxes.json` in a character folder; the current backend ignores these files.
- Canonical field definitions live in `src-tauri/src/schema/mod.rs`.

## Export Adapters

| Adapter | Output | Use case |
|---------|--------|----------|
| `json-blob` | Single JSON file | Runtime loading / debugging |
| `zx-fspack` | `.fspk` binary pack | Nethercore ZX (`no_std`/WASM-friendly) |

ZX FSPK details: `docs/zx-fspack.md`.

## Key Design Decisions

1. **Engine-agnostic:** Data format is JSON, exporters handle engine-specific output
2. **Directory-based:** One file per state for easy git diffs and merge conflicts
3. **Central cancel table:** Cancel relationships in one file (supports explicit pairs and tag-based rules)
4. **Tag-based cancel rules:** States can be tagged (e.g., "normal", "special") and cancel rules can match by tag
5. **Rules-driven validation:** Defaults + constraints configured via rules files
6. **Registry-aware:** Resources/events validated against a registry (no silent typos)

## Editor Views

1. **Character Overview** - Character list, properties
2. **Frame Data Table** - Spreadsheet view with configurable type-based filtering
3. **State Editor** - Form-based editing with sprite and GLTF animation preview
4. **Cancel Graph** - Visualization of cancel relationships (supports both explicit cancel tables and tag-based rules)

## MCP Server

Framesmith includes an MCP server binary (`src-tauri/src/bin/mcp.rs`). Configuration and tool list: `docs/mcp-server.md`.
