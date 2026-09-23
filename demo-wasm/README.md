# FrameSmith OVERDRIVE

[Play the platform fighter](https://robdavenport.github.io/framesmith/).

A small four-fighter platform brawler using real authored FSPK data and FrameSmith's Rust runtime compiled to WebAssembly. No Tauri, backend, character JSON sidecar, CDN, extra engine or external art in the deployed game.

## Frozen prototype

**Question (LOOP + FEEL):** does a readable two-button platform fight explain how FrameSmith's authored moves, collision, cancels and resources work outside a traditional duel?

Current-project seam: `demo-wasm/` only; one bounded implementation/verification pass, then human play. Four styles: **Relay / all-rounder**, **Bulwark / heavyweight grappler**, **Sable / zoner**, **Zip / rushdown**. A large main platform, two raised side platforms and a higher center platform. Free movement, two jumps, one upward special recovery per airtime, percentage knockback, three stocks, seeded CPU and rematch. No body-push or opponent-tracking camera. No blocking, ledge grabs, dodge/shield, items, networking or full Smash simulation.

Player read: build damage, then launch the opponent across a dashed blast boundary; losing all three stocks loses the match. Percentage, stock pips, launch motion, hitstop and ring-out callouts explain the loop. The fixed camera and all ordinary controls fit **1280×720 desktop** and **390×844 portrait**. Runtime diagnostics are an optional bounded overlay, never required to play.

Provisional direction: **TRANSFORM** the earlier traditional duel. Keep signal: a fresh player can explain attacks, recover and finish a stock match without the lab. Repair knobs: knockback, jump/recovery strength and CPU aggression. Transform/stop if players still cannot read why they fell or what they can do. Automated checks are not a fun/balance verdict; fresh-player judgment remains open.

## Controls

- A/D or left/right: move and face that direction. W/up and S/down are attack modifiers, **not jump**.
- **Space: jump; press again to double jump.** Down + Space drops through a small platform. Hold down while falling to fall faster. Platforms catch on descent; the main deck cannot be dropped through.
- **J (or Z): Normal.** Neutral jab; repeated presses on contact chain stronger normals. Side + Normal hits hard, Up + Normal launches, Down + Normal attacks low. Aerial normals also respond to direction.
- **K (or X): Special.** Signature move; **Up + Special recovers upward once per airtime**, Down + Special spends 50 energy on a burst. Landing restores jumps/recovery. Touch uses the same directional controls, Jump, Normal and Special.
- P/Escape pauses; R rematches. VS CPU or Practice (idle rival).
- Runtime lab: step, full checkpoint/restore and per-frame replay. Boxes display actual decoded collision rectangles. Optional sound starts after interaction.

## Build and play

Prerequisites: the repository's Rust/native CLI toolchain, Python 3, `wasm32-unknown-unknown` and `wasm-pack` (same as the existing WASM integration).

```sh
python demo-wasm/build.py
python demo-wasm/serve.py
```

Open `http://127.0.0.1:8080/framesmith/`. Production uses the identical static directory under GitHub Pages' repository subpath. `python demo-wasm/build.py --packs-only` validates/exports all four fighters through the existing CLI. Editable source is in `authoring/`; open that directory in the editor. Generated packs, WASM and site files stay ignored.

```sh
cargo fmt --check --manifest-path demo-wasm/Cargo.toml
cargo test --manifest-path demo-wasm/Cargo.toml --locked
cargo clippy --manifest-path demo-wasm/Cargo.toml --all-targets --locked -- -D warnings
npx playwright test --config demo-wasm/playwright.config.ts
```

The build runs native consumer tests. Browser checks load the compiled WASM and real packs and use keyboard/multi-touch controls; `FRAMESMITH_DEMO_URL` targets deployment. `build-info.json` identifies assets/source, **not character data**, and labels dirty local builds. `pages.yml` builds and checks before deploying from `codex-production-readiness-plan` or `main`; this neither merges the audit nor certifies broader repository CI.

## Ownership and finite acceptance

FrameSmith owns FSPK validation and payloads, authored identity/timing/shapes, cancel eligibility, resource definitions/costs and hit queries. This demo owns the stage, fixed-tick movement/gravity, damage/knockback, stocks, projectiles, hit deduplication, buffers, bot, timer and presentation. JavaScript supplies input and renders the returned state; it does not duplicate combat. The fixed 60-Hz tick and this arena are not library requirements.

1. All four distinct binary-authored fighters load at a cold repository subpath; no character JSON or mock backend is fetched.
2. Normal/Special and directional input, free movement/crossovers, two jumps, all platform levels, dropping and offstage recovery work through the public inputs.
3. Hits increase percentage and scale knockback; stock loss resets percentage, respawns protect the fighter, three lost stocks end the match, and rematch resets. CPU engages and can lose/win stocks.
4. Checkpoint and per-frame replay exactly reproduce the full Rust world: helper states, movement, platforms, jumps/recovery, percentages/stocks, respawn, projectiles, resources, inputs/buffers, hitstop, RNG and result. Recording is bounded by the three-minute timer.
5. The ordinary loop fits both target viewports, is exercised at real speed without browser errors, and the deployed asset revision is read back and tested. Legibility and fun remain human judgments, not a claim from automated tests.
