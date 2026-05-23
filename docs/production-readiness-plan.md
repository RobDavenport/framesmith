# Production Readiness Plan

Status: active
Last reviewed: 2026-05-23

This document is the durable checklist for deciding when Framesmith can be used
as a production character-authoring pipeline for a real fighting game.

Current assessment: Framesmith is no longer blocked by the build and schema
drift found in the initial audit. The current workspace builds, tests, rebuilds
WASM from source, runs browser smoke tests, produces Windows packages, and has
an executable first-target training scenario contract. It is still not finished
as a production game-development pipeline because branch protection and manual
platform smoke testing remain open.

PR #1 produced one clean-checkout CI failure on 2026-05-23 because the workflow
ran frontend type checking before rebuilding ignored WASM bindings. The workflow
has been corrected to build and verify WASM before `npm run check`; the
clean-run gate remains open until GitHub Actions passes on the repaired branch.
The next PR run passed that gate and exposed a second clean-checkout assumption:
runtime WASM tests needed ignored `exports/test_char.fspk` output. CI and the
release runbook now generate that fixture from tracked character data before the
runtime WASM crate test. A follow-up run showed the same test crate still
referenced the ignored legacy `exports/glitch.fspk`; those integration tests now
use the generated `test_char.fspk` fixture instead. Run `26328577057` passed on
GitHub for candidate SHA `0b442b53ff827be491f61ba1c11eee1c3c386be3` and
uploaded the `framesmith-windows-installers` artifact. Later PR runs exposed
browser-smoke timing flakiness and a Windows installer verifier bug in the
single-MSI/single-NSIS case; the smoke tests now wait on stable frame-table and
training HUD elements, and the verifier wraps both installer query results
before concatenation. The latest code-bearing PR run observed before this
documentation update, `26332001870`, passed for candidate SHA
`ef14779e718a5ffd22666f9533f6b9023db42355` and uploaded installer artifact
`7176775382`. Check the latest PR head before merge because evidence-only
commits can trigger another CI run.

## Readiness Definition

Framesmith is production ready when all of these are true:

- The editor, backend, runtime crates, WASM runtime, CLI, MCP server, and
  packaged desktop app build from source on supported platforms.
- Frontend types, Rust schema types, docs, generated schemas, generated WASM,
  and sample project data agree on the same data model.
- Exported data either preserves every game-facing field, or each intentional
  adapter limitation is documented, tested, and accepted by the target game.
- Training mode validates real authored data and covers common fighting-game
  states such as idle, crouch, jump, blockstun, hitstun, cancels, resources,
  throws, and reset behavior.
- CI runs Rust tests, TypeScript tests, Svelte checks, clippy, WASM builds,
  browser smoke tests, app builds, and export compatibility checks.
- Documentation examples work as written.

## Current Snapshot

Verified on 2026-05-23 in the current Windows workspace:

