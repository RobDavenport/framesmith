# Production Gap Backlog

Status: active
Last reviewed: 2026-05-23

This backlog converts production-readiness gaps into concrete implementation
issues. A gap becomes release-blocking only when the target game requires
Framesmith, FSPK, or the runtime to own that behavior instead of accepting the
current `json-blob` handoff or engine-owned policy.

## How To Use This Backlog

For each production candidate:

1. Compare the target game's combat requirements with
   [`combat-coverage.md`](combat-coverage.md),
   [`training-scenario-contract.md`](training-scenario-contract.md), and
   [`export-fidelity-contract.md`](export-fidelity-contract.md).
2. Mark each relevant backlog item as required, deferred, or not applicable in
   the release notes or project issue tracker.
3. Implement required items with the tests listed under that item.
4. Update the export contract, training scenario contract, runtime docs, and
   this backlog before considering the item complete.

## External Release Gates

### PROD-CI-001: Clean-Checkout CI Certification

Trigger: every release candidate.

Scope:

- Push the exact candidate commit to GitHub.
- Let `.github/workflows/ci.yml` run on a fresh Windows runner.
- Download or inspect the uploaded `framesmith-windows-installers` artifact.

Required evidence:

- GitHub Actions run URL.
- Commit SHA.
- Pass/fail status.
- Artifact names and sizes.

Acceptance criteria:

- The CI workflow passes without local-only files or generated drift.
- Installer artifacts are produced by CI, not only by a developer workspace.

### PROD-CI-002: Required CI Before Merge

Trigger: before using the repository as a production source of truth.

Scope:

- Protect `main` or configure an equivalent ruleset.
- Follow [`branch-protection-setup.md`](branch-protection-setup.md).
- Require `Windows Checks` from the `CI` workflow before merge.
- Require branches to be up to date before merging.
- Require pull requests or another reviewed-change policy if the production
  game team uses shared branches.

Required evidence:

- Branch protection or ruleset screenshot/export.
- A blocked merge attempt or ruleset configuration showing CI is required.

Acceptance criteria:

- Production branches cannot accept changes without a green CI result.
- Release evidence names the protected branch or ruleset and records the
  blocked-merge proof.

### PROD-WIN-001: Windows Installer Manual Smoke

Trigger: every Windows release candidate.

Scope:

- Run [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) for
  MSI and NSIS installers.
- Record the evidence requested by that document.

Required evidence:

- Windows version and architecture.
- Installer source: local path or CI artifact URL.
- MSI result.
- NSIS result.

Acceptance criteria:

- Installed apps launch, load `TEST_CHAR`, start Training Mode, export, and
  uninstall cleanly.

## Export And Runtime Gaps

### FSPK-MOVE-001: Runtime-Owned Movement Export

Trigger: the target game requires FSPK/runtime, not `json-blob`, to own
movement curves, velocity, dash distance, launch, or forced movement.

Implementation scope:

- Add FSPK sections for movement data or a versioned FSPK v2 movement model.
- Add reader APIs in `framesmith-fspack`.
- Add runtime APIs that make ownership explicit.
- Update `docs/export-fidelity-contract.json`,
  [`movement-reference.md`](movement-reference.md), and
  [`runtime-guide.md`](runtime-guide.md).

Required tests:

- FSPK roundtrip tests for movement data.
- Runtime tests for authored movement behavior or documented pass-through
  ownership.
- Export contract tests proving movement is no longer silently omitted for the
  chosen adapter.

Acceptance criteria:

- A consuming engine can reconstruct the target game's movement policy from the
  exported data and runtime API without relying on undocumented side tables.

### FSPK-HIT-001: Advanced Hit Model Export

Trigger: the target game requires FSPK/runtime to own multi-hit attacks, chip
damage, block damage, per-hit reactions, or advanced hit metadata.

Implementation scope:

- Serialize `hits[]` or a versioned equivalent.
- Preserve chip damage and per-hit timing.
- Add FSPK reader views for the new data.
- Update [`export-fidelity-contract.md`](export-fidelity-contract.md) and
  [`combat-coverage.md`](combat-coverage.md).

Required tests:

- FSPK roundtrip tests for multi-hit attacks and chip damage.
- Runtime or WASM tests for authored hit/block behavior that uses the advanced
  hit model.
- Browser smoke coverage if editor-visible behavior changes.

Acceptance criteria:

- The runtime-facing handoff can distinguish every authored hit window that the
  target game requires.

### RUNTIME-THROW-001: Throw Collision And Tech Policy

