# Framesmith

Engine-agnostic fighting game character authoring tool. Define complete, portable fighting game characters (frame data, hitboxes, moves, properties, cancel routes) in a format that can be exported to any game engine.

## Features

- **Frame Data Table** - Spreadsheet view of all moves with sorting, filtering, and multi-character comparison
- **Move Editor** - Form-based editing with animation preview and hitbox overlay
- **Cancel Graph** - Visual node graph showing move cancel relationships
- **Export Adapters** - Generate engine-specific output (Rust constants, JSON blob, more to come)
- **MCP Server** - LLM integration for content generation, balancing, and documentation

## Quick Start

```bash
npm install              # Install dependencies
npm run tauri dev        # Development mode
npm run tauri build      # Production build
```

## Documentation

- [`docs/design.md`](docs/design.md) - Full design specification
- [`CLAUDE.md`](CLAUDE.md) - Claude Code guidelines and domain knowledge

## Tech Stack

| Component | Technology |
|-----------|------------|
| Desktop framework | Tauri (Rust backend + web frontend) |
| Frontend | Svelte + TypeScript |
| 3D rendering | Threlte (Three.js wrapper for Svelte) |
| Data format | JSON (directory-based, one file per move) |

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
