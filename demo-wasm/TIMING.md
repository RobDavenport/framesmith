# SF6 Ryu-inspired timing subset

This educational kit demonstrates how authored fighting-game data becomes working game code. Original block meshes and four-button controls present a sourced timing subset; the [motion study](art/README.md) reconstructs the corresponding Ryu gestures. It is not a complete or frame-perfect SF6 character recreation.

## Copied source and frame convention

The snapshot in `frame-reference.json` copies nine attack rows plus Denjin Charge's 52-frame total from Capcom's official SF6 Ryu table, retrieved 2026-09-25.[1]

- `5L` ← standing light punch: **4 / 3 / 7**, +4 / −1.
- `5M` ← standing medium punch: **6 / 4 / 11**, +7 / −1; **two extra recovery frames on whiff**.
- `5H` ← Collarbone Breaker (`6MP`): **20 / 4 / 19**, +3 / −3.
- `2L` ← crouching light kick: **5 / 2 / 10**, +3 / −1.
- `2M` ← crouching medium kick: **8 / 3 / 19**, +1 / −6.
- `2H` ← crouching heavy punch: **9 / 6 / 22**, +1 / −7.
- Target kick ← High Double Strike: **9 / 4 / 20**, knockdown / −8.
- `5S` ← light Hashogeki: **12 / 6 / 18**, +2 / −3.
- Lab variant ← medium Hashogeki: **19 / 6 / 17**, +2 / −6.[1]

Triples use **one-based first active / active count / recovery**, as Capcom does. The runtime's `startup` stores pre-active ticks: `first_active − 1`. A 6f MP contacts on simulation tick 6, not tick 7. This demo runs combat at 60 Hz; the engine does not mandate a render or simulation rate. The frame-data UI displays source convention; source JSON displays runtime convention.

The host counts impact as frame zero and then `stun` subsequent ticks. To reproduce the table's first-contact advantage: `stun = active + contact_recovery − 1 + advantage`. This conversion is not a claim to copy SF6's internal raw stun implementation. Shared hitstop is excluded from advantage.

MP authors its full **13f whiff recovery** so collision windows remain validated through its last frame. `properties.whiff_recovery = 2` selects the existing runtime instance-duration override on hit/block, yielding **11f contact recovery**. The new property is bounded and validated by the consumer. It adds no binary field, pack version, or engine rule.

## Recognizable moves, explicit adaptation

The timing-only pass reused unrelated motions and was rejected by the user. The subsequent [motion study](art/README.md) rebuilds the jab, compact MP, one-arm overhead, low checks, hand-braced MK, uppercut, high turning kick and asymmetric Hashogeki from visible Ryu references. These remain hand-authored reconstructions, not extracted animation data. The medium Hashogeki-inspired variation retains the old internal `special~charged` identifier for compatibility; its UI no longer claims to be Denjin-boosted Hashogeki.

The source's MP +7 advantage genuinely links into its 6f MP. The first manual lesson now teaches **5M, 5M**. The deliberately broken lab jab → MP experiment is still broken at source defaults; shortening its recovery is an explicit authoring experiment, not a hidden practice/trial preset.

This is **not full SF6 parity**. Original collision, locomotion, displacement, damage scaled to one tenth, resource rules, 8/10/12-tick hitstop, low-chain cancels, remapped MP→target, refill/super cancels, Twin Pulse and the three-hit barrage are demo policy. The copied table does not provide the host's hitstop values. Target kick uses standing stun instead of source knockdown; the overhead does not implement the source's conditional second-hit guard behavior. Denjin contributes only its **52f duration**; instant ammo refill remains a labeled demo extension rather than pretending to implement SF6 stock gained on frame 51.[1]

The fastest real normals are still fast. No global slow-motion multiplier, modified core clock, new dependencies or binary schema were needed. The motion study replaces the block atlases, not the sourced timing rows. Longer overhead/special anticipation, copied recovery, and stronger shared impact holds address readability without falsifying the source's light/medium startup.

## Acceptance gates

- Copied numeric rows, one-based conversion, first-contact advantage and MP whiff/contact distinction checked against the frozen reference.
- Existing native/browser routes, wrong-input/gap rejection, source→FSPK parity, export/reimport and rollback/replay remain green.
- Real 1× before/after gameplay, exact freeze, browser errors and encoded playback checked; direct delivery for human review.

Sources:
[1] https://www.streetfighter.com/6/en-us/character/ryu/frame
