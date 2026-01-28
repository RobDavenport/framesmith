# Claude Guidelines for Framesmith

**Status:** Active
**Last reviewed:** 2026-01-29

## Project Context

Framesmith is an engine-agnostic fighting game character authoring tool. It allows defining complete, portable fighting game characters (frame data, hitboxes, moves, properties, state machines, animations) in a format that can be exported to any game engine.

## Tech Stack

- **Desktop framework:** Tauri (Rust backend + web frontend)
- **Frontend:** Svelte + TypeScript
- **3D rendering:** Threlte (Three.js wrapper for Svelte)
- **Data format:** JSON (directory-based, one file per move)

## Character Data Format

Characters are stored as directories:

```
characters/glitch/
  character.json         # Properties (health, speed, etc.)
  moves/
    5L.json              # One file per move
    5M.json
    236P.json
    ...
  cancel_table.json      # Cancel relationships
  hurtboxes.json         # Default hurtboxes
  assets.json            # Asset references
```

## Export Adapters

| Adapter | Output | Use case |
|---------|--------|----------|
| `breakpoint-rust` | Rust constants | Breakpoint (compile-time) |
| `json-blob` | Single JSON file | Runtime loading |

## Key Design Decisions

1. **Engine-agnostic:** Data format is JSON, exporters handle engine-specific output
2. **Directory-based:** One file per move for easy git diffs and merge conflicts
3. **Central cancel table:** All cancel relationships in one file for easy visualization
4. **Hybrid assets:** References during dev, bake to bundle for export
5. **Visual hitbox editing:** Edit hitboxes overlaid on animation frames

## Editor Views

1. **Character Overview** - Character list, properties
2. **Frame Data Table** - Spreadsheet view, multi-character comparison
3. **Move Editor** - Frame data form, animation preview, hitbox overlay
4. **Cancel Graph** - Visual node graph of cancel relationships
