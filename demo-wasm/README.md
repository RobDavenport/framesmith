# FrameSmith Arena

[Play FrameSmith Arena](https://robdavenport.github.io/framesmith/).

A small playable validation game, not another engine. Two sparring bots fight using real authored FSPK data and the Rust helper crates compiled to WebAssembly. No Tauri, backend, character JSON sidecar, CDN or external art is used by the deployed game.

## Why a duel?

Considered: a timing/parry toy, a wave-survival arena, and a compact duel. The duel exercises the widest relevant slice without adding navigation, an entity framework or a content pipeline: spacing, active windows, hit/block confirmation, cancels, resources, KO and restart.

- **Player read:** approach, read the opponent's yellow windup, guard or punish recovery. Cyan is you; orange is the opponent.
- **Core change:** J starts a fast jab; cancel a confirmed hit into K, then L when meter permits. Whiffed attacks must recover.
- **Minimum cues:** attack poses and active arcs, hit/block flashes, health/meter, readable KO result; optional actual collision rectangles.
- **Target viewport:** one screen at 1280×720 desktop and 390×844 portrait; keyboard plus on-screen multi-touch controls.
- **Debug boundary:** optional diagnostics show authoritative state/frame, contacts, cancels, resource spending and replay results without replacing the playfield.

## Build and play

Prerequisites: the repository's Rust/native CLI toolchain, Python 3, `wasm32-unknown-unknown`, and `wasm-pack` (same toolchain as the existing WASM integration).

```sh
python demo-wasm/build.py
python demo-wasm/serve.py
```

Open `http://127.0.0.1:8080/framesmith/`. Production uses the identical static directory at the repository's GitHub Pages subpath.

`python demo-wasm/build.py --packs-only` validates/exports both fighters using the **existing FrameSmith CLI**. Editable source is in `authoring/`; open that directory in the editor. Generated packs, WASM and site files remain ignored.

## Controls and checks

- Move: A/D or arrows. J jab, K heavy, L burst (50 meter), hold Space to guard.
- P/Escape pauses; R starts a new bout. On-screen buttons support simultaneous touches.
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
- This consumer: a side-locked grounded duel, integer simulation ticks, movement, guarding, hit deduplication, health, stun/hitstop, resource rewards, counter-hit policy, seeded opponent, timer/KO, input buffering and presentation.
- The demo's fixed 60-Hz loop and stage coordinates are **not requirements of the library**. No jumping, cross-ups, network play, external engine adapter or imported animation assets are claimed.
- Snapshot/replay includes the whole match: both helper states, positions, health, buffers, previous inputs, hitstop, bot RNG/cooldown, events, statistics and terminal state. Replay compares every recorded frame by exact Rust state equality; it is not merely a matching final health bar. Recording is bounded by the one-minute round.
- Browser code only supplies input and renders returned state. Combat is not reimplemented in JavaScript.

## Acceptance

1. A cold static page loads real `.fspk` and `.wasm` files under a repository URL prefix, with no editor/mock services or character JSON requests.
2. Real keyboard/touch actions move, hit, block, spend meter, cancel on contact, reach KO and restart; the seeded opponent can also win.
3. Displayed collision rectangles come from the same decoded shapes queried by the runtime, and denied/invalid input does not corrupt a match.
4. Mid-match checkpoint/restore and per-frame replay reproduce complete authoritative state, including bot decisions; focused native and browser checks exercise the production path.
5. Required play controls fit the target viewports; deployed artifact identity and the live Pages URL are checked. Fun and final human feel remain a human verdict.
