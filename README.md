# Framesmith

Framesmith is a frame-based, binary-first fighting-game helper with engine-independent
Rust crates and an optional character editor. JSON is editable authoring data;
FSPK v2 is the full-fidelity runtime handoff, without a JSON sidecar.

Start with [`framesmith-fspack`](crates/framesmith-fspack/README.md) for generic
zero-copy payloads or [`framesmith-runtime`](crates/framesmith-runtime/README.md)
for optional deterministic frame/cancel/resource/collision helpers. Neither
requires Tauri or a game engine. See the [handoff/migration policy](docs/production-handoff-decision.md).

## Core capabilities

- Frame-data table with filtering and sorting
- State editor with sprite and GLTF preview
- Cancel graph view for route visualization
- Rules system for defaults and validation
- Export adapters (`json-blob`, `fspk`)
- MCP server for scripted and LLM-assisted workflows

## Framesmith project format

A Framesmith project is a directory with this shape:

```text
my-game/
  framesmith.rules.json
  characters/
    test_char/
      character.json
      cancel_table.json
      states/
        5L.json
        236P.json
      rules.json          # Optional per-character overrides
```

This repository root is also a valid project because it includes `framesmith.rules.json` and `characters/`.

## Quick start

```bash
npm install
npm run tauri dev
```

## Common commands

```bash
# Frontend + app shell
npm run dev
npm run tauri dev
npm run tauri build

# TypeScript checks/tests
npm run check
npm run test:run

# Runtime WASM package
npm run wasm:build
npm run wasm:build:dev

# Rust backend checks/tests
cd src-tauri
cargo test
cargo clippy --all-targets
```

## MCP server

```bash
cd src-tauri
cargo run --bin mcp -- --characters-dir ../characters
```

See `docs/mcp-server.md` for tools, resources, and integration details.

## CLI export

```bash
cd src-tauri
cargo run --bin framesmith-cli -- export --project .. --all --out-dir ../exports
```

See `docs/cli.md` for full CLI reference.

## Documentation map

- `docs/README.md`: documentation index and routing guide
- `docs/data-formats.md`: on-disk JSON schema and layout
- `docs/rules-spec.md`: rules semantics and validation behavior
- `docs/zx-fspack.md`: binary pack format reference
- `docs/runtime-guide.md`: runtime integration guide
- `docs/runtime-api.md`: runtime API details
- `docs/mcp-server.md`: MCP server setup and tool list
- `docs/global-states.md`: global state authoring and behavior
- `docs/character-authoring-guide.md`: practical authoring workflow
- `AGENTS.md`: canonical agent workflow, invariants, code map, and targeted checks
- `CLAUDE.md`: compatibility import of `AGENTS.md`

## Repo map

```text
framesmith/
  src/              # SvelteKit UI
  src-tauri/        # Rust app backend, MCP server, CLI
  crates/           # Runtime and FSPK library crates
  characters/       # Local project data samples
  docs/             # Design and reference docs
```
