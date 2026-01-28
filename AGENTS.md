# Framesmith Agent Guidelines

Engine-agnostic fighting game character authoring tool (Tauri + Svelte).

## Start Here (Canonical)

| Topic | Location |
|-------|----------|
| Agent workflow rules | `AGENTS.md` (this file) |
| Claude Code guidelines | `CLAUDE.md` |
| Documentation index | `docs/README.md` |
| Full design spec | `docs/design.md` |

## Repo Map

```
framesmith/
  src-tauri/               # Rust backend
    src/
      main.rs              # Tauri entry point
      lib.rs               # Library exports
      bin/mcp.rs           # MCP server binary
      codegen/             # Export adapters
        breakpoint.rs      # Breakpoint adapter (Rust constants)
        json_blob.rs       # JSON blob adapter
      schema/              # Character data types
      commands.rs          # Tauri commands
      mcp/                 # MCP server modules
        validation.rs      # Move validation
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
    plans/                 # Implementation plans
```

## Quick Commands

```bash
npm install              # Install dependencies
npm run tauri dev        # Development mode
npm run tauri build      # Production build
cd src-tauri && cargo test   # Run Rust tests
```

## Key Constraints

1. **Engine-agnostic** - Data format is JSON; exporters handle engine-specific output
2. **Directory-based** - One file per move for easy git diffs and merge conflicts
3. **Central cancel table** - All cancel relationships in one file for easy visualization
4. **Hybrid assets** - References during dev, bake to bundle for export

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

### Tools

| Tool | Description |
|------|-------------|
| `list_characters` | List all characters with IDs, names, and move counts |
| `get_character` | Get complete character data (properties, moves, cancel table) |
| `get_move` | Get a single move's complete data |
| `update_move` | Update a move (validates before save) |
| `create_move` | Create a new move for a character |
| `delete_move` | Delete a move from a character |
| `list_moves` | List all moves with basic stats |
| `get_cancel_table` | Get cancel relationships |
| `get_frame_data_table` | Get compact frame data table with computed advantage |

### Resources

| Resource | Description |
|----------|-------------|
| `framesmith://notation_guide` | Reference for fighting game numpad notation |
