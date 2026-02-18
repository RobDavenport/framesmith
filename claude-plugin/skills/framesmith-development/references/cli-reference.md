# Framesmith CLI Reference

The CLI binary lives at `src-tauri/src/bin/framesmith.rs`. It provides automation for export tasks, running the same rules and validation pipeline as the GUI application.

## Build

```bash
cd src-tauri
cargo build --bin framesmith --release
```

The binary will be at `src-tauri/target/release/framesmith.exe` (Windows) or `src-tauri/target/release/framesmith` (Linux/macOS).

## Export Command

### Export One Character

```bash
cd src-tauri
cargo run --bin framesmith -- export \
  --project .. \
  --character test_char \
  --out ../exports/test_char.fspk
```

If `--out` is omitted, it defaults to `<project>/exports/<character>.fspk`.

### Export All Characters

```bash
cd src-tauri
cargo run --bin framesmith -- export \
  --project .. \
  --all \
  --out-dir ../exports
```

### Characters Directory Override

You can point directly at a `characters/` directory instead of a project root:

```bash
cd src-tauri
cargo run --bin framesmith -- export \
  --characters-dir ../characters \
  --all \
  --out-dir ../exports
```

Or set the `FRAMESMITH_CHARACTERS_DIR` environment variable:

```bash
export FRAMESMITH_CHARACTERS_DIR=../characters
cargo run --bin framesmith -- export --all --out-dir ../exports
```

## Export Adapters

| Flag | Output | Notes |
|------|--------|-------|
| `--adapter zx-fspack` | `.fspk` binary | Default adapter |
| `--adapter json-blob` | `.json` file | Supports `--pretty` flag |

### Examples

```bash
# Default: FSPK binary export
cargo run --bin framesmith -- export --project .. --character ryu --out ryu.fspk

# JSON blob export
cargo run --bin framesmith -- export --project .. --character ryu \
  --adapter json-blob --out ryu.json

# Pretty-printed JSON blob
cargo run --bin framesmith -- export --project .. --character ryu \
  --adapter json-blob --pretty --out ryu.json
```

## Validation Pipeline

The CLI runs the same validation as the app:

1. **Project rules** from `<project>/framesmith.rules.json`
2. **Character rules** from `<project>/characters/<id>/rules.json` (optional)
3. Validation errors abort the export with a non-zero exit code

## MCP Server

The MCP server is a separate binary for AI tool integration:

```bash
# Build
cd src-tauri && cargo build --bin mcp

# Run
cd src-tauri && cargo run --bin mcp -- --characters-dir ../characters
```

Full docs: `docs/mcp-server.md`