| Check | Result | Notes |
|-------|--------|-------|
| `npm ci` | Pass | Reinstalls the locked frontend/tooling dependency graph. |
| `npm audit` | Pass | Frontend/tooling dependency audit reports 0 vulnerabilities after dependency refresh and the narrow `cookie@0.7.2` override. |
| Tauri npm/Rust package alignment | Pass | `@tauri-apps/api`, `@tauri-apps/cli`, and `@tauri-apps/plugin-opener` are pinned to the Rust-aligned 2.9/2.5 minor lines; `npm run tauri build` verifies the match. |
| `npm run check` | Pass | `svelte-check` reports 0 errors and 0 warnings. |
| `npm run test:run` | Pass | 204 Vitest tests pass. |
| `npm run test:e2e` | Pass | 4 Playwright smoke tests cover project load, editor save, variant read-only behavior, cancel graph, globals, export, embedded training startup, separate dummy character/FSPK selection, and detached training startup through BroadcastChannel sync. |
| `npm run wasm:build` | Pass | Rebuilds `src/lib/wasm/` from `crates/framesmith-runtime-wasm`. |
| `cargo run --manifest-path src-tauri/Cargo.toml --bin generate_schema` | Pass | Refreshes `schemas/rules.schema.json`; the audit found and accepted generated schema drift. |
| `npm run build` | Pass | Production web build succeeds with rebuilt WASM. |
| `npm run test:run -- src/lib/views/training/TrainingLoop.test.ts src/lib/training/buildMoveList.test.ts src/lib/training/InputManager.test.ts` | Pass | 43 targeted training tests cover dummy behavior propagation, hit/block damage, combo reset, push separation, authored movement, throw inputs, and input mapping. |
| `npm run tauri build` | Pass | Produces `framesmith.exe`, MSI, and NSIS installer on Windows. |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Pass | Backend, schema, codegen, globals, MCP, and pipeline tests pass, including 26 MCP command tests. |
| `cargo test --manifest-path src-tauri/Cargo.toml --test docs_cli_examples` | Pass | Executes documented `framesmith-cli` export examples against a temporary project. |
| `cargo test --manifest-path src-tauri/Cargo.toml --test export_fidelity_contract` | Pass | Contract covers schema fields, maps every preserved/derived FSPK field to named roundtrip tests, and keeps lossy examples documented. |
| `cargo test --manifest-path src-tauri/Cargo.toml --test fspk_roundtrip` | Pass | 21 FSPK reader roundtrip tests cover preserved/derived contract fields. |
| `cargo test --manifest-path src-tauri/Cargo.toml --test production_docs` | Pass | Keeps production docs linked, variant deferral documented, and temporary plans migrated. |
| `cargo test --manifest-path crates/framesmith-runtime/Cargo.toml` | Pass | Runtime unit and integration tests pass. |
| `cargo test --manifest-path crates/framesmith-runtime-wasm/Cargo.toml` | Pass | WASM wrapper tests pass, including first-target hitstun/blockstun/resource/throw scenario checks against `exports/test_char.fspk`. |
| `cargo test --manifest-path crates/framesmith-fspack/Cargo.toml` | Pass | no_std FSPK reader tests pass. |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Pass | CI-level backend clippy gate is clean. |
| Runtime crate clippy with `-D warnings` | Pass | `framesmith-runtime`, `framesmith-runtime-wasm`, and `framesmith-fspack` are clean. |
| Cargo fmt checks | Pass | Formatting is clean for `src-tauri` and runtime crates. |
| CI workflow | Pass | `.github/workflows/ci.yml` passed on GitHub Actions run `26332001870`, checking dependency audit, generated WASM, generated schemas, runtime WASM FSPK fixture generation, formatting, frontend, browser smoke tests, Rust tests, backend/runtime clippy, Tauri packaging, non-empty Windows installer outputs, and Windows installer artifact upload; branch protection still must be configured in GitHub. |

Clean-checkout CI is verified for candidate SHA
`ef14779e718a5ffd22666f9533f6b9023db42355`. Required-branch enforcement and
manual installer smoke testing are still open; if another commit lands, the new
PR head must also pass CI.

Current candidate evidence is recorded in
[`release-evidence-2026-05-23.md`](release-evidence-2026-05-23.md). The
candidate branch `codex-production-readiness-plan` is open as PR #1. The first
GitHub Actions run failed because generated WASM was not built before
type-checking. The second run passed that gate and failed because the runtime
WASM crate expected an ignored FSPK fixture. The workflow and runbook now
generate the fixture before runtime WASM tests. A third run exposed a remaining
legacy `glitch.fspk` include in the same test crate; the test now uses the
generated `test_char.fspk` fixture. The next run passed and uploaded installer
artifact `framesmith-windows-installers`. A subsequent evidence-only commit
exposed browser-smoke timing flakiness; the tests were hardened and current-head
run `26329764725` passed. A later installer-output verification hardening commit
failed because the verifier did not handle single `FileInfo` values, then run
`26332001870` passed after wrapping both installer query results before
concatenation.

## Completed Since Audit

