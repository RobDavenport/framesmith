# Framesmith Runtime Integration Guide

**Status:** Active
**Last reviewed:** 2026-02-01

## Overview

`framesmith-runtime` is a `no_std` Rust library for simulating fighting game character state machines. It provides the core gameplay logic for frame-by-frame character simulation, hit detection, and cancel validation.

### Design Philosophy

1. **Stateless**: Functions are pure - pass in state, get new state back
2. **Copy-friendly**: `CharacterState` is 22 bytes, `Copy`, and deterministic
3. **`no_std` compatible**: No heap allocations (unless `alloc` feature is enabled)
4. **Rollback-ready**: Cheap state cloning enables efficient rollback netcode

## Quick Start

### Minimal Example

```rust
use framesmith_runtime::{
    CharacterState, FrameInput, PackView,
    next_frame, init_resources,
};

// 1. Load the character pack (zero-copy parse)
let pack_data: &[u8] = load_fspk_file("character.fspk");
let pack = PackView::parse(pack_data).expect("invalid pack");

// 2. Initialize character state
let mut state = CharacterState::default();
init_resources(&mut state, &pack);

// 3. Run the game loop
loop {
    // Build input (None = continue current state)
    let input = FrameInput {
        requested_state: player_wants_to_attack().then_some(1),
    };

    // Advance one frame
    let result = next_frame(&state, &pack, &input);
    state = result.state;

    if result.move_ended {
        // State finished - transition to idle or next state
        state.current_state = 0; // Return to idle
        state.frame = 0;
    }
}
```

## Core Concepts

### Character State Lifecycle

A character's state is fully contained in `CharacterState`:

```rust
pub struct CharacterState {
    pub current_state: u16,       // Active state index (0 = idle by convention)
    pub frame: u8,                // Current frame within state
    pub instance_duration: u8,    // Override duration (0 = use state default)
    pub hit_confirmed: bool,      // Hit connected (opens on-hit cancels)
    pub block_confirmed: bool,    // Attack was blocked (opens on-block cancels)
    pub resources: [u16; 8],      // Resource pools (meter, heat, etc.)
}
```

The state progresses through frames automatically. When `frame` reaches the state's total duration, `FrameResult::move_ended` becomes `true`.

### Frame-by-Frame Simulation

Each frame, call `next_frame()`:

```rust
let result = next_frame(&state, &pack, &input);

// result.state    - The new character state
// result.move_ended - True if the state finished this frame
```

**Transition flow:**

1. If `input.requested_state` is `Some(target)` and the cancel is valid, transition immediately
2. Otherwise, advance `frame` by 1
3. Check if `frame >= total_duration` to set `move_ended`

### Input Handling and Cancel System

Players request state transitions via `FrameInput::requested_state`. The runtime validates cancels based on:

1. **Explicit denies** - Hard blocks between specific states
2. **Explicit chain cancels** - State-specific cancel routes (rekkas, target combos)
3. **Tag-based rules** - Pattern rules like "normals can cancel into specials on hit"

```rust
use framesmith_runtime::{can_cancel_to, available_cancels};

// Check if a specific cancel is valid
if can_cancel_to(&state, &pack, target_state) {
    // Cancel is allowed
}

// Get all valid cancel targets (requires "alloc" feature)
let cancels = available_cancels(&state, &pack);
for target in cancels {
    println!("Can cancel to state {}", target);
}

// For no_std: use the buffer variant
let mut buf = [0u16; 16];
let count = available_cancels_buf(&state, &pack, &mut buf);
```

**Cancel conditions:**

- `always` - Cancel allowed anytime in frame range
- `on_hit` - Only after `report_hit()` called
- `on_block` - Only after `report_block()` called
- `on_whiff` - Only when neither hit nor block confirmed

### Action Cancels

Actions are special cancel targets with IDs at or above the state count. They represent game-defined actions like jumping:

```rust
use framesmith_runtime::{ACTION_CHAIN, ACTION_SPECIAL, ACTION_SUPER, ACTION_JUMP};

let move_count = pack.states().map(|s| s.len()).unwrap_or(0) as u16;

// Request a jump cancel
let input = FrameInput {
    requested_state: Some(move_count + ACTION_JUMP),
};
```

The runtime checks the current state's cancel flags to allow/deny action cancels.

## Hit Detection

### Checking for Hits

Use `check_hits()` each frame to detect hitbox/hurtbox overlaps:

```rust
use framesmith_runtime::{check_hits, report_hit, report_block};

let hits = check_hits(
    &attacker_state, &attacker_pack, (attacker_x, attacker_y),
    &defender_state, &defender_pack, (defender_x, defender_y),
);

for hit in hits.iter() {
    println!("Hit! Damage: {}, Hitstun: {}", hit.damage, hit.hitstun);
}
```

### Processing Hit Results

`HitResult` contains all data needed to apply the hit:

```rust
pub struct HitResult {
    pub attacker_move: u16,    // Which state hit
    pub window_index: u16,     // Which hit window
    pub damage: u16,           // Damage to apply
    pub chip_damage: u16,      // Chip damage (on block)
    pub hitstun: u8,           // Hitstun frames
    pub blockstun: u8,         // Blockstun frames
    pub hitstop: u8,           // Freeze frames for both characters
    pub guard: u8,             // Guard type (high/mid/low)
    pub hit_pushback: i32,     // Pushback on hit (pixels)
    pub block_pushback: i32,   // Pushback on block (pixels)
}
```

### Reporting Hits and Blocks

After confirming a hit (game logic decides hit vs block), report it:

```rust
// On hit - opens on-hit cancel windows
report_hit(&mut attacker_state);

// On block - opens on-block cancel windows
report_block(&mut attacker_state);
```

This updates `hit_confirmed` or `block_confirmed` on the state, which tag-based cancel rules check.

