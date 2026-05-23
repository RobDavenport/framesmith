# Release Evidence 2026-05-23

Status: candidate evidence, not production-ready
Last reviewed: 2026-05-23

This record follows [`release-runbook.md`](release-runbook.md) for the current
production-readiness candidate.

## Candidate

```text
Candidate version: 0.1.0
Candidate branch: codex-production-readiness-plan
Candidate commit SHA: 51c3be4c5b5e4d67b093f0f7aaafc96ed244e26d
Target branch: main
Supported platforms for this candidate: Windows
Target game / integration: first production target contract in production-readiness-plan.md
Release owner: repository owner / maintainer
```

Branch:

```text
https://github.com/RobDavenport/framesmith/tree/codex-production-readiness-plan
```

Commit:

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
git diff --exit-code -- src/lib/wasm
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
cargo test --manifest-path crates/framesmith-runtime-wasm/Cargo.toml
cargo test --manifest-path crates/framesmith-fspack/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-runtime/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-runtime-wasm/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path crates/framesmith-fspack/Cargo.toml --all-targets -- -D warnings
npm run tauri build
git diff --check
```

Local package outputs:

```text
src-tauri/target/release/framesmith.exe
src-tauri/target/release/bundle/msi/Framesmith_0.1.0_x64_en-US.msi
src-tauri/target/release/bundle/nsis/Framesmith_0.1.0_x64-setup.exe
```

## GitHub CI State

Observed state on 2026-05-23:

```text
Candidate SHA: 51c3be4c5b5e4d67b093f0f7aaafc96ed244e26d
Branch pushed: yes
GitHub workflow runs for candidate SHA: none observed through the GitHub connector
Pull request: not created
Pull request creation attempt: GitHub connector returned 403 Resource not accessible by integration
```

The branch must still be opened as a pull request or otherwise run through
GitHub Actions before the clean-checkout CI gate can be marked complete.

## Branch Protection State

Observed state on 2026-05-23:

```text
Branch/ruleset evidence: not available through the current connector
Required CI evidence: not available
Blocked merge evidence: not available
```

The maintainer must configure or verify branch protection/rulesets requiring
the CI workflow before the branch-protection gate can be marked complete.

## Installer Smoke State

Observed state on 2026-05-23:

```text
Windows version: not recorded
Architecture: not recorded
MSI source: local build path exists
MSI result: not manually smoke tested
NSIS source: local build path exists
NSIS result: not manually smoke tested
Warnings: unsigned-build warning expected but not manually verified
```

Run [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) on a
Windows machine or clean VM before marking the installer gate complete.

## Decision

```text
Decision: not ready
Reason: GitHub clean-checkout CI, required-CI branch protection, and manual Windows installer smoke evidence are still missing.
```