- Restored runtime cancel availability APIs and rebuilt WASM from source.
- Aligned frontend types and views with property-based characters plus
  `tag_rules`/`deny` cancel tables.
- Added fixture tests so schema drift against `characters/test_char` fails fast.
- Normalized binary adapter naming to `fspk`, while retaining `zx-fspack` as a
  legacy alias.
- Updated CLI, MCP, README, and docs to use `framesmith-cli` and `fspk`.
- Added Windows CI covering frontend checks, Vitest, WASM rebuild, Playwright,
  web build, Rust tests, clippy, Tauri packaging, generated-file checks, and
  Windows installer artifact upload.
- Added Playwright smoke tests that generate real FSPK and JSON fixtures through
  the Rust CLI, then exercise the editor against those resolved outputs.
- Fixed State Editor and save-path Svelte proxy serialization bugs found by the
  smoke suite.
- Fixed the detached Training Mode route so BroadcastChannel data refreshes do
  not retrack mutable session state and repeatedly recreate the WASM session.
- Added generated-output ignores for Playwright artifacts.
- Refreshed `schemas/rules.schema.json` from the Rust schema generator.
- Updated frontend/tooling dependencies within existing semver ranges and added
  a narrow `cookie@0.7.2` override so `npm audit` reports 0 vulnerabilities.
- Pinned Tauri npm packages to the Rust-aligned minor versions after the
  dependency refresh exposed a package-build mismatch.
- Brought clippy to zero warnings for the backend and runtime crates.
- Added [`schema-migration.md`](schema-migration.md) with migration examples for
  character properties, cancel tables, variants, and the `fspk` adapter rename.
- Added [`training-scenario-contract.md`](training-scenario-contract.md) with
  executable first-target training scenarios and named test evidence.
- Added [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) so
  the remaining manual Windows installer gate has repeatable steps and evidence
  to record.
- Added [`branch-protection-setup.md`](branch-protection-setup.md) with the
  exact `Windows Checks` requirement and evidence template for the remaining CI
  enforcement gate.
- Added reusable PowerShell evidence helpers for branch-protection API
  verification and MSI/NSIS artifact integrity checks.

## Remaining Production Blockers

### 1. Clean-Checkout And CI Enforcement

Status: GitHub clean-run complete; branch protection remains external.

Automation note: the available GitHub connector reports admin permission on the
repository but does not expose branch-protection or ruleset mutation. The local
environment does not have `gh`, and Git credential lookup timed out, so required
CI enforcement still needs to be configured in GitHub settings or through an
authenticated API client.

Actions:

- Follow [`branch-protection-setup.md`](branch-protection-setup.md) and require
  `Windows Checks` from the `CI` workflow before merges to `main`.
- Keep Windows as the first supported packaging target.
- Keep Linux and macOS out of the first production target until platform
  dependencies, package formats, and manual smoke-test steps are documented.
- Track external release gates through
  [`production-gap-backlog.md`](production-gap-backlog.md) and execute
  [`release-runbook.md`](release-runbook.md) for each release candidate.

Acceptance criteria:

- A fresh CI runner passes the complete workflow.
- Protected branches reject merges without green CI.
- Windows installer artifacts are produced by CI, not only locally.
- Linux and macOS are either verified or explicitly not supported for the
  release target.

### 2. Full-Fidelity Export Contract

Status: complete for the first production target; future target-specific
scenarios are tracked in [`production-gap-backlog.md`](production-gap-backlog.md).

Original risk:

- `json-blob` is the most complete authoring handoff.
- `fspk` is compact and runtime-friendly, but it is still a runtime subset.
- FSPK field loss needed an explicit contract so omitted fields were deliberate
  adapter policy rather than silent export drift.

Completed:

- Added `docs/export-fidelity-contract.json` as the machine-readable field
  classification contract for `json-blob` and `fspk`.
- Added `docs/export-fidelity-contract.md` with adapter policy and known FSPK
  v1 limits.