Trigger: the target game requires Framesmith to own throw boxes, throw tech,
throw invulnerability, or throw-vs-strike priority.

Implementation scope:

- Add authoring fields for throw boxes and tech windows.
- Add FSPK serialization or document `json-blob` as the only supported handoff.
- Add runtime collision and priority APIs if runtime-owned.
- Add training scenarios before implementation.

Required tests:

- Schema validation tests for throw authoring fields.
- FSPK roundtrip tests if exported to FSPK.
- Runtime/WASM tests for throw priority and throw-tech outcomes.
- Training scenario tests for the target fixture.

Acceptance criteria:

- A throw cannot be confused with a strike purely because both share input
  notation or tags.

### RUNTIME-FREEZE-001: Hitstop And Blockstop Scheduling

Trigger: the target game requires Framesmith or the runtime to schedule
attacker/defender freeze, blockstop, rollback timing, or freeze-sensitive
events.

Implementation scope:

- Decide whether blockstop is a separate field or derived from hitstop.
- Serialize required data in the chosen handoff.
- Add runtime APIs for freeze state and frame stepping.
- Document rollback and event-ordering responsibilities.

Required tests:

- Schema tests for any new blockstop fields.
- FSPK roundtrip tests for freeze metadata if exported.
- Runtime tests for frame advancement during freeze.
- Training tests that prove hitstun/blockstun timing still matches authored
  data when freeze is active.

Acceptance criteria:

- The target game can reproduce the same freeze and reaction timing in training
  mode and in-engine integration.

### RUNTIME-RESOURCE-001: Resource Side-Effect Timing

Trigger: the target game requires Framesmith runtime to apply meter/resource
gain, costs, refunds, or on-hit/on-block/on-whiff deltas automatically.

Implementation scope:

- Define authoritative timing for resource side effects.
- Add runtime state mutation APIs or keep mutation engine-owned with examples.
- Update training scenarios for hit, block, whiff, and cancel costs.

Required tests:

- Runtime tests for resource costs and deltas at each supported timing point.
- WASM tests proving snapshots/restores include resource side effects.
- Export contract tests for any new serialized timing fields.

Acceptance criteria:

- Resource totals are deterministic across live play, training reset, and
  step-back/restore.

### RUNTIME-STAGE-001: Stage Bounds, Corners, And Push Resolution

Trigger: the target game requires Framesmith runtime to own stage bounds,
corner behavior, push priority, or collision clamping.

Implementation scope:

- Define stage/corner inputs to the runtime.
- Add deterministic push resolution APIs.
- Decide whether stage data belongs in project config, character data, or the
  consuming game.

Required tests:

- Runtime tests for pushbox collision against another actor and against stage
  bounds.
- Training tests for corner push behavior.
- Documentation examples showing engine integration.

Acceptance criteria:

- Pushbox behavior is deterministic and documented for rollback integration.

### RUNTIME-EVENT-001: Transition Events And Spawned Entities

Trigger: the target game requires Framesmith/FSPK/runtime to own
`on_use.enters_state`, projectile spawning, visual/audio event dispatch, or
status-effect application.

Implementation scope:

- Version event payloads and entity spawn data in the chosen handoff.
- Add runtime APIs for emitted actions or keep application engine-owned with a
  stable event stream.
- Update MCP and CLI examples if authoring workflows change.

Required tests:

- Schema and validation tests for event payloads.
- FSPK roundtrip tests if event payloads become runtime-owned.
- Runtime tests for transition or emitted-action ordering.
- Documentation examples showing how the engine consumes the event stream.

Acceptance criteria:

- The target game can replay state transitions and spawned effects from the
  exported handoff without undocumented interpretation rules.

## Platform Gaps

### PLATFORM-LINUX-001: Linux Package Support

Trigger: Linux becomes a supported release target.

Required work:

- Define package format and dependencies.
- Add CI package build.
- Add manual smoke-test steps matching the Windows installer smoke depth.
- Add artifact upload and release evidence.

Acceptance criteria:

- Linux is no longer just "builds on a developer machine"; it has repeatable
  CI artifacts and smoke-test evidence.

### PLATFORM-MAC-001: macOS Package Support

Trigger: macOS becomes a supported release target.

Required work:

- Define signing/notarization expectations.
- Add CI package build or documented manual build path.
- Add manual smoke-test steps matching the Windows installer smoke depth.
- Add artifact upload and release evidence.

Acceptance criteria:

- macOS support includes install, launch, load, training, export, and uninstall
  evidence.
