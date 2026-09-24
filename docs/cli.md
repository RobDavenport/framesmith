# Framesmith CLI

**Status:** Active
**Last reviewed:** 2026-05-23

Framesmith includes a small Rust CLI for automation tasks like exporting `.fspk` packs.

The CLI binary lives at `src-tauri/src/bin/framesmith.rs`.
The export examples in this document are executed by
`cargo test --manifest-path src-tauri/Cargo.toml --test docs_cli_examples`.

## Build

```bash
cd src-tauri
cargo build --bin framesmith-cli --release
```

The binary will be at `src-tauri/target/release/framesmith-cli.exe`.

## Export

The `export` command runs the same rules + validation pipeline as the app:

- Project rules: `<project>/framesmith.rules.json`
- Character rules: `<project>/characters/<id>/rules.json` (optional)

### Export One Character

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --character test_char --out ../exports/test_char.fspk
```

If `--out` is omitted, it defaults to `<project>/exports/<character>.fspk`.

### Export All Characters

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --all --out-dir ../exports
```

### Characters Directory

You can point directly at a `characters/` directory instead of a project root:

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --characters-dir ../characters --all --out-dir ../exports
```

Or set `FRAMESMITH_CHARACTERS_DIR`.

### Adapters

- `--adapter fspk` (default) writes `.fspk`
- `--adapter json-blob` writes `.json` (`--pretty` supported)

`zx-fspack` is accepted as a legacy alias for `fspk`, but new docs and scripts
should use `fspk`.
