# Framesmith

Engine-agnostic fighting game character authoring tool. Define portable fighting game characters (frame data, hitboxes, moves, resources, events, cancel routes) as JSON on disk and export to engine/runtime-specific formats.

## Features

- **Frame Data Table** - Spreadsheet view of all moves with sorting and filtering
- **Move Editor** - Form-based editing with sprite/GLTF animation preview
- **Cancel Graph** - Visual node graph showing move cancel relationships
- **Export Adapters** - `json-blob` (single JSON) and `zx-fspack` (compact binary pack)
- **Rules System** - Project/character rules for defaults + validation, plus a registry for resources/events
- **MCP Server** - LLM integration and programmatic editing/validation via MCP tools

## What Is A “Project”?

A Framesmith project is a folder with this structure:

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
      rules.json          (optional character overrides)
```

This repo root is also a valid project (it contains `framesmith.rules.json` and `characters/`).

## Quick Start

```bash
npm install              # Install dependencies
npm run tauri dev        # Development mode
npm run tauri build      # Production build
```

Rust tests:

```bash
cd src-tauri && cargo test
```

## MCP Server

The MCP server is documented in `docs/mcp-server.md`.

```bash
cd src-tauri
cargo run --bin mcp
```

## CLI

For automation (like exporting `.fspk` packs), use the `framesmith` CLI.

```bash
cd src-tauri
cargo run --bin framesmith -- export --project .. --all --out-dir ../exports
```

See `docs/cli.md`.

## Documentation

- Start here: [`docs/README.md`](docs/README.md)
- Data formats: [`docs/data-formats.md`](docs/data-formats.md)
- Rules (SSOT): [`docs/rules-spec.md`](docs/rules-spec.md)
- MCP server: [`docs/mcp-server.md`](docs/mcp-server.md)
- ZX FSPK format: [`docs/zx-fspack.md`](docs/zx-fspack.md)
- Design notes: [`docs/design.md`](docs/design.md)
- Contributor/agent notes: [`AGENTS.md`](AGENTS.md), [`CLAUDE.md`](CLAUDE.md)

## Tech Stack

| Component | Technology |
|-----------|------------|
| Desktop framework | Tauri (Rust backend + web frontend) |
| Frontend | Svelte + TypeScript |
| 3D rendering | Threlte / Three.js (sprite + GLTF preview) |
| Data format | JSON (directory-based, one file per move) |

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
