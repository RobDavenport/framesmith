# Release Evidence 2026-05-23

Status: candidate evidence, not production-ready
Last reviewed: 2026-05-23

This record follows [`release-runbook.md`](release-runbook.md) for the current
production-readiness candidate.

## Candidate

```text
Candidate version: 0.1.0
Candidate branch: codex-production-readiness-plan
Candidate branch head SHA before CI repair: fe6f44ae86fe4305301dd501d8fcdb0cb6874046
Latest observed passing CI SHA before this evidence update: ef14779e718a5ffd22666f9533f6b9023db42355
Local validation baseline SHA: 51c3be4c5b5e4d67b093f0f7aaafc96ed244e26d
Target branch: main
Supported platforms for this candidate: Windows
Target game / integration: first production target contract in production-readiness-plan.md
Release owner: repository owner / maintainer
```

Branch:

```text
https://github.com/RobDavenport/framesmith/tree/codex-production-readiness-plan
```

Local validation baseline commit:

```text
https://github.com/RobDavenport/framesmith/commit/51c3be4c5b5e4d67b093f0f7aaafc96ed244e26d
```

## Local Validation

These commands passed in the Windows workspace before pushing the branch:

```bash
npm ci
npm audit
npm ls @tauri-apps/api @tauri-apps/cli @tauri-apps/plugin-opener
cargo run --manifest-path src-tauri/Cargo.toml --bin generate_schema
npm run wasm:build
pwsh -NoProfile -Command "if (-not (Test-Path 'src/lib/wasm/framesmith_runtime_wasm.js')) { throw 'Missing generated WASM JavaScript binding' }; if (-not (Test-Path 'src/lib/wasm/framesmith_runtime_wasm.d.ts')) { throw 'Missing generated WASM TypeScript declarations' }"
git diff --exit-code -- schemas/rules.schema.json
npm run check
npm run test:run
npm run test:e2e
npm run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo fmt --check --manifest-path crates/framesmith-runtime/Cargo.toml
cargo fmt --check --manifest-path crates/framesmith-runtime-wasm/Cargo.toml
cargo fmt --check --manifest-path crates/framesmith-fspack/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path crates/framesmith-runtime/Cargo.toml
cargo run --manifest-path src-tauri/Cargo.toml --bin framesmith-cli -- export --project . --character test_char --adapter fspk --out exports/test_char.fspk
cargo test --manifest-path crates/framesmith-runtime-wasm/Cargo.toml
cargo test --manifest-path crates/framesmith-fspack/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-runtime/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-runtime-wasm/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-fspack/Cargo.toml --all-targets -- -D warnings
npm run tauri build
.\scripts\verify-windows-installer-artifacts.ps1 -Path src-tauri\target\release\bundle -Version 0.1.0
git diff --check
```

Local package outputs:

```text
src-tauri/target/release/framesmith.exe
src-tauri/target/release/bundle/msi/Framesmith_0.1.0_x64_en-US.msi
src-tauri/target/release/bundle/nsis/Framesmith_0.1.0_x64-setup.exe
```

## GitHub CI State

Observed state on 2026-05-23 after PR #1 was opened:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: fe6f44ae86fe4305301dd501d8fcdb0cb6874046
Branch pushed: yes
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26327908309
Workflow/job: CI / Windows Checks
CI status: failed
Failing step: TypeScript and Svelte check
Failure summary: missing generated module `$lib/wasm/framesmith_runtime_wasm.js`
```

Root cause: the workflow ran `npm run check` before `npm run wasm:build`.
`src/lib/wasm/` is generated and ignored, so a clean GitHub checkout does not
contain the WASM JavaScript or TypeScript bindings until the build step runs.

Repair action: the workflow now rebuilds the WASM package before frontend type
checking and explicitly verifies the generated JavaScript and TypeScript binding
files exist. The clean-checkout CI gate remains open until a repaired PR run
passes.

Second observed state on 2026-05-23 after the WASM-order repair:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: 333b1ba8d222267c10b7f9537be21a0a70060f75
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26328098854
Workflow/job: CI / Windows Checks
CI status: failed
Failing step: Test runtime WASM crate
Failure summary: missing ignored fixture `exports/test_char.fspk`
```

Root cause: `framesmith-runtime-wasm` test code includes
`exports/test_char.fspk` at compile time, but `exports/*.fspk` artifacts are
generated and ignored. The local workspace had the file from a previous export;
a clean GitHub checkout did not.

Repair action: the workflow now exports `characters/test_char` with
`framesmith-cli` before running the runtime WASM crate tests. The runbook records
the same fixture-generation step for clean local release verification.

Third observed state on 2026-05-23 after the fixture-generation repair:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: a568acfc81e4eb48390b8175fda49eb3bd1db1a0
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26328384684
Workflow/job: CI / Windows Checks
CI status: failed
Failing step: Test runtime WASM crate
Failure summary: integration tests still included ignored legacy fixture `exports/glitch.fspk`
```

Root cause: the runtime WASM integration tests referenced `exports/glitch.fspk`,
but no tracked `characters/glitch` source exists to regenerate that ignored
binary artifact in a clean checkout.

Repair action: the integration tests now use the generated `test_char.fspk`
fixture that CI builds from tracked `characters/test_char` data.

Passing observed state on 2026-05-23:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: 0b442b53ff827be491f61ba1c11eee1c3c386be3
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26328577057
Workflow/job: CI / Windows Checks
CI status: passed
Installer artifact: framesmith-windows-installers
Artifact ID: 7175845263
Artifact digest: sha256:bc208ae2bc18fd2435ed7b18079e37b2d00ec2c72a499eeaf601fe8de849c4f3
Artifact expires: 2026-08-21T08:52:31Z
```

