# Framesmith Agent Guidelines

Engine-agnostic fighting game character authoring tool (Tauri + Svelte).

## Start Here (Canonical)

| Topic | Location |
|-------|----------|
| Agent workflow rules | `AGENTS.md` (this file) |
| Claude Code guidelines | `CLAUDE.md` |
| Documentation index | `docs/README.md` |
| Design notes | `docs/design.md` |
| Data formats | `docs/data-formats.md` |
| MCP server | `docs/mcp-server.md` |
| Rules spec (SSOT) | `docs/rules-spec.md` |

## Repo Map

```
framesmith/
  src-tauri/               # Rust backend
    src/
      main.rs              # Tauri entry point
      lib.rs               # Library exports
      bin/mcp.rs           # MCP server binary
      bin/generate_schema.rs # Generates JSON schema artifacts
      codegen/             # Export adapters
        json_blob.rs       # JSON blob adapter
        zx_fspack.rs        # ZX FSPK pack exporter
        zx_fspack_format.rs # ZX FSPK format constants
      schema/              # Character data types
      commands.rs          # Tauri commands
      rules/                # Rules system (defaults + validation)
        mod.rs
      mcp/                 # MCP server modules
        validation.rs      # Move validation
  crates/
    framesmith-fspack/      # no_std-ish FSPK reader crate
  src/                     # Svelte frontend
    lib/
      views/               # Main editor views
      components/          # Reusable UI components
      stores/              # Svelte stores
    App.svelte
    main.ts
  characters/              # Working directory for character data
  docs/
    design.md              # Full design document
    plans/                 # Plans (deleted when implemented)
```

## Quick Commands

```bash
npm install              # Install dependencies
npm run tauri dev        # Development mode
npm run tauri build      # Production build
cd src-tauri && cargo test   # Run Rust tests
cd src-tauri && cargo run --bin mcp   # Run MCP server
cd src-tauri && cargo run --bin framesmith -- export --all --project .. --out-dir ../exports   # Export .fspk packs
```

## Key Constraints

1. **Engine-agnostic** - Data format is JSON; exporters handle engine-specific output
2. **Directory-based** - One file per state for easy git diffs and merge conflicts
3. **Central cancel table** - All cancel relationships in one file for easy visualization
4. **Rules-driven validation** - Defaults + constraints are configured via rules files
5. **Registry-aware** - Resources/events are validated against the rules registry (no silent typos)

## MCP Server

Framesmith includes an MCP server for LLM-assisted workflows (content generation, balancing, documentation).

### Configuration

Add to your Claude Code MCP config (`.mcp.json`):

```json
{
  "mcpServers": {
    "framesmith": {
      "command": "target/debug/mcp",
      "cwd": "./src-tauri",
      "env": {
        "FRAMESMITH_CHARACTERS_DIR": "../characters"
      }
    }
  }
}
```

`FRAMESMITH_CHARACTERS_DIR` should point to a project’s `characters/` directory (so project rules at `<project>/framesmith.rules.json` can be discovered via the parent directory).

### Tools

| Tool | Description |
|------|-------------|
| `ping` | Verify the MCP server is running |
| `list_characters` | List all characters with IDs, names, and move counts |
| `get_character` | Get complete character data (properties, moves, cancel table) |
| `list_moves` | List all moves with basic stats |
| `get_move` | Get a single move's complete data |
| `create_move` | Create a move by writing a full move object (validates before save) |
| `update_move` | Update a move by overwriting with a full move object (validates before save) |
| `delete_move` | Delete a move from a character |
| `get_cancel_table` | Get cancel relationships |
| `get_frame_data_table` | Get compact frame data table with computed advantage |
| `get_rules_schema` | Get JSON Schema for rules files |
| `get_builtin_validations` | List built-in validations that always run |

### Resources

| Resource | Description |
|----------|-------------|
| `framesmith://notation_guide` | Reference for fighting game numpad notation |
| `framesmith://rules_guide` | Embedded rules spec (`docs/rules-spec.md`) |
