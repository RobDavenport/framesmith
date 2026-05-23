# Production Handoff Decision

Status: active
Last reviewed: 2026-05-23

This document records the current production handoff policy for teams using
Framesmith in a real game pipeline.

## Decision

For the first production target, `json-blob` is the canonical source-of-truth
runtime handoff.

`fspk` v1 is a compact validated runtime pack for the subset currently covered
by `docs/export-fidelity-contract.md`, but it is not the only authoritative
handoff for a full game integration. A game may generate FSPK from the canonical
JSON data as a cache or fast-load runtime artifact.

## Why

- `json-blob` preserves every resolved `CharacterData` field.
- `fspk` v1 intentionally omits or derives editor-facing fields, resolved
  variant identity, advanced hit data, movement values, advanced hurtbox flags,
  and super-freeze behavior.
- Shipping a game from `json-blob` plus optional FSPK caches avoids pretending
  the current binary format is full-fidelity before FSPK v2 exists.

## Movement Policy

Movement is `json-blob` only for FSPK v1.

Framesmith can author `movement` values and `json-blob` preserves them. FSPK v1
may mark a state as movement by type, but it does not serialize movement
distance, velocity, acceleration, frame ranges, or easing. The consuming engine
owns movement application, collision against floors/walls/corners, stage bounds,
and any velocity or movement accumulator values needed for rollback.

## Example Pipeline

Canonical handoff:

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter json-blob --pretty --out ../exports/test_char.json
```

Optional runtime pack:

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --character test_char --adapter fspk --out ../exports/test_char.fspk
```

Engine-side policy:

- Load `json-blob` for full authored data, tooling, debugging, movement, and
  mechanics not represented in FSPK v1.
- Load `fspk` for covered runtime-fast paths such as state timing, legacy hit
  and hurt windows, pushboxes, resource records, events, tags, cancel rules, and
  compact properties.
- Treat FSPK as a generated cache unless the target game explicitly accepts the
  v1 subset as complete for its combat model.

## When This Decision Changes

Use FSPK as the canonical handoff only after FSPK v2 or later has:

- A migration plan for existing packs.
- Field classifications updated in `docs/export-fidelity-contract.json`.
- Roundtrip tests for every newly preserved or derived field.
- Runtime or engine-consumption examples for movement and any newly runtime-owned
  combat mechanics.
