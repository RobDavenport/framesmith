# Framesmith Design

**Status:** Draft
**Last reviewed:** 2026-01-28
**Scope:** Full design spec covering data model, editor UI, and export pipeline

## Overview

Framesmith is an engine-agnostic fighting game character authoring tool. It enables fast iteration on frame data, hitboxes, move lists, and character properties without requiring game engine rebuilds.

**Purpose:** Define complete, portable fighting game characters in a format that can be exported to any game engine.

**Core workflow:**
1. Author characters in Framesmith (desktop app)
2. Save as directory structure (JSON files + asset references)
3. Export to engine-specific format via codegen adapters
4. Game engine consumes the output (Rust constants, binary blob, etc.)

## Goals

- **Fast iteration:** Tweak frame data and see results without full rebuilds
- **Visual overview:** Spreadsheet view for comparing characters side-by-side
- **Engine-agnostic:** Portable to future fighting games, not locked to Breakpoint
- **Complete characters:** Frame data, hitboxes, moves, properties, state machines, animations, assets

## Tech Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Desktop framework | Tauri | Small binaries, Rust backend aligns with ecosystem |
| Frontend | Svelte + TypeScript | Lightweight, compiles away, good for form-heavy UIs |
| 3D rendering | Threlte (Three.js) | Native Svelte integration, animation timeline support |
| Data format | JSON | Human-readable, easy to diff/version |
| Backend | Rust | File I/O, JSON parsing, codegen, asset loading |

## Data Model

### Directory Structure

A character is a directory with this structure:

```
characters/
  glitch/
    character.json        # Identity + global properties
    moves/
      5L.json             # One file per move (numpad notation)
      5M.json
      5H.json
      2L.json
      j.H.json
      236P.json           # Special moves
      632146PP.json       # Supers
      ...
    cancel_table.json     # All cancel relationships
    hurtboxes.json        # Default hurtbox sets (stand, crouch, airborne)
    assets.json           # References to mesh, textures, animations
```

### character.json

Global character properties:

```json
{
  "id": "glitch",
  "name": "GLITCH",
  "archetype": "rushdown",
  "health": 1000,
  "walk_speed": 4.5,
  "back_walk_speed": 3.2,
  "jump_height": 120,
  "jump_duration": 45,
  "dash_distance": 80,
  "dash_duration": 18
}
```

### moves/{input}.json

Single move definition:

```json
{
  "input": "5L",
  "name": "Standing Light",
  "startup": 7,
  "active": 3,
  "recovery": 8,
  "damage": 30,
  "hitstun": 17,
  "blockstun": 11,
  "hitstop": 6,
  "guard": "mid",
  "hitboxes": [
    { "frames": [7, 9], "box": { "x": 0, "y": -40, "w": 30, "h": 16 } }
  ],
  "hurtboxes": [
    { "frames": [0, 6], "box": { "x": -10, "y": -60, "w": 30, "h": 60 } },
    { "frames": [7, 17], "box": { "x": 0, "y": -55, "w": 35, "h": 55 } }
  ],
  "pushback": { "hit": 2, "block": 2 },
  "meter_gain": { "hit": 5, "whiff": 2 },
  "animation": "stand_light"
}
```

### cancel_table.json

Central relationship graph defining all cancel routes:

```json
{
  "chains": {
    "5L": ["5L", "5M", "2M", "2L"],
    "5M": ["5H", "2H"],
    "2L": ["2M", "5M"]
  },
  "special_cancels": ["5L", "5M", "5H", "2L", "2M", "2H"],
  "super_cancels": ["5H", "2H", "236P", "214P"],
  "jump_cancels": ["5H"]
}
```

### assets.json

Asset references (hybrid model - references during dev, bake for export):

```json
{
  "mesh": "meshes/glitch.glb",
  "textures": {
    "base": "textures/glitch_base.png",
    "matcap": "textures/matcap_chrome.png"
  },
  "animations": {
    "stand_light": "animations/glitch/5L.glb",
    "stand_medium": "animations/glitch/5M.glb"
  }
}
```

## Editor UI

### Four Main Views

**1. Character Overview**
- Left panel: character list (all characters in project)
- Center: selected character's properties (health, speed, etc.)
- Right: at-a-glance stats (total moves, archetype, portrait)