- Added Rust tests that compare the contract against current direct
  `Character`, `State`, and `CancelTable` schema fields and reject unknown or
  unexplained statuses.
- Added a contract coverage audit requiring every `preserved` or `derived`
  FSPK field to name at least one roundtrip test through `framesmith-fspack`.
- Added missing roundtrip coverage for character-id-derived mesh keys,
  character properties, and `on_block` events/resource deltas.
- Added explicit lossy FSPK v1 examples for resolved variant identity,
  advanced multi-hit data, movement, advanced hurtbox flags, and super freeze.
- Formalized `json-blob` as the canonical first production handoff with `fspk`
  v1 as an optional runtime subset/cache.

Maintenance triggers:

- Keep fixture coverage for tags, cancel rules, properties, resources, events,
  pushboxes, hit windows, and schema sections.
- When an omitted or engine-owned field becomes required by a target game, open
  or implement the matching item from
  [`production-gap-backlog.md`](production-gap-backlog.md) and add the listed
  tests before changing the export contract status.

Acceptance criteria:

- No field can be silently lost during export.
- Every intentional adapter limitation has a test and a documentation entry.

### 3. Variant Identity And Editing Semantics

Status: complete for the current first-target scenario contract; future target
games may add stricter scenarios.

Problem:

- Variant overlays resolve with unique `id` values but may share the same
  gameplay `input`.
- Editor selection and graph nodes now use `State.id` when present and fall
  back to `input`.
- Resolved variants are read-only in the current State Editor and through
  `save_move` because loaded variants are resolved snapshots, not overlay diffs.

Completed:

- Added shared frontend state identity helpers.
- Updated State Editor and Frame Data selection to distinguish base states from
  variants.
- Updated Cancel Graph nodes and deny matching to distinguish resolved variant
  IDs from shared gameplay inputs.
- Blocked `save_move` for resolved variants so a variant cannot overwrite its
  base state.
- Added unit, backend, and browser smoke coverage for variant identity/read-only
  behavior.
- Added [`variant-editing-decision.md`](variant-editing-decision.md), which
  explicitly keeps variants as JSON-authored overlays for the first production
  target and defines when overlay-aware UI editing must be reopened.

Maintenance triggers:

- Reopen overlay-aware variant editing only if the target game rejects JSON-only
  overlay authoring or needs non-programmer variant editing in the UI.
- If reopened, implement direct overlay-file editing, inherited/overridden field
  display, and tests proving a fully resolved state cannot be serialized back
  into an overlay patch.

Acceptance criteria:

- For the first production target, a game team can author variants as JSON
  overlays, inspect resolved variants in the editor, export and test them, and
  cannot accidentally overwrite a base move or another variant through the
  current UI/MCP save path.

### 4. Movement, Collision, And Engine Boundaries

Status: complete for the first production target ownership contract; future
runtime-owned movement work is tracked in
[`production-gap-backlog.md`](production-gap-backlog.md).

Problem:

- Movement authoring exists.
- Runtime docs currently put stage bounds, corners, full movement simulation,
  and some resource/effect application on the consuming game engine.
- FSPK movement support and runtime movement ownership need a hard contract.

Completed:

- Added a runtime ownership contract to `docs/runtime-guide.md`.
- Updated `docs/runtime-api.md` with `PushboxResult` and `check_pushbox()`.
- Updated `docs/movement-reference.md` to state that `json-blob` preserves
  movement while `fspk` v1 does not serialize movement values.
- Linked movement/export limitations through the export-fidelity contract.
- Formalized movement as `json-blob` only for FSPK v1 in
  `docs/production-handoff-decision.md`.
- Added engine-consumption examples for applying `json-blob` movement and FSPK
  resource deltas in `docs/runtime-guide.md`.

Maintenance triggers:

- Add roundtrip/export tests for any movement data that becomes runtime-owned.
- Use `FSPK-MOVE-001` from
  [`production-gap-backlog.md`](production-gap-backlog.md) if a target game
  requires movement in the compact runtime handoff.

