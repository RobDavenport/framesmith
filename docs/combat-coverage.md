# Combat Coverage

Status: active
Last reviewed: 2026-05-22

This matrix classifies common fighting-game mechanics by current Framesmith
support. It should be reviewed before choosing Framesmith as the production
authoring pipeline for a game.

## Coverage Levels

- `supported end-to-end`: authoring data, export, runtime behavior, and tests
  exist in the current pipeline.
- `exported as data only`: Framesmith preserves or emits data, but the game
  engine applies behavior.
- `engine-owned`: the game engine owns the mechanic today; Framesmith may expose
  related authoring fields.
- `out of scope`: not currently represented in the data model or runtime plan.

## Mechanic Matrix

| Mechanic | Current Level | Notes |
|----------|---------------|-------|
| Core state timing | supported end-to-end | `startup`, `active`, `recovery`, `total`, and `next_frame()` are covered by runtime tests. |
| Tag-rule cancels | supported end-to-end | `tag_rules`, `deny`, cancel conditions, frame windows, and resource preconditions are exported to FSPK and tested. |
| Legacy hitboxes/hurtboxes | supported end-to-end | Legacy rectangular hit/hurt windows export to FSPK and are consumed by `check_hits()`. |
| Pushboxes | supported end-to-end | `pushboxes[]` export to FSPK and are consumed by `check_pushbox()`. Stage/corner policy remains engine-owned. |
| Character resources | supported end-to-end | Resource definitions, resource costs, and resource preconditions are exported and consumed by the runtime. |
| State tags and custom properties | exported as data only | Tags and properties export to FSPK; game-specific property behavior is engine-owned. |
| Events and event args | exported as data only | Event emits and primitive args export to FSPK; dispatch timing and side effects are engine-owned. |
| Resource deltas | exported as data only | FSPK stores resource deltas. The engine applies them when a hit/block/whiff/event becomes authoritative. |
| Meter gain | exported as data only | Legacy `meter_gain` is derived into meter resource deltas for FSPK when nonzero. Runtime does not auto-apply hit/whiff gain. |
| Hitstop | exported as data only | Hit windows carry hitstop. The engine schedules attacker/defender freeze and rollback-authoritative timing. |
| Blockstop | engine-owned | No separate blockstop field exists. Engines can use hitstop or custom properties until a dedicated field is added. |
| Chip damage | exported as data only | `hits[].chip_damage` exists in JSON. FSPK v1 legacy hit windows currently encode chip damage as zero. |
| Multi-hit attacks | exported as data only | `hits[]` exists in JSON. FSPK v1 exports legacy `hitboxes[]`, not the advanced `hits[]` model. |
| Throws | engine-owned | States can be typed/tagged as throws, but throw boxes, throw tech, invulnerability, and throw-vs-hit resolution are engine-owned. |
| Projectiles/spawned entities | engine-owned | JSON can describe `on_use.spawn_entity`; FSPK v1 does not serialize projectile behavior. |
| Movement curves/velocity | engine-owned | `json-blob` preserves `movement`; FSPK v1 does not serialize movement values and runtime does not apply movement. |
| Forced movement/launch/knockback | engine-owned | JSON has `on_hit.knockback`; FSPK v1 does not serialize it and runtime does not apply launch/forced movement. |
| Status/timed effects | engine-owned | JSON has status effect structures; FSPK v1 does not serialize timed effect behavior. |
| State transition events | engine-owned | JSON has `on_use.enters_state`; FSPK v1 does not serialize transition events and runtime does not auto-transition. |

## Production Decision

Use `json-blob` as the production handoff when the target game needs the full
combat model today. Use `fspk` as the production handoff only if the target game
accepts the current runtime subset or commits to the missing FSPK/runtime work
listed above.

See also:

- [`export-fidelity-contract.md`](export-fidelity-contract.md)
- [`runtime-guide.md`](runtime-guide.md)
- [`data-formats.md`](data-formats.md)
- [`production-gap-backlog.md`](production-gap-backlog.md)
