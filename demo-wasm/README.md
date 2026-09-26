# FrameSmith Training Demo

[Play the hosted showcase](https://robdavenport.github.io/framesmith/).

**Play the combat. Inspect what happened. Change the actual data.**
One character, one dummy, eight guided experiments, free play, and seven manual combo trials. This replaces the platform-fighter landing page; the earlier `authoring/` fixtures remain untouched.

## Play first

Practice opens at 1× with the sourced SF6 Ryu-inspired timing subset, not a paused tutorial or hidden fast preset. See [TIMING.md](TIMING.md) and the frozen [numeric reference](frame-reference.json) for copied values, zero-based conversion and explicitly demo-only rules.

- **A / D** or arrows: move; **S / down**: crouch; **W / Space / up**: jump. Walk backward, jump over the dummy and attack facing the other way. Touch has a four-direction pad and exactly four attack buttons.
- **J / K / L / I** (also **1–4**): **L / M / H / S**. Standing normals are 5L/5M/5H; hold Down for 2L/2M/2H. S is the driver, Down+S reloads, Forward+S is the three-hit meter finisher. Forward is relative to facing, not always screen-right.
- **5M→5H** uses the High Double Strike-inspired high turning kick, distinct from raw 5H's one-arm Collarbone Breaker-inspired overhead. The [motion study](art/README.md) maps the actual Ryu reference gestures to the block rig. Seven lessons cover a link, special cancel, super cancel, combined route, low normal chain, target combo and reload cancel. Utilities count only when the refill succeeds; multi-hit actions count only after all authored hits connect.
- **R**: reset placement and attempt; **P**: pause; **.**: advance one frame.
- **F1 / Lab tools**: the optional authoring/inspection drawer. **F2**: frame meter; **F3**: collision boxes. Speed and reset stay directly accessible.
- Combo trials offer an on-stage route and immediate retry; a failed attempt resets after 60 simulation ticks rather than freezing the fight. A new attack or R retries immediately. Autoplay never earns a clear.
- Trial presets remain separate from the editable practice draft. Frame-accurate inputs, real contacts, uninterrupted hitstun and the specified link/cancel transitions still determine credit.

The first trial uses the real source **MP +7 → 6f MP** link, including the existing input buffer. For the deliberately broken jab→MP authoring experiment, **Reset edits** restores the copied 7-frame jab recovery; explicitly shorten it to 4 and compare. Practice, lab reset and trials retain the source defaults rather than silently making that custom route work.

**Art:** Original Blender-authored block mannequin, with 21 native-alpha clips covering the current attacks, reload, locomotion, jump/fall and reactions. The editable rig/actions and reproducible atlas pipeline are in [art/README.md](art/README.md). Native hit windows select contact poses (including every super/Twin Pulse hit); source animation FPS never controls combat timing. Default reach/spacing is calibrated to the unarmed figure. Arbitrary imported hitbox edits remain authoritative and do not automatically retarget the art. Existing Martial Hero files and their CC0 provenance are retained in `www/assets/`, but are no longer loaded. Exported projects contain combat authoring, not the consumer renderer/texture bundle.

Replay records movement, crouch, jump and resolved attacks along with the complete consumer state. Training continues across bounded 60-second recording segments; replay, rewind and checkpoints cover the current segment. Starting a new segment retires the old checkpoint, not the fight or reset placement. Facing is a consumer coordinate transform; compatible imported packs must keep hurt/push rectangles centered. Unsupported asymmetric body boxes are rejected without replacing the current simulation.

Automated checks verify the combat, data and input paths. Animation readability and game feel still require human play review.

## Feature coverage and ownership

1. **Timing and links:** edit recovery/startup; compare synchronized attacker/defender timelines. The frame table and editor timing utility use the same first-contact convention. A native test measures readiness on both actors and checks the displayed advantage, including shared hitstop. Reaction-state rows have no fictional attack advantage.
2. **Cancels:** enable/disable the actual tag rule, select hit/block/whiff/always, change its inclusive window, or add an explicit deny. Runtime eligibility is authoritative. The demonstration does not silently turn a denied cancel into a different link.
3. **Resources:** named energy/ammo pools, caps, starting values, requirements and atomic costs. The host applies the authored on-hit/on-use deltas. Reload explicitly starts that demonstration instance with empty ammo.
4. **Tags:** the custom `chainable` group covers the normal kit and target follow-up. The experiment removes it from 5L/5M; move categories also produce implicit runtime tags and are not confused with this custom group.
5. **Contact:** actual FSPK AABB hit/hurt/push windows, spacing, reach and guard policies. Separate read-only circle/circle, capsule/capsule and AABB/circle probes call the real geometry helpers and expose the exact caller inputs. They are labeled queries, not alternate fighter collision or combo evidence.
6. **Events and extensibility:** frame notifies, typed event arguments, hit/use triggers, optional synthesized sound, and a separate character `spark_size` property consumed by presentation. Twin Pulse applies the two rich per-hit definitions, not repeated contact on every active frame.
7. **Reuse:** `globals/states/idle.json` is included with a per-character name override; `special~charged.json` inherits the driver, while `5H~target.json` is the high turning-kick override. Source and resolved state can be inspected side by side. Full variant-overlay editing is not claimed.
8. **Validation and handoff:** a genuinely invalid resource reference is rejected by the shared validator without changing the current pack. Download the current FSPK or an editable project ZIP; compile that ZIP through the CLI and get identical binary bytes. Importing an edited pack preserves its actual data and locks unavailable overlay editing. A rejected import keeps the existing simulation.

Frame-data filtering/sorting, the rule graph, input history, forward/back stepping, complete checkpoints and per-frame replay are part of this screen. **What runs where?** distinguishes library behavior, consumer policy, retained data and editor/integration-only features. The native editor's sprite/GLTF asset workflow and CLI/MCP automation are not represented as a browser port. Movement, damage reactions, blocking, presentation, buffers, hit deduplication and trial policy belong to the consumer; the core is not a complete game engine.

## Real paths, not a JS imitation

`lab-authoring/` → existing CLI → `dist/packs/lab.fspk` → Rust/WASM runtime helpers.

Live edits use `framesmith-authoring`, a private portable build of the **same** schema, global resolver, variant flattener, rules/validators and exporters used by the desktop/CLI. Its library entrypoint is `src-tauri/src/portable.rs`; there is no copied exporter or new engine adapter. Compilation is transactional: construct and validate a replacement pack before replacing the current definition/world. Existing core runtime code and binary version remain unchanged. The demo kit has a version-2 marker; earlier seven-action kits retain their original four direct command bindings. Missing/partial new kits are explicit, not silently remapped. Legacy one-hit supers use a single-contact presentation rather than fake extra hits.

`dist/lab-source.json` is a generated build input embedded in the WASM authoring UI. It is **not fetched or used as a runtime character sidecar**. The combat consumer loads the binary; JavaScript supplies inputs and presentation only. Exported project file text is serialized in Rust, not normalized through JavaScript numbers: `18.0` and `18` can be different typed event arguments.

## Build and verify

Use the repository's existing Rust, Python, wasm-pack and Playwright installations:

```sh
python demo-wasm/build.py
python demo-wasm/serve.py
# http://127.0.0.1:8080/framesmith/

cargo fmt --check --manifest-path demo-wasm/Cargo.toml
cargo test --manifest-path demo-wasm/Cargo.toml --locked
cargo clippy --manifest-path demo-wasm/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path crates/framesmith-authoring/Cargo.toml --locked
npx vitest run src/lib/views/frameDataUtils.test.ts src/lib/training/FrameAdvantage.test.ts
npx playwright test --config demo-wasm/playwright.config.ts
```

On Windows/MSYS, set TMP/TEMP/TMPDIR to a native Windows temporary directory before invoking Cargo directly; `build.py` does this itself. `--packs-only` exports the binary and prepares the embedded authoring input. Playwright owns its local server, or set `FRAMESMITH_DEMO_URL` to exercise the public deployment. The export test uses the already-built local CLI to independently recompile the browser's downloaded ZIP.

`dist/` is the complete static browser application: serve it over HTTP at any directory prefix. It needs a JavaScript/WebAssembly-capable browser, not an API server, CDN, account, external asset host or installed editor. After startup, play and live authoring also work with the network disconnected; an offline cold reload is not promised. The downloaded project ZIP is combat authoring data, not a standalone copy of this application.

### Frozen acceptance (five gates)

1. Cold `/framesmith/` boot loads real WASM and one FSPK, not mocked services or character JSON requests; every served asset matches `build-info.json`. Cross-origin requests are blocked during this check, then play and an actual authoring rebuild are exercised offline.
2. The original lab fixture route fails, an actual control edit rebuilds data, and the corrected route produces measured links/cancels, resource spending and continuous hits; negative rules/conditions/spacing/costs remain negative.
3. Every manual trial clears through keyboard input; autoplay, wrong inputs, blocks and gaps cannot clear it. Phone multi-touch clears low-chain, reload and directional-super trials. Primary controls fit 1280×720 and 390×844.
4. Rich hits, globals/overrides, variants, notifies and properties are observable; the ZIP recompiles byte-identically, imported data behaves identically, invalid data preserves the prior run, and checkpoint/rewind/replay reproduce full consumer state.
5. **Publication gate:** the scoped Pages workflow builds/tests a fresh checkout; the exact public revision and hashes are read back and gameplay is exercised there. A green demo workflow does not certify unrelated repository CI or human taste.

`build-info.json` identifies the commit, dirty-source state and asset hashes. Generated WASM, binary packs, screenshots and archives remain ignored. The fixed 60-Hz playback is demo policy; the library is not tied to this clock, renderer, stage or input scheme. Recording is explicitly bounded at 3,600 simulation frames; retry starts a new recording. The lab supports this named kit and a 512 KiB import ceiling, not arbitrary unknown game projects.