Acceptance criteria:

- The runtime guide tells an engine implementer exactly what Framesmith provides
  and exactly what the engine must supply.
- If movement becomes runtime-owned, FSPK movement export and runtime tests exist.

### 5. Combat Data Coverage

Status: complete for first-target classification; target-required engine-owned
mechanics are tracked in [`production-gap-backlog.md`](production-gap-backlog.md).

Completed:

- Added `docs/combat-coverage.md` with a mechanic-by-mechanic production
  support matrix.
- Classified current coverage for chip damage, multi-hit attacks, throws,
  projectiles/entities, hitstop, blockstop, forced movement, resource gain,
  status effects, and state transition events.

Maintenance triggers:

- Turn target-game-required `engine-owned` or `exported as data only` mechanics
  into concrete FSPK/runtime implementation issues.
- Add roundtrip/runtime tests when a mechanic moves to `supported end-to-end`.
- Use [`production-gap-backlog.md`](production-gap-backlog.md) as the canonical
  issue source for those implementation decisions.

Acceptance criteria:

- A production game can decide whether Framesmith already covers its combat
  model or which engine glue/FSPK v2 work is required.

### 6. Training Mode Maturity

Status: complete for the current first-target training contract; future
runtime-owned policies are tracked in
[`production-gap-backlog.md`](production-gap-backlog.md).

Completed:

- Training mode loads in browser smoke tests from rebuilt WASM and real CLI
  exported FSPK data.
- Runtime, WASM wrapper, input, cancel, dummy-controller, frame-advantage, and
  render-mapping unit tests pass.
- Added WASM `TrainingSession.snapshot()`/`restore()` support for deterministic
  browser training rewind.
- Added bounded step-back history for embedded and detached training mode,
  restoring visible loop state, WASM session state, and input-buffer state.
- Added Vitest coverage for input-buffer snapshots and TrainingLoop step-back,
  reset, and empty-history behavior.
- Updated the WASM training wrapper so dummy stand/crouch/jump/block behavior
  resolves authored FSPK states by input/tag lookup instead of fixed indices
  when the pack provides those states.
- Added blocked-hit reporting to WASM `HitResult` and TrainingLoop chip-damage
  handling for blocking dummy modes.
- Added throw-button (`T`) input support for authored inputs such as `5T`.
- Added TrainingLoop behavior coverage for dummy behavior propagation,
  hit/block damage, combo reset, push separation, authored movement, and throw
  input resolution.
- Added embedded Training Mode dummy-character selection that loads separate
  dummy character data and FSPK bytes before creating the two-pack WASM
  `TrainingSession`.
- Extended Playwright smoke coverage so Training Mode switches to a separate
  dummy character/FSPK in the embedded view.
- Added detached route browser smoke coverage through BroadcastChannel sync and
  real exported FSPK bytes.
- Fixed the detached route reinitialization loop that could blank the session
  after startup when reactive state changes retriggered initialization.
- Added first-target WASM scenario coverage proving authored hitstun/blockstun
  state routing plus resource/throw policy preservation for `test_char`.
- Added [`training-scenario-contract.md`](training-scenario-contract.md) as the
  documented minimum training behavior contract for the first production target.

Maintenance triggers:

- Add new scenario rows and tests before making Framesmith own currently
  engine-owned policies such as throw tech, throw priority, hitstop scheduling,
  resource side-effect timing, or stage/corner behavior.
- Link each new scenario to the matching
  [`production-gap-backlog.md`](production-gap-backlog.md) item.

Acceptance criteria:

- Training mode demonstrates real authored behavior, not just startup success.

### 7. Documentation And Release Hygiene

Status: local documentation complete; release-candidate evidence remains tied
to external CI and installer smoke gates.

Completed:

- Added integration coverage that executes the documented `framesmith-cli`
  export examples against a temporary project and parses the generated FSPK.