## Resources

### Resource Pool Management

Characters have up to 8 resource pools (meter, heat, ammo, etc.). Initialize from the pack:

```rust
use framesmith_runtime::{init_resources, resource, set_resource};

// Initialize resources to starting values from pack
init_resources(&mut state, &pack);

// Read current value
let meter = resource(&state, 0);

// Set value manually (for game events)
set_resource(&mut state, 0, meter + 10);
```

### Resource Handling Split

The runtime handles costs and preconditions automatically:

| Aspect | Handled By | When |
|--------|------------|------|
| **Costs** | Runtime (automatic) | Deducted on state transition via `next_frame()` |
| **Preconditions** | Runtime (automatic) | Checked before allowing cancel in `can_cancel_to()` |
| **Deltas** | Engine (manual) | Applied by game based on events |

**Why deltas are manual:** The runtime is stateless and doesn't know when hits "count" - rollback might revert them. The engine must read deltas from the FSPK and apply them when appropriate:

```rust
// Example: applying on-hit resource delta (engine responsibility)
if hit_confirmed_by_game_logic {
    // Read deltas from FSPK and apply
    let meter_gain = 10; // from pack data
    let current = resource(&attacker_state, 0);
    set_resource(&mut attacker_state, 0, current.saturating_add(meter_gain));
}
```

## Integration Patterns

### Game Loop Integration

A typical game loop structure:

```rust
fn game_tick(game: &mut GameState) {
    // 1. Read player inputs and map to requested states
    let p1_input = FrameInput {
        requested_state: game.p1_buffered_input.take(),
    };
    let p2_input = FrameInput {
        requested_state: game.p2_buffered_input.take(),
    };

    // 2. Advance character states
    let p1_result = next_frame(&game.p1_state, &game.p1_pack, &p1_input);
    let p2_result = next_frame(&game.p2_state, &game.p2_pack, &p2_input);

    game.p1_state = p1_result.state;
    game.p2_state = p2_result.state;

    // 3. Check hits (both directions)
    let p1_hits = check_hits(
        &game.p1_state, &game.p1_pack, game.p1_pos,
        &game.p2_state, &game.p2_pack, game.p2_pos,
    );
    let p2_hits = check_hits(
        &game.p2_state, &game.p2_pack, game.p2_pos,
        &game.p1_state, &game.p1_pack, game.p1_pos,
    );

    // 4. Process hits (determine hit vs block, apply damage, etc.)
    for hit in p1_hits.iter() {
        process_hit(&mut game.p1_state, &mut game.p2_state, hit);
    }

    // 5. Handle state completion
    if p1_result.move_ended {
        game.p1_state.current_state = 0; // Return to idle
        game.p1_state.frame = 0;
    }
}
```

### Rollback Netcode Compatibility

`CharacterState` is designed for rollback:

```rust
// Save state (22 bytes, Copy, no heap)
let saved_state = game.p1_state;

// ... frames pass, prediction was wrong ...

// Restore state (instant, just a copy)
game.p1_state = saved_state;

// Re-simulate from saved frame with corrected inputs
for frame in rollback_start..current_frame {
    let input = get_corrected_input(frame);
    let result = next_frame(&game.p1_state, &game.p1_pack, &input);
    game.p1_state = result.state;
}
```

### WASM/Browser Usage

For browser usage, use `framesmith-runtime-wasm`:

```typescript
import { TrainingSession, DummyState } from 'framesmith-runtime-wasm';

// Load FSPK files
const playerFspk = await fetch('/characters/player.fspk').then(r => r.arrayBuffer());
const dummyFspk = await fetch('/characters/dummy.fspk').then(r => r.arrayBuffer());

// Create session
const session = new TrainingSession(
    new Uint8Array(playerFspk),
    new Uint8Array(dummyFspk)
);

// Game loop
function tick() {
    const playerInput = getPlayerInput(); // 0xFFFF = no input
    const result = session.tick(playerInput, DummyState.Stand);

    // result.player - player state
    // result.dummy  - dummy state
    // result.hits   - hit results this frame

    render(result);
    requestAnimationFrame(tick);
}
```

## Troubleshooting

### Cancel Not Working

1. **Check preconditions**: Does the target state require resources you don't have?
   ```rust
   if !check_resource_preconditions(&state, &pack, target) {
       println!("Missing resources for state {}", target);
   }
   ```

2. **Check frame range**: Is the current frame within the cancel window?

3. **Check condition**: Does the rule require `on_hit` but you haven't called `report_hit()`?

4. **Check explicit denies**: Is there a deny rule blocking this specific cancel?

### Hit Detection Not Working

1. **Verify positions**: Are character positions correct? Hit detection uses pixel coordinates.

2. **Check frame ranges**: Are you on an active frame? Hit windows have `start_frame..=end_frame`.

3. **Verify pack data**: Does the state have hit windows defined?
   ```rust
   if let Some(states) = pack.states() {
       if let Some(state) = states.get(state_id) {
           println!("Hit windows: {}", state.hit_windows_len());
       }
   }
   ```

### State Duration Issues

If states end too early or late:

1. **Check `instance_duration`**: Non-zero overrides the state's default duration
   ```rust
   // Reset to use default duration
   state.instance_duration = 0;
   ```

2. **Verify pack data**: Check the state's `total()` value matches expectations

### Resource Initialization

Resources are zero by default. Always call `init_resources()` after creating a new state:

```rust
let mut state = CharacterState::default();
init_resources(&mut state, &pack); // Sets starting values
```

## See Also

- [Runtime API Reference](runtime-api.md) - Complete type and function documentation
- [ZX FSPK Format](zx-fspack.md) - Binary pack format specification
- [Data Formats](data-formats.md) - On-disk JSON formats
