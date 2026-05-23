# Export Fidelity Contract

Status: active
Last reviewed: 2026-05-23

This document explains the machine-readable contract in
`docs/export-fidelity-contract.json`. The contract exists so export behavior
cannot drift silently as the Rust schema changes.

## Status Values

- `preserved`: the adapter carries the field or an accepted structural
  equivalent.
- `derived`: the adapter intentionally transforms the field into a runtime form.
- `omitted`: the adapter does not carry the field.
- `engine-owned`: the field is authored in Framesmith, but the consuming game or
  runtime currently owns the behavior.

## Adapter Policy

- `json-blob` is the canonical production handoff for the first production
  target. It serializes `CharacterData` directly and is the safest handoff when
  a game needs every authored field.
- `fspk` is a compact runtime pack. It preserves the current runtime-critical
  subset and intentionally derives or omits editor-facing and unresolved
  gameplay fields.

See [`production-handoff-decision.md`](production-handoff-decision.md) for the
formal handoff decision and FSPK v1 movement policy.

## Known FSPK V1 Limits

- Resolved variant `id` is not stored separately from gameplay `input`.
- Advanced `hits[]` are not exported; legacy `hitboxes[]` are exported.
- Advanced hurtbox shapes and flags are not exported; legacy `hurtboxes[]` are
  exported.
- Only resource-based `preconditions[]` and `costs[]` are exported.
- `movement`, `super_freeze`, some `on_use`, `on_hit`, and `on_block` gameplay
  fields still need an ownership decision before `fspk` can be a full production
  handoff.

## FSPK V1 Lossy Examples

These examples are intentional v1 behavior. A production game that needs these
fields at runtime should use `json-blob` as the complete handoff or fund the
corresponding FSPK v2/runtime work.

### Resolved Variant Identity

Input:

```json
{
  "id": "5H~level2",
  "input": "5H",
  "name": "Standing Heavy Level 2"
}
```

FSPK v1 result: `input` is preserved as `5H`; `id` and `name` are omitted. The
runtime cannot distinguish this resolved variant from another state with the
same gameplay input unless the consuming game keeps a side table.

### Advanced Multi-Hit Data

Input:

```json
{
  "input": "236P",
  "hits": [
    { "frames": [6, 7], "damage": 20 },
    { "frames": [12, 13], "damage": 30 }
  ],
  "hitboxes": [{ "frames": [6, 13], "box": { "x": 20, "y": -40, "w": 32, "h": 18 } }]
}
```

FSPK v1 result: `hits[]` is omitted. The legacy `hitboxes[]` data is exported
as `HIT_WINDOWS` plus `SHAPES`.

### Movement Ownership

Input:

```json
{
  "input": "66",
  "movement": { "distance": 80, "direction": "forward" }
}
```

FSPK v1 result: `movement` is not serialized. The game engine owns dash
distance, collision against stage bounds/corners, and rollback state for
movement.

### Advanced Hurtbox Flags

Input:

```json
{
  "input": "j.H",
  "advanced_hurtboxes": [
    { "frames": [0, 12], "shape": { "type": "circle", "x": 0, "y": -48, "r": 18 }, "flags": ["airborne"] }
  ]
}
```

FSPK v1 result: `advanced_hurtboxes[]` is omitted. Only legacy rectangular
`hurtboxes[]` are exported as `HURT_WINDOWS` plus `SHAPES`.

### Super Freeze

Input:

```json
{
  "input": "236236P",
  "super_freeze": { "frames": 40, "darken": 0.6, "zoom": 1.2 }
}
```

FSPK v1 result: `super_freeze` is omitted. The game engine must schedule camera,
screen-darkening, and freeze behavior from JSON data or engine-side scripts.

The Rust test `export_fidelity_contract_covers_current_schema_direct_fields`
checks that every direct `Character`, `State`, and `CancelTable` field has a
classification for every adapter listed in the JSON contract.

The Rust test `fspk_preserved_and_derived_fields_have_named_roundtrip_coverage`
checks that every `fspk` field classified as `preserved` or `derived` is mapped
to at least one named `fspk_roundtrip` test that reads the exported bytes through
`framesmith-fspack`.

When a target game promotes an omitted or engine-owned field into the runtime
handoff, start from the matching item in
[`production-gap-backlog.md`](production-gap-backlog.md), update the JSON
contract, and add the required roundtrip/runtime tests before changing this
document's status language.