- Fixed the stale `AGENTS.md` CLI export command to use `framesmith-cli`.
- Migrated completed temporary `docs/plans/` content into
  [`implementation-history.md`](implementation-history.md) and permanent docs,
  then removed the stale plan files.
- Added [`schema-migration.md`](schema-migration.md) for the current schema
  migration path and release-candidate verification commands.
- Added [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) for
  MSI/NSIS release-candidate smoke testing.
- Added [`production-gap-backlog.md`](production-gap-backlog.md) and
  [`release-runbook.md`](release-runbook.md) so future production gaps and
  release-candidate evidence have permanent homes.
- Added [`branch-protection-setup.md`](branch-protection-setup.md) for exact
  required-check configuration and branch-protection evidence capture.
- Added [`release-evidence-2026-05-23.md`](release-evidence-2026-05-23.md) for
  the current candidate branch and external release-gate evidence.

Maintenance triggers:

- Keep docs updated after each completed item.
- Run the release checklist below for every candidate release.
- Record release evidence with [`release-runbook.md`](release-runbook.md).

Acceptance criteria:

- New contributors can follow docs without discovering command drift.
- Release artifacts are reproducible from a clean checkout.

## Release Checklist

The checklist below is the reusable template for a tagged release. For the
current `0.1.0` candidate, the machine-verifiable items are covered by the
current snapshot and GitHub Actions evidence above. The only unresolved
candidate items are branch protection enforcement and manual Windows installer
smoke testing.

Current candidate release-checklist evidence:

| Area | Current candidate status | Evidence |
|------|--------------------------|----------|
| Version metadata | Pass | `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` all declare `0.1.0`. |
| Dependency install, audit, and Tauri package alignment | Pass | Current snapshot records `npm ci`, `npm audit`, and Tauri package alignment; CI run `26332439997` passed dependency install and audit. |
| Generated schemas and WASM bindings | Pass | CI run `26332439997` passed schema refresh, generated-file drift check, WASM rebuild, and generated WASM existence checks. |
| Frontend checks, Vitest, Playwright, and web build | Pass | CI run `26332439997` passed `npm run check`, `npm run test:run`, browser smoke tests, and `npm run build`. |
| Rust formatting, tests, and clippy | Pass | CI run `26332439997` passed formatting, backend/runtime/FSPK tests, runtime WASM fixture generation, and clippy with warnings denied. |
| Windows Tauri package build and artifact upload | Pass | CI run `26332439997` passed `npm run tauri build`, installer-output verification, and uploaded `framesmith-windows-installers`. |
| Documentation examples and export limitations | Pass | `docs_cli_examples`, `export_fidelity_contract`, `fspk_roundtrip`, and `production_docs` tests cover documented CLI examples, field preservation, lossy examples, and release docs. |
| Target-game fit review | Pass for the first production target | `production-handoff-decision.md`, `combat-coverage.md`, `training-scenario-contract.md`, and `production-gap-backlog.md` document current ownership and future target-game gaps. |
| Branch protection | Open external gate | Follow `branch-protection-setup.md`; no branch/ruleset evidence has been recorded yet. |
| Windows installer smoke | Open external gate | Run `windows-installer-smoke-test.md` against the MSI and NSIS installers and record evidence; `scripts/verify-windows-installer-artifacts.ps1` verifies artifact contents before install. |
| Linux and macOS smoke | Not in first target scope | Both platforms are explicitly out of scope for the first supported target. |

Run this checklist before a tagged release:

- [ ] Version bump in package metadata and Tauri config.
- [ ] `npm ci` from a clean checkout.
- [ ] `npm audit`.
- [ ] `npm ls @tauri-apps/api @tauri-apps/cli @tauri-apps/plugin-opener`.
- [ ] `cargo run --manifest-path src-tauri/Cargo.toml --bin generate_schema`.
- [ ] Generated JSON schemas committed with no unexpected drift.
- [ ] `npm run wasm:build`.
- [ ] Generated WASM JavaScript and TypeScript bindings exist.
- [ ] `npm run check`.
- [ ] `npm run test:run`.
- [ ] `npm run test:e2e`.
- [ ] `npm run build`.
- [ ] `cargo fmt --check` for `src-tauri` and runtime crates.
- [ ] Runtime WASM test fixture generated with `framesmith-cli`.
- [ ] `cargo test` for `src-tauri`, `framesmith-runtime`,
  `framesmith-runtime-wasm`, and `framesmith-fspack`.
