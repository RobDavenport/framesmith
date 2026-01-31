# Training Mode Design

**Status:** Approved
**Created:** 2026-01-31

## Overview

Training mode allows character creators to play as their character against a configurable dummy, providing immediate feedback on how the character feels and validating frame data, cancel routes, and combos.

## Key Decisions

| Aspect | Decision |
|--------|----------|
| **Goal** | Full training mode - play as character against configurable dummy |
| **Input methods** | Keyboard (real-time) + sequence recorder (precise combos) |
| **Dummy V1** | Stand, crouch, jump, block (standing/crouching/auto) |
| **Visuals** | Side-by-side 2D view, classic fighting game camera |
| **HUD** | Developer-focused: frame data, cancels, hitbox toggle, input history |
| **Location** | Fifth main view + detachable window for live editing workflow |
| **Data sync** | User toggle: live sync OR reload on save |
| **Runtime** | WASM in browser (compiled from framesmith-runtime) |
| **Input config** | Project-level in `framesmith.rules.json` |
| **Notation** | Numpad (industry standard, matches existing move files) |

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Framesmith Editor                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │ Move Editor │  │ Frame Data  │  │ Training Mode (View 5)  │ │
│  │             │  │   Table     │  │                         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│         │                │                      │               │
│         └────────────────┼──────────────────────┘               │
│                          ▼                                      │
│              character.svelte.ts (store)                        │
│                          │                                      │
│         ┌────────────────┼────────────────┐                     │
│         ▼                ▼                ▼                     │
│   Save to disk    Live sync to      Auto-reload to              │
│   (Tauri cmd)     Training Mode     detached window             │
└─────────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
    ┌─────────────────┐      ┌─────────────────────┐
    │ Training Mode   │      │ Training Mode       │
    │ (embedded view) │      │ (detached window)   │
    └────────┬────────┘      └──────────┬──────────┘
             │                          │
             └──────────┬───────────────┘
                        ▼
              ┌─────────────────┐
              │ framesmith-     │
              │ runtime.wasm    │
              │ (in-browser)    │
              └─────────────────┘
```

## WASM Integration

### New crate: `framesmith-runtime-wasm`

Thin wrapper around `framesmith-runtime` exposing functions to JavaScript via `wasm-bindgen`.

```
crates/
  framesmith-runtime/          # Existing, no changes needed
  framesmith-runtime-wasm/     # New wrapper crate
    Cargo.toml
    src/lib.rs                 # wasm-bindgen bindings
```

### TypeScript API

```typescript
export class TrainingSession {
  static new(player_fspk: Uint8Array, dummy_fspk: Uint8Array): TrainingSession;
  tick(player_input: number, dummy_state: DummyState): FrameResult;
  player_state(): CharacterState;
  dummy_state(): CharacterState;
  available_cancels(): number[];
  hit_results(): HitResult[];
}

export interface CharacterState {
  current_move: number;
  frame: number;
  hit_confirmed: boolean;
  block_confirmed: boolean;
  resources: number[];
}

export interface FrameResult {
  player: CharacterState;
  dummy: CharacterState;
  hits: HitResult[];
}
```

## Input System

### Project configuration (`framesmith.rules.json`)

```json
{
  "training_inputs": {
    "directions": {
      "up": "KeyW",
      "down": "KeyS",
      "left": "KeyA",
      "right": "KeyD"
    },
    "buttons": {
      "L": "KeyU",
      "M": "KeyI",
      "H": "KeyO",
      "P": "KeyJ",
      "K": "KeyK",
      "S": "KeyL"
    }
  }
}
```

### Input handling flow

```
Keyboard Event (keydown/keyup)
        │
        ▼
┌─────────────────────┐
│ InputManager.svelte │  Tracks held keys, converts to numpad + buttons
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│ Input Buffer        │  Stores recent inputs, detects motions (236, etc.)
└─────────────────────┘
        │
        ▼
┌─────────────────────┐
│ Move Resolver       │  Matches buffer to move names, checks available_cancels()
└─────────────────────┘
        │
        ▼
  Pass move index to WASM tick()
