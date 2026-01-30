# Framesmith CLI

**Status:** Active
**Last reviewed:** 2026-01-30

Framesmith includes a small Rust CLI for automation tasks like exporting `.fspk` packs.

The CLI binary lives at `src-tauri/src/bin/framesmith.rs`.

## Build

```bash
cd src-tauri
cargo build --bin framesmith --release
```

The binary will be at `src-tauri/target/release/framesmith.exe`.

## Export

The `export` command runs the same rules + validation pipeline as the app:

- Project rules: `<project>/framesmith.rules.json`
- Character rules: `<project>/characters/<id>/rules.json` (optional)

### Export One Character

```bash
cd src-tauri
cargo run --bin framesmith -- export --project .. --character test_char --out ../exports/test_char.fspk
```

If `--out` is omitted, it defaults to `<project>/exports/<character>.fspk`.

### Export All Characters

```bash
cd src-tauri
cargo run --bin framesmith -- export --project .. --all --out-dir ../exports
```

### Characters Directory

You can point directly at a `characters/` directory instead of a project root:

```bash
cd src-tauri
cargo run --bin framesmith -- export --characters-dir ../characters --all --out-dir ../exports
```

Or set `FRAMESMITH_CHARACTERS_DIR`.

### Adapters

- `--adapter zx-fspack` (default) writes `.fspk`
- `--adapter json-blob` writes `.json` (`--pretty` supported)