- [ ] `cargo clippy --all-targets -- -D warnings` for the backend and runtime
  crates.
- [ ] `npm run tauri build`.
- [ ] Installer smoke test on Windows.
- [ ] Linux package smoke test, if Linux is supported.
- [ ] macOS package smoke test, if macOS is supported.
- [ ] Documentation examples checked against current commands.
- [ ] Known export limitations reviewed against the target game.

## Recommended Execution Order

1. Run the new CI workflow on a clean runner and enforce it before merges.
2. Keep the canonical export contract and export-fidelity reporting current.
3. Review [`production-gap-backlog.md`](production-gap-backlog.md) with the
   target game's required mechanics.
4. Reopen overlay-aware variant editing only if the target game rejects
   JSON-only overlay authoring.
5. Implement movement/combat gaps only when the target game rejects the current
   engine-owned/json-blob handoff.
6. Keep the training scenario contract current when a game adds stricter
   hitstun/blockstun/resource/throw policies.
7. Package and smoke test every supported platform.

## Production Readiness Checklist

- [x] WASM source builds in the verified workspace.
- [x] Generated WASM artifacts are reproducible in the verified workspace.
- [x] Frontend types match the current Rust/data schema for audited fields.
- [x] Current sample project loads in browser smoke tests.
- [x] Cancel graph supports `tag_rules` and `deny`.
- [x] Character overview supports property-based characters.
- [x] Binary adapter names are consistent in docs, UI, CLI, MCP, and code.
- [x] Backend and runtime clippy checks have zero warnings.
- [x] CI workflow exists.
- [x] Browser smoke tests cover core editor workflows.
- [x] FSPK limitations are documented at a high level.
- [x] Training mode rebuilds from source and passes startup smoke coverage.
- [x] Release checklist exists.
- [x] Windows package builds in the verified workspace.
- [x] Variant selection and save blocking are data-loss-safe.
- [x] Export adapter field classifications are machine-checked against the Rust schema.
- [x] Runtime ownership boundaries are documented.
- [x] Combat mechanic coverage is classified.
- [x] Training mode step-back restores runtime, loop, and input-buffer state.
- [x] Documented `framesmith-cli` export examples are executed in automation.
- [x] Every `preserved`/`derived` FSPK field has named roundtrip coverage.
- [x] Intended lossy FSPK export cases have explicit documented examples.
- [x] Canonical first production handoff is documented as `json-blob` plus optional FSPK cache.
- [x] FSPK movement ownership is explicitly accepted as json-blob-only for the first production target.
- [x] Runtime guide includes engine-consumption examples for movement and resource deltas.
- [x] Production gap backlog exists for target-game-required runtime/FSPK work.
- [x] Release runbook exists for clean-checkout, CI, branch-protection, and installer evidence.
- [x] Branch protection setup is documented with the exact required CI check.
- [x] Branch protection and installer artifact evidence helpers exist.
- [x] Current candidate release evidence is recorded.
- [x] Frontend/tooling dependency audit reports 0 vulnerabilities in the verified workspace.
- [x] Tauri npm packages are pinned to the Rust-compatible minor line.
- [x] Clean-checkout CI run has passed.
- [ ] CI is required before merges.
- [x] Overlay-aware variant editing is implemented or explicitly deferred for
  the target game.
- [x] Target-game training scenarios cover hitstun/blockstun/resource/throw
  policies.
- [x] Stale temporary plans are migrated or removed.
- [ ] Windows installer is manually smoke tested.
- [x] Linux package is out of scope for the first supported target.
- [x] macOS package is out of scope for the first supported target.
