# Training Scenario Contract

Status: active
Last reviewed: 2026-05-23

This document defines the current first-target training scenarios that must keep
working before Framesmith can be considered production ready for game
development.

The current target fixture is `characters/test_char` exported to
`exports/test_char.fspk`. A real game may add stricter policies, but these
scenarios are the minimum executable contract for authored training behavior.

## Scenario Matrix

| Scenario | Policy | Evidence |
|----------|--------|----------|
| Authored hitstun | A hit routes the defender into an authored `hitstun` state found by input or tag and sets `instance_duration` from the hit's `hitstun`. | `target_training_fixture_resolves_authored_reaction_states` in `crates/framesmith-runtime-wasm/src/lib.rs` |
| Authored blockstun | Blocking dummy modes route the defender into an authored `blockstun` state found by input or tag and set `instance_duration` from `blockstun`. | `target_training_fixture_resolves_authored_reaction_states` in `crates/framesmith-runtime-wasm/src/lib.rs` |
| Resource policy | FSPK preserves target fixture resource definitions and resource deltas; the engine applies resource side effects at the authoritative hit/block/use timing. | `target_training_fixture_preserves_resource_and_throw_policies` and `fspk_exports_resources_and_events_sections` |
| Throw input policy | Throw states can be authored as `type: "throw"` and resolved from inputs such as `5T`; throw collision, teching, invulnerability, and priority remain engine-owned. | `target_training_fixture_preserves_resource_and_throw_policies`, `supports authored throw inputs that use the T button`, and `resolves throw inputs through the regular input path` |
| Roundtrip reload | The exported pack must parse after generation, preserve state inputs/tags/resources, and remain usable by the training WASM wrapper. | `fspk_roundtrip`, `export_fidelity_contract`, and `crates/framesmith-runtime-wasm` tests |
| Embedded training smoke | The editor Training view loads rebuilt WASM and real CLI-exported FSPK bytes, then can switch to a separate dummy character/FSPK. | `loads training mode from rebuilt WASM and exported FSPK data` in `tests/e2e/editor-smoke.spec.ts` |
| Detached training smoke | The detached route receives character/project data over BroadcastChannel and starts from real exported FSPK bytes. | `loads detached training mode through BroadcastChannel sync` in `tests/e2e/editor-smoke.spec.ts` |

## Engine-Owned Policies

The current contract intentionally leaves these policies to the consuming game:

- Throw boxes, throw tech, throw invulnerability, and throw-vs-strike priority.
- Resource side-effect timing beyond exported costs/preconditions/deltas.
- Hitstop, blockstop, rollback freeze scheduling, and visual/audio event
  dispatch.
- Stage bounds, corner behavior, forced movement, launch, and knockdown state
  machines.

If a target game wants Framesmith to own any of these behaviors, add a concrete
scenario here before changing runtime or FSPK behavior, then link the scenario
to the matching item in [`production-gap-backlog.md`](production-gap-backlog.md).

## Verification Commands

Run these before changing this contract:

```bash
npm run test:run -- src/lib/views/training/TrainingLoop.test.ts src/lib/training/buildMoveList.test.ts src/lib/training/InputManager.test.ts
cargo test --manifest-path crates/framesmith-runtime-wasm/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --test fspk_roundtrip
npm run test:e2e
```
