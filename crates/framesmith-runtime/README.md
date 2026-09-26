# framesmith-runtime

Small `no_std` frame/cancel/resource/collision helpers over `framesmith-fspack`.
The binary reader is usable independently; these convenience helpers do not
constitute a game engine. Version 0.2.0 is the source version, not a registry
publication claim. Default features require no allocator or Tauri dependency;
`alloc` enables allocated result helpers, and `std` includes it.

From the repository root:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin framesmith-cli -- export --project . --character test_char --adapter fspk --out exports/test_char.fspk
cargo run --manifest-path crates/framesmith-runtime/Cargo.toml --example headless -- exports/test_char.fspk
```

`headless` loads real binary data, steps 120 frames and checks snapshot/replay.
Add `--features std` before `--example`, and `--bench` after the pack path, to
measure validation, cached views and basic frame stepping on your machine.

`next_frame(&state, &pack, &input)` returns a new `Copy` state. It does not choose
idle states, apply movement, interpret an input language, set a clock frequency,
or implement guard/damage/animation policy. State indices are not gameplay input
strings or resolved variant IDs; use the pack's lookup/data views to map them.

0.2 migration: `frame` and `instance_duration` are u16 (state size is currently
24 bytes, not a portable serialization layout). Invalid target indices cannot
become states. Cost payment is atomic; `init_resources` returns false without
mutation when the fixed eight-slot helper cannot represent resource definitions.
Use typed payloads and engine-owned state for larger/different resource models.
Action flags have the separate `can_cancel_action` query; they are not state IDs.

Collision helpers use integer pixel geometry: AABB/AABB, circle/circle,
AABB/circle and capsule/capsule. Rotated rectangles and mixed capsule pairs are
not supported by `shapes_overlap`; it returns false for those pairs. Non-crossing
capsule projections are integer-quantized; consume exact payload geometry in the
engine for subpixel collision policies. Snapshot engine-owned movement, entity,
event-consumption and other mutable state as well as `CharacterState` for rollback.