**2. Move List / Frame Data Table**
- Spreadsheet view of all moves for a character
- Columns: input, startup, active, recovery, total, damage, hitstun, blockstun, advantage on hit, advantage on block
- Sortable, filterable (show only lights, only specials, etc.)
- Click row to open move editor
- Multi-character comparison mode: side-by-side tables for balance checking

**3. Move Editor**
- Left: form fields for all frame data properties
- Center: animation preview with timeline scrubber
- Right: hitbox/hurtbox overlay editor
  - Scrub to frame, draw/resize boxes on that frame
  - Toggle hitbox vs hurtbox visibility
  - Copy hitbox data across frames

**4. Cancel Graph**
- Visual node graph showing move relationships
- Nodes = moves, edges = cancel routes
- Color-coded by cancel type (chain, special, super, jump)
- Click edge to remove, drag node-to-node to add
- Auto-layout with manual adjustment

### Animation Timeline Controls

For the move editor's animation preview:

- Play / Pause button
- Frame scrubber (drag to any frame)
- Step backward / forward (single frame with `←` / `→` keys)
- Current frame display: `Frame 7 / 24`
- Speed control (0.5x, 1x, 2x)

Hitbox editing happens with animation paused at a specific frame.

## Export System

### Pipeline

```
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  Character dir  │  →   │  framesmith     │  →   │  Engine output  │
│  (JSON files)   │      │  export <adapter>│     │                 │
└─────────────────┘      └─────────────────┘      └─────────────────┘
```

### Adapters

| Adapter | Output | Use case |
|---------|--------|----------|
| `breakpoint-rust` | Rust constants matching `AttackData` struct | Breakpoint (compile-time) |
| `json-blob` | Single minified JSON file | Runtime loading |

### breakpoint-rust Output Example

```rust
// Generated by Framesmith - do not edit
pub const STAND_LIGHT: AttackData = AttackData {
    startup: FramesU8(7),
    active: FramesU8(3),
    recovery: FramesU8(8),
    damage: Damage(30),
    hitstun: FramesU8(17),
    blockstun: FramesU8(11),
    // ...
};
```

### Asset Baking

For portable distribution:

```bash
framesmith bake glitch --output glitch.fgc
```

Produces a single `.fgc` (fighting game character) bundle containing:
- All JSON data
- Embedded assets (mesh, textures, animations)
- Portable package for sharing or porting to another project

## Project Structure

```
framesmith/
  src-tauri/                # Rust backend
    src/
      main.rs
      codegen/
        mod.rs
        breakpoint.rs       # Breakpoint adapter
        json_blob.rs        # JSON blob adapter
      schema/               # Character data types
      assets/               # Asset loading for preview
  src/                      # Frontend (Svelte + TypeScript)
    lib/
      components/
      stores/
      views/
        CharacterOverview.svelte
        FrameDataTable.svelte
        MoveEditor.svelte
        CancelGraph.svelte
    App.svelte
    main.ts
  characters/               # Working directory for character data
  Cargo.toml
  package.json
  tauri.conf.json
```

## First Integration: Breakpoint

Initial use case is migrating Breakpoint's existing characters:

1. Define JSON schema matching current `AttackData` fields
2. Export existing 6 characters from Rust to Framesmith format (one-time migration)
3. Build `breakpoint-rust` adapter to generate Rust constants
4. Update Breakpoint build to run `framesmith export breakpoint-rust`
5. Remove hand-written Rust constants, use generated code

## Future Considerations

These are explicitly out of scope for v1 but noted for later:

- **Live connection to game:** WebSocket or file-watching for hot-reload (left to game engine)
- **Animation/frame-data sync:** Auto-adjusting animations when frame counts change (game-dependent)
- **Additional adapters:** Godot, Unity, etc. (add when needed)
- **Collaborative editing:** Multi-user support (not needed for internal tool)
- **Plugin system:** Custom validators, exporters (premature for v1)

## Open Questions

1. **Schema versioning:** How to handle breaking changes to the JSON format?
2. **Hitbox coordinate system:** Screen-space or character-local? Pixels or normalized?
3. **Animation format:** Accept .glb only, or also sprite sheets for 2D games?
