# FrameSmith Combo Lab

[Open the lab](https://robdavenport.github.io/framesmith/).

**Change the rules. See exactly why a combo works. Take the data into your game.**
One character, one dummy, eight guided experiments, free play, and four manual combo trials. This replaces the platform-fighter landing page; the earlier `authoring/` fixtures remain untouched.

## First minute

Run the broken sequence. The dummy guards the gap. Shorten **Jab recovery** from 12 to 4 and run again: one link, two cancels, four uninterrupted hits. Then try the trials at quarter speed.

- **1 / 2 / 3 / 4** or the move buttons: Jab / Follow-up / Arc / Finisher.
- **P**: pause/play; **R**: retry; **.**: advance one frame.
- Slow motion and a three-simulation-frame input buffer are allowed in trials. Autoplay never grants trial credit.
- A trial requires ordered **actual unblocked contacts**, uninterrupted defender hitstun, and the specified link/cancel transitions. Wrong order, whiffs, blocks, expired input and gaps fail the attempt.
- Trial presets are isolated from the workshop instance. Returning to design preserves the draft and its recording.

## Feature coverage and ownership

1. **Timing and links:** edit recovery/startup; compare synchronized attacker/defender timelines. The frame table and editor timing utility use the same first-contact convention. A native test measures readiness on both actors and checks the displayed advantage, including shared hitstop. Reaction-state rows have no fictional attack advantage.
2. **Cancels:** enable/disable the actual tag rule, select hit/block/whiff/always, change its inclusive window, or add an explicit deny. Runtime eligibility is authoritative. The demonstration does not silently turn a denied cancel into a different link.
3. **Resources:** named energy/ammo pools, caps, starting values, requirements and atomic costs. The host applies the authored on-hit/on-use deltas. Reload explicitly starts that demonstration instance with empty ammo.
4. **Tags:** the custom `chainable` group covers both normals. Removing it affects both routes; move categories also produce implicit runtime tags and are not confused with this custom group.
5. **Contact:** actual FSPK AABB hit/hurt/push windows, spacing, reach and guard policies. Separate read-only circle/circle, capsule/capsule and AABB/circle probes call the real geometry helpers and expose the exact caller inputs. They are labeled queries, not alternate fighter collision or combo evidence.
6. **Events and extensibility:** frame notifies, typed event arguments, hit/use triggers, optional synthesized sound, and a separate character `spark_size` property consumed by presentation. Twin Pulse applies the two rich per-hit definitions, not repeated contact on every active frame.
7. **Reuse:** `globals/states/idle.json` is included with a per-character name override; `special~charged.json` inherits Arc. Source and resolved state can be inspected side by side. Full variant-overlay editing is not claimed.
8. **Validation and handoff:** a genuinely invalid resource reference is rejected by the shared validator without changing the current pack. Download the current FSPK or an editable project ZIP; compile that ZIP through the CLI and get identical binary bytes. Importing an edited pack preserves its actual data and locks unavailable overlay editing. A rejected import keeps the existing simulation.

Frame-data filtering/sorting, the rule graph, input history, forward/back stepping, complete checkpoints and per-frame replay are part of this screen. **What runs where?** distinguishes library behavior, consumer policy, retained data and editor/integration-only features. The native editor's sprite/GLTF asset workflow and CLI/MCP automation are not represented as a browser port. Movement, damage reactions, blocking, presentation, buffers, hit deduplication and trial policy belong to the consumer; the core is not a complete game engine.

## Real paths, not a JS imitation

`lab-authoring/` → existing CLI → `dist/packs/lab.fspk` → Rust/WASM runtime helpers.

Live edits use `framesmith-authoring`, a private portable build of the **same** schema, global resolver, variant flattener, rules/validators and exporters used by the desktop/CLI. Its library entrypoint is `src-tauri/src/portable.rs`; there is no copied exporter or new engine adapter. Compilation is transactional: construct and validate a replacement pack before replacing the current definition/world. Existing core runtime code and binary version remain unchanged.

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

### Frozen acceptance (five gates)

1. Cold `/framesmith/` boot loads real WASM and one FSPK, not mocked services or character JSON requests; every served asset matches `build-info.json`.
2. The initial route fails, an actual control edit rebuilds data, and the corrected route produces measured links/cancels, resource spending and continuous hits; negative rules/conditions/spacing/costs remain negative.
3. Every manual trial clears through keyboard input; autoplay, wrong inputs, blocks and gaps cannot clear it. Phone touch clears a trial. Primary controls fit 1280×720 and 390×844.
4. Rich hits, globals/overrides, variants, notifies and properties are observable; the ZIP recompiles byte-identically, imported data behaves identically, invalid data preserves the prior run, and checkpoint/rewind/replay reproduce full consumer state.
5. The scoped Pages workflow builds/tests a fresh checkout; the exact public revision and hashes are read back and gameplay is exercised there. A green demo workflow does not certify unrelated repository CI or human taste.

`build-info.json` identifies the commit, dirty-source state and asset hashes. Generated WASM, binary packs, screenshots and archives remain ignored. The fixed 60-Hz playback is demo policy; the library is not tied to this clock, renderer, stage or input scheme. Recording is explicitly bounded at 3,600 simulation frames; retry starts a new recording. The lab supports this named kit and a 512 KiB import ceiling, not arbitrary unknown game projects.
