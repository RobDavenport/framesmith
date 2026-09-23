# FrameSmith Arena

[Play FrameSmith Arena](https://robdavenport.github.io/framesmith/).

A small playable validation game, not another engine. Four selectable fighters fight using real authored FSPK data and the Rust helper crates compiled to WebAssembly. No Tauri, backend, character JSON sidecar, CDN or external art is used by the deployed game.

## Four-corner expansion — frozen slice

Solo 1v1, first to two rounds: **Relay / shoto**, **Bulwark / grappler**, **Sable / zoner**, **Zip / rushdown**. Inspired by the middle and three corners of the supplied archetype triangle; not a frame-accurate SF2 or 2XKO port. Four action buttons **L / M / H / S**, hold away to block, down-back for lows, jump-ins for overheads, down+H launcher, down+S anti-air and H+S super. Keyboard J/K/L/U and WASD/arrows; the same four actions on touch. No tag-team system, motion-command parser or extra dependencies in this slice.

Acceptance: (1) all four authored binary fighters are selectable and mechanically distinct; (2) back/high/low blocking, projectile/jump/anti-air/grab counterplay and grounded/air chains work through production inputs; (3) KO, first-to-two rounds, rematch and seeded opponents work; (4) full-state replay includes air physics, facing, projectiles, combos and rounds; (5) the public static build is exercised with keyboard and touch, errors/layout checked, and game-first presentation inspected. Fun/balance remain a human verdict.

## Original slice

The duel exercises spacing, high/low/throw defense, projectiles, jumps, air chains, confirms/cancels, resources, rounds and rematch without a second engine or content pipeline. Target viewports: 1280×720 desktop and 390×844 portrait. Runtime diagnostics stay optional and below the playfield.

## Build and play

Prerequisites: the repository's Rust/native CLI toolchain, Python 3, `wasm32-unknown-unknown`, and `wasm-pack` (same toolchain as the existing WASM integration).

```sh
python demo-wasm/build.py
python demo-wasm/serve.py
```

Open `http://127.0.0.1:8080/framesmith/`. Production uses the identical static directory at the repository's GitHub Pages subpath.

`python demo-wasm/build.py --packs-only` validates/exports all four fighters using the **existing FrameSmith CLI**. Editable source is in `authoring/`; open that directory in the editor. Generated packs, WASM and site files remain ignored.

## Controls and checks

- Move: A/D or arrows; W/↑ jump, S/↓ crouch. J/K/L/U are Light/Medium/Heavy/Special. Hold away from the rival to block; down-away guards lows. Space also jumps.
- Down+H launches; down+S anti-airs; H+S spends 50 meter on a super. Confirm L → M → H → S and jump-cancel a confirmed launcher into L → M → H in the air.
- P/Escape pauses; R rematches. Touch controls are multi-touch.
- Runtime lab: single-step, save/restore a full checkpoint, verify every recorded state.
- Opponent selector: live seeded duel, stationary lab target, or guarding lab target.
- Collision overlays use decoded runtime rectangles. Sound is optional and starts muted.

```sh
cargo fmt --check --manifest-path demo-wasm/Cargo.toml
cargo clippy --manifest-path demo-wasm/Cargo.toml --all-targets --locked -- -D warnings
npx playwright test --config demo-wasm/playwright.config.ts
```

The build runs native consumer tests; browser checks use the real static site, actual keyboard/multi-touch input and the compiled WASM. Set `FRAMESMITH_DEMO_URL` to test the deployed site instead. `build-info.json` is an asset/commit manifest, **not character data**; it labels uncommitted local builds rather than attributing them to a clean commit.

`pages.yml` builds and checks the static artifact before deployment. It currently accepts `codex-production-readiness-plan` and `main`; this does not merge the binary-first audit into `main` or certify the wider editor audit. Source, lockfile and authored definitions are tracked; compiled files stay ignored.

## Ownership and limits

- FrameSmith: FSPK parsing/typed payloads, authored state identity/timing/shapes, cancel eligibility, atomic resource costs, resource definitions, hit and pushbox queries.
- This consumer: a solo 1v1 duel, integer simulation ticks, movement/air physics, guarding, projectiles, hit deduplication, health, stun/hitstop, resource rewards, counter-hit policy, seeded opponent, timer/KO, input buffering and presentation.
- The demo's fixed 60-Hz loop and stage coordinates are **not requirements of the library**. Jumping and cross-ups are demo-owned. No tag system, network play, external engine adapter or imported animation assets are claimed.
- Snapshot/replay includes the whole match: both helper states, positions and velocity, projectiles, health, buffers, previous inputs, hitstop, bot RNG/cooldown, rounds, events, statistics and terminal state. Replay compares every recorded frame by exact Rust state equality; it is not merely a matching final health bar. Recording is bounded by the three-round match.
- Browser code only supplies input and renders returned state. Combat is not reimplemented in JavaScript.

## Acceptance

1. A cold static page loads real `.fspk` and `.wasm` files under a repository URL prefix, with no editor/mock services or character JSON requests.
2. Real keyboard/touch actions move, hit, block, spend meter, cancel on contact, reach KO and restart; the seeded opponent can also win.
3. Displayed collision rectangles come from the same decoded shapes queried by the runtime, and denied/invalid input does not corrupt a match.
4. Mid-match checkpoint/restore and per-frame replay reproduce complete authoritative state, including bot decisions; focused native and browser checks exercise the production path.
5. Required play controls fit the target viewports; deployed artifact identity and the live Pages URL are checked. Fun and final human feel remain a human verdict.
