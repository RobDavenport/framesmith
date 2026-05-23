# Windows Installer Smoke Test

Status: active
Last reviewed: 2026-05-23

Run this manual smoke test for every Windows release candidate after
`npm run tauri build` or after downloading the CI `framesmith-windows-installers`
artifact.

## Artifacts

Expected files:

```text
src-tauri/target/release/bundle/msi/Framesmith_<version>_x64_en-US.msi
src-tauri/target/release/bundle/nsis/Framesmith_<version>_x64-setup.exe
```

## MSI Smoke Test

1. Install the MSI on a Windows machine or clean VM.
2. Launch Framesmith from the Start menu.
3. Open the repository project folder.
4. Load `TEST_CHAR`.
5. Switch to State Editor and select `5L`.
6. Switch to Training and confirm `P1` and `CPU` appear without an initialization
   error.
7. Export `TEST_CHAR` as `json-blob`.
8. Close Framesmith.
9. Uninstall Framesmith from Windows Apps.

Pass criteria:

- The app launches without Windows SmartScreen or installer errors beyond the
  expected unsigned-build warning.
- The project opens and `TEST_CHAR` loads.
- Training starts from the packaged WASM and FSPK path.
- Export completes without an IPC error.
- Uninstall removes the app entry.

## NSIS Smoke Test

Repeat the same flow with the NSIS `setup.exe`.

## Evidence To Record

For the release note or checklist, record:

- Windows version and architecture.
- Installer type tested: MSI, NSIS, or both.
- Artifact source: local build path or CI run URL.
- Framesmith version.
- Pass/fail result and any warnings.