Clean-checkout CI is now verified for this candidate SHA. The release remains
not production-ready until branch protection requires the CI gate and the Windows
installer artifact is manually smoke tested.

Fourth observed state on 2026-05-23 after an evidence-only commit:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: 577ffe6b835de8eb5ece0fcf524501a86f58cf68
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26329160394
Workflow/job: CI / Windows Checks
CI status: failed
Failing step: Browser smoke tests
Failure summary: Playwright smoke tests waited on transient text and short timeouts
```

Repair action: browser smoke tests now wait on stable frame-table and training
HUD elements, avoid asserting the transient initialization message, and give the
heavier smoke paths a 60-second test budget.

Passing observed state after browser-smoke hardening on 2026-05-23:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: 369e367a2715c9d67050faa9579be60aef0b7f35
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26329764725
Workflow/job: CI / Windows Checks
CI status: passed
Installer artifact: framesmith-windows-installers
Artifact ID: 7176181990
Artifact digest: sha256:ec363840dbfcf4c4b45d51e7199b10f4f60a6cfa5a597d8ffd7135eb0a906184
Artifact expires: 2026-08-21T09:56:05Z
```

Clean-checkout CI is verified for this observed candidate SHA. Before merging,
the latest PR head must still show a passing CI check. The release remains not
production-ready until branch protection requires the CI gate and the Windows
installer artifact is manually smoke tested.

Additional hardening after this run: the CI workflow now verifies that at least
one MSI and one NSIS setup executable exist and that all installer outputs are
non-empty before uploading the `framesmith-windows-installers` artifact. Shell
attempts to download and inspect artifact `7176298714` from the signed file URL
failed in this environment because PowerShell/curl could not complete the TLS
download; GitHub artifact metadata remains the authoritative evidence for the
remote artifact until manual smoke testing downloads it.

Fifth observed state on 2026-05-23 after adding installer-output verification:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: 4df4b57ee9a0066f0a669dea3bd7c95ad0b75883
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26330825628
Workflow/job: CI / Windows Checks
CI status: failed
Failing step: Verify Windows installer outputs
Failure summary: verifier treated single `FileInfo` values as arrays before concatenation
```

Repair action: the verifier now wraps both installer query results with `@(...)`
before concatenating them, so it handles the one-MSI/one-NSIS case observed in
CI while preserving the non-empty installer-output gate.

Latest passing observed state before this evidence update on 2026-05-23:

```text
Pull request: https://github.com/RobDavenport/framesmith/pull/1
Candidate SHA: ef14779e718a5ffd22666f9533f6b9023db42355
GitHub Actions run: https://github.com/RobDavenport/framesmith/actions/runs/26332001870
Workflow/job: CI / Windows Checks
CI status: passed
Installer artifact: framesmith-windows-installers
Artifact ID: 7176775382
Artifact digest: sha256:41245e80a68d4c02a3089d81cfafae6d86a981eb6cecdc07428eb2c553fff5ac
Artifact expires: 2026-08-21T11:55:22Z
```

This run passed the repaired `Verify Windows installer outputs` step and the
artifact upload. The release remains not production-ready until branch
protection requires the CI gate and the Windows installer artifact is manually
smoke tested.

## Branch Protection State

Observed state on 2026-05-23:

```text
Branch/ruleset evidence: not available through the current connector
Required CI evidence: not available
Blocked merge evidence: not available
Automation note: repository connector reports admin permission, but exposes no branch-protection/ruleset mutation; GitHub CLI is not installed; local Git credential lookup timed out without returning usable API credentials.
```

The maintainer must configure or verify branch protection/rulesets requiring
the `Windows Checks` job from the CI workflow before the branch-protection gate
can be marked complete. Follow
[`branch-protection-setup.md`](branch-protection-setup.md) and record the
resulting evidence here. `scripts/check-branch-protection.ps1` can validate an
authenticated branch-protection API response or a saved JSON response; CI
exercises the saved-response fixture so the helper does not drift silently.

## Installer Smoke State

Observed state on 2026-05-23:

```text
Windows version: not recorded
Architecture: not recorded
MSI source: local build path exists
CI installer artifact: framesmith-windows-installers, artifact ID 7176775382
MSI result: not manually smoke tested
NSIS source: local build path exists
NSIS result: not manually smoke tested
Warnings: unsigned-build warning expected but not manually verified
```

Run [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) on a
Windows machine or clean VM before marking the installer gate complete.
`scripts/verify-windows-installer-artifacts.ps1` can verify MSI/NSIS artifact
contents before the manual install/uninstall smoke flow. The CI installer
verification step uses this helper before uploading the artifact.

## Decision

```text
Decision: not ready
Reason: GitHub clean-checkout CI now passes, but required-CI branch protection and manual Windows installer smoke evidence are still missing.
```
