# Release Runbook

Status: active
Last reviewed: 2026-05-23

Use this runbook with
[`production-readiness-plan.md`](production-readiness-plan.md) before tagging a
Framesmith release candidate.

## Inputs

Record these before starting:

```text
Candidate version:
Candidate commit SHA:
Target branch:
Supported platforms:
Target game / integration:
Release owner:
```

## Version And Metadata

Framesmith currently keeps these version values aligned:

- `package.json` `version`
- `src-tauri/Cargo.toml` `package.version`
- `src-tauri/tauri.conf.json` `version`

Before a tagged release, update all three values together and include the
version in the release evidence. For an audit-only candidate, keep the existing
version and record that no tag is being cut.

## Clean Checkout Verification

Run from a clean checkout of the candidate commit:

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
pwsh -NoProfile -Command "$msi = Get-ChildItem -Path 'src-tauri/target/release/bundle/msi' -Filter '*.msi' -File; $nsis = Get-ChildItem -Path 'src-tauri/target/release/bundle/nsis' -Filter '*setup.exe' -File; if ($msi.Count -eq 0) { throw 'No MSI installer was produced' }; if ($nsis.Count -eq 0) { throw 'No NSIS setup executable was produced' }; $installerFiles = @($msi) + @($nsis); foreach ($file in $installerFiles) { if ($file.Length -le 0) { throw \"Installer output is empty: $($file.FullName)\" } }"
```

Expected local package outputs:

```text
src-tauri/target/release/framesmith.exe
src-tauri/target/release/bundle/msi/Framesmith_<version>_x64_en-US.msi
src-tauri/target/release/bundle/nsis/Framesmith_<version>_x64-setup.exe
```

## GitHub CI Verification

After pushing the candidate commit:

1. Open the GitHub Actions run for `.github/workflows/ci.yml`.
2. Confirm the run passed for the candidate SHA.
3. Confirm the `framesmith-windows-installers` artifact exists.
4. Download the artifact or record the artifact URL for installer smoke
   testing.

Record:

```text
GitHub Actions URL:
Candidate SHA:
CI status:
Artifact name:
Artifact URL or download source:
```

## Branch Protection Verification

Before treating `main` as production-protected:

1. Configure a branch protection rule or repository ruleset for `main`.
2. Require the CI workflow to pass before merge.
3. Require pull requests if the team uses reviewed changes.
4. Record evidence that a non-green change cannot merge.

Record:

```text
Protected branch/ruleset:
Required status checks:
Review requirement:
Evidence location:
```

## Installer Smoke Verification

Run [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) for
both installer formats.

Record:

```text
Windows version:
Architecture:
MSI source:
MSI result:
NSIS source:
NSIS result:
Warnings:
```

## Target Game Fit Review

Before declaring Framesmith production-ready for a game, review:

- [`production-handoff-decision.md`](production-handoff-decision.md)
- [`export-fidelity-contract.md`](export-fidelity-contract.md)
- [`combat-coverage.md`](combat-coverage.md)
- [`training-scenario-contract.md`](training-scenario-contract.md)
- [`production-gap-backlog.md`](production-gap-backlog.md)

Record every required backlog item as implemented, deferred, or not applicable.
If a required item is deferred, the release is not production-ready for that
target game.

## Final Evidence Template

```text
Version:
Commit:
CI run:
Local validation:
Installer smoke:
Supported platforms:
Target-game required backlog items:
Known accepted limitations:
Decision: ready / not ready
Decision owner:
Date:
```