```

### Input sequence recorder

```typescript
interface RecordedSequence {
  name: string;
  inputs: { frame: number; input: string }[];
}
```

- Record keyboard inputs with frame timing
- Manually build sequences via UI
- Playback at real speed or frame-by-frame
- Save/load sequences to project folder

## Dummy Behavior

### States (V1)

| State | Behavior |
|-------|----------|
| `stand` | Idle standing, takes hits standing |
| `crouch` | Crouch animation, takes hits crouching |
| `jump` | Jumps in place on loop, takes hits airborne |
| `block_stand` | Blocks standing (high) |
| `block_crouch` | Blocks crouching (low) |
| `block_auto` | Blocks high/low based on attack type |

### Configuration

```typescript
interface DummyConfig {
  state: 'stand' | 'crouch' | 'jump' | 'block_stand' | 'block_crouch' | 'block_auto';
  recovery: 'neutral' | 'reversal';
  reversal_move?: string;
  counter_on_hit: boolean;
}
```

## Visual Display & HUD

```
┌─────────────────────────────────────────────────────────────────────┐
│ [Player Health ████████████]              [Dummy Health ████████████]│
│ [Meter ████░░░░] [Heat 3]                 [Meter ░░░░░░░░]          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│         ┌─────────┐                       ┌─────────┐               │
│         │ Player  │                       │  Dummy  │               │
│         │ Sprite/ │                       │ Sprite/ │               │
│         │  GLTF   │                       │  GLTF   │               │
│         └─────────┘                       └─────────┘               │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ 5L (3/2/9)  frame 4        │ Standing          │ Combo: 3 hits 847 │
│ Cancels: 5M, 2M, 5H, 236P  │ +6 on hit         │ [Input History]   │
│                            │ -2 on block       │ ↓ 5L              │
│ [▶][⏸][⏮][⏭] Speed: 1x    │                   │ ↓ 5M              │
└─────────────────────────────────────────────────────────────────────┘
```

### HUD elements

| Element | Description |
|---------|-------------|
| Health bars | Damage visualization, resets on command |
| Resource meters | All character resources |
| Move name + frame data | Current move, startup/active/recovery |
| Frame counter | Current frame within move |
| Available cancels | Live list from `available_cancels()` |
| Frame advantage | On hit / on block |
| Combo display | Hit count + total damage |
| Input history | Scrolling list of recent inputs |
| Playback controls | Play/pause, step, speed |

### Hitbox overlay (toggle)

- Red = hitboxes (attack areas)
- Green = hurtboxes (vulnerable areas)
- Blue = pushboxes (collision)

## Data Sync

### Two modes (user toggle)

| Mode | Behavior | Use case |
|------|----------|----------|
| **Live sync** | Changes reflect instantly as you edit | Rapid iteration |
| **Sync on save** | Reloads only on file save | Stability |

### Detached window communication

Uses `BroadcastChannel` API for cross-window sync:

```typescript
const channel = new BroadcastChannel('framesmith-training-sync');

// Main window sends
channel.postMessage({ type: 'change', character: data });

// Detached window receives
channel.onmessage = (event) => reloadCharacterData(event.data.character);
```

## File Structure

### New files

```
crates/
  framesmith-runtime-wasm/
    Cargo.toml
    src/lib.rs

src/lib/
  wasm/
    framesmith_runtime.js
    framesmith_runtime.d.ts
    framesmith_runtime_bg.wasm

  training/
    TrainingSession.ts
    InputManager.svelte.ts
    InputBuffer.ts
    MoveResolver.ts
    DummyController.ts
    SequenceRecorder.ts

  components/
    training/
      TrainingViewport.svelte
      TrainingHUD.svelte
      DummySettings.svelte
      InputHistory.svelte
      PlaybackControls.svelte
      SequencePanel.svelte

  views/
    TrainingMode.svelte
```

### Modified files

- `src/routes/+page.svelte` - Add Training Mode to view switcher
- `src/lib/stores/character.svelte.ts` - Add change/save events
- `src-tauri/src/lib.rs` - Command to open detached window
- `framesmith.rules.json` schema - Add `training_inputs`
- `package.json` - Add wasm-pack build scripts

## Implementation Phases

### Phase 1: WASM Foundation
- Create `framesmith-runtime-wasm` crate
- Set up wasm-pack build pipeline
- TypeScript wrapper (`TrainingSession.ts`)
- Verify: load FSPK → tick → get state

### Phase 2: Input System
- `InputManager` - keyboard handling
- `InputBuffer` - motion detection
- `MoveResolver` - match inputs to moves
- Project config for `training_inputs`

### Phase 3: Core Training View
- `TrainingViewport` - render player + dummy
- Basic `TrainingHUD` - health, resources, move name
- `DummyController` - state machine
- Add as fifth view

### Phase 4: Developer Overlay
- Frame counter, cancel windows, frame advantage
- Hitbox/hurtbox overlay toggle
- Input history display
- Combo counter + damage

### Phase 5: Detached Window + Sync
- Tauri command for detached window
- BroadcastChannel sync
- Live / save sync toggle

### Phase 6: Sequence Recorder
- Record inputs with timing
- Manual sequence builder
- Playback controls
- Save/load sequences
