\

**Status:** Approved
**Created:** 2026-02-01

## Overview

Training Mode currently renders a placeholder viewport (`TrainingViewport.svelte`) while the State Editor preview renders real visuals via separate components (`SpritePreview.svelte` + `GltfPreview.svelte`). This plan unifies Training Mode and the State Editor preview behind a shared rendering core so both use the same asset loading, animation sampling, and rendering behavior.

Primary goal: Training Mode 3D preview uses the same rendering core as the 2D/3D preview, with a single shared viewport (one scene/camera) rendering both player and dummy.

Non-goals:

- Changing Training Mode simulation logic (WASM tick loop)
- Reworking hitbox editor UX (existing overlays remain in the State Editor)

## Key Decisions

- **One shared viewport**: Training Mode renders both actors in a single scene/camera.
- **One shared core**: Extract a framework-agnostic render core (TypeScript) and use it from both State Editor preview and Training viewport.
- **Deterministic state indices**: Canonicalize state ordering so WASM indices map correctly to authoring data (and therefore to animations).

## Problem: State Index Ordering Must Be Canonical

Training Mode relies on `CharacterState.current_state` (WASM) and passes indices into `session.tick(...)`. Those indices must refer to the same ordered state list that the exporter wrote into the `.fspk`.

Current risks:

- Backend loads `states/*.json` via `fs::read_dir` without sorting, so `CharacterData.moves` order can vary by filesystem.
- Training Mode currently sorts moves for UI purposes, which can make indices diverge from pack order.

Decision:

- Define canonical state order as: sort by `State.input` ascending (after rules are applied).
- Enforce it in:
  - `src-tauri/src/commands.rs` when producing `CharacterData.moves`
  - `src-tauri/src/codegen/zx_fspack.rs` before packing into FSPK
- Training UI may still sort for display, but must preserve canonical indices for runtime.

## Architecture

Add a shared render core under `src/lib/rendercore/`:

- `RenderCore` owns viewport lifecycle, resizing (DPR-aware), rendering clock, and an actor list.
- Actors are independent render units (sprite or glTF) managed by the core.
- Asset I/O is abstracted via an `AssetProvider` so embedded and detached windows can load assets consistently.

Svelte components become thin wrappers:

- State Editor preview uses the core in single-actor mode.
- Training viewport uses the core in training mode with two actors and a shared camera.

## RenderCore API (Proposed)

Core surface area (framework-agnostic TS, no Svelte):

- Lifecycle:
  - `mount(containerEl: HTMLElement)`
  - `unmount()`
- Sizing:
  - `setViewportSize(w: number, h: number, dpr: number)`
- Clock:
  - `setClockMode('manual' | 'raf')`
  - `renderOnce()` (manual)
  - `start()` / `stop()` (raf)
- Scene:
  - `setSceneMode('single' | 'training')`
  - `setActors(actors: ActorSpec[])`
- Status:
  - `getActorStatus(id)` returns `{ loading, error }`

ActorSpec includes:

- `id` (e.g. `p1`, `cpu`)
- `pos: { x, y }`, `facing: 'left' | 'right'`
- `visual`:
  - `sprite`: `{ texturePath, clip, frameIndex }`
  - `gltf`: `{ modelPath, clip, frameIndex }`

## Rendering Approach

Recommended implementation: a unified Three.js-backed viewport.

- Sprites render as textured quads in the same Three.js scene.
- glTF renders as skinned meshes with `AnimationMixer`, sampled by `frameIndex / clip.fps`.
- Training Mode uses one renderer/canvas and a camera that frames both actors.

State Editor overlay note:

- Hitbox/hurtbox editing overlays remain outside the core.
- The core exposes enough camera/world transform helpers (or stable viewport rules) so overlays can remain as a separate layer.

## Data Flow

Training Mode per-frame:

1. WASM sim tick updates `playerState` / `dummyState`.
2. Map `current_state` (index) to canonical `State`:
   - `state = currentCharacter.moves[current_state]`
3. Map `State.animation` to `assets.animations[...]` clip.
4. Compute sampling frame:
   - sprite: clamp `frameIndex` to `[0..frames-1]`
   - glTF: sample time `t = frameIndex / clip.fps`
5. Pass two actors into `RenderCore` and render via the shared viewport.

Asset loading:

- Use a Tauri-backed `AssetProvider` that calls `read_character_asset_base64` with explicit `charactersDir + characterId`.
- Avoid relying on main-window stores so detached training can reuse the same provider.

## Error Handling

- Per-actor errors are isolated (P1 failure does not break CPU rendering).
- Missing animation/asset references produce a visible fallback (debug mesh/box) plus a clear error string.
- Async load cancellation uses a monotonic sequence counter per actor (`loadSeq`).
- `unmount()` disposes geometries/materials/textures/mixers and removes the renderer canvas.
- Rendering errors never throw into the Training Mode sim tick loop.

## Testing + Verification

Deterministic ordering:

- Rust tests:
  - `load_character` produces states sorted by `input`.
  - Exporting with shuffled input yields identical FSPK bytes.

RenderCore logic tests (TS):

- Frame sampling math (sprite clamp, glTF time mapping).
- Load cancellation (late resolves ignored after prop changes).
- Error surfacing (missing keys -> fallback + error).

Manual verification:

- State Editor: sprite + glTF preview works; scrubbing works; hitbox overlay editing unchanged.
- Training Mode (embedded + detached): both actors render in one viewport; animation tracks `current_state` and `frame`; missing assets degrade cleanly.

## File-Level Changes (Planned)

New:

- `src/lib/rendercore/RenderCore.ts`
- `src/lib/rendercore/assets/TauriAssetProvider.ts`
- `src/lib/rendercore/actors/GltfActor.ts`
- `src/lib/rendercore/actors/SpriteActor.ts`

Modify:

- `src/lib/components/MoveAnimationPreview.svelte` (use the shared core viewport)
- `src/lib/components/training/TrainingViewport.svelte` (replace placeholder DOM renderer)
- `src/lib/views/TrainingMode.svelte` (map WASM state indices to animation clips + pass actors)
- `src/routes/training/+page.svelte` (same as embedded mode)
- `src-tauri/src/commands.rs` (sort loaded states by `input`)
- `src-tauri/src/codegen/zx_fspack.rs` (enforce canonical sort before packing)

## Open Questions

- Sprite rendering implementation detail: GPU quad in Three.js (recommended) vs maintaining a separate Canvas2D stack. This plan assumes GPU quads to keep Training Mode as a single shared viewport.
