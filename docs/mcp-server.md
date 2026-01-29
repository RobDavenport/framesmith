# Framesmith MCP Server

**Status:** Active
**Last reviewed:** 2026-01-30

Framesmith ships an MCP server binary at `src-tauri/src/bin/mcp.rs`. It exposes tools for inspecting and editing character data on disk, with the same validation pipeline used by the app/exporters.

## Run It

From repo root:

```bash
cd src-tauri
cargo run --bin mcp
```

The server reads `FRAMESMITH_CHARACTERS_DIR` to find your characters folder. If unset, it defaults to `./characters` relative to `src-tauri/`.

Recommended: point it at a project’s `characters/` folder (so it can also find `<project>/framesmith.rules.json` via the parent directory).

## Configuration (.mcp.json)

If you use `"command": "target/debug/mcp"`, build it first:

```bash
cd src-tauri
cargo build --bin mcp
```

Example for this repo (project root is the repo root):

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

Example for an external project at `D:/games/my-game`:

```json
{
  "mcpServers": {
    "framesmith": {
      "command": "target/debug/mcp",
      "cwd": "./src-tauri",
      "env": {
        "FRAMESMITH_CHARACTERS_DIR": "D:/games/my-game/characters"
      }
    }
  }
}
```

## Tools

Implemented tools (see `src-tauri/src/bin/mcp.rs`):

| Tool | Description |
|------|-------------|
| `ping` | Verify the MCP server is running |
| `list_characters` | List all characters with IDs, names, and move counts |
| `get_character` | Get complete character data (properties, moves, cancel table) |
| `list_moves` | List moves with basic stats |
| `get_move` | Get a single move’s complete JSON |
| `create_move` | Create a move by writing a full move object (validates before save) |
| `update_move` | Update a move by overwriting with a full move object (validates before save) |
| `delete_move` | Delete a move file |
| `get_cancel_table` | Get cancel relationships |
| `get_frame_data_table` | Get a compact computed frame-data table |
| `get_rules_schema` | Get JSON Schema for rules files (for IDE autocomplete) |
| `get_builtin_validation` | List built-in validations that always run |

## Resources

| Resource | Description |
|----------|-------------|
| `framesmith://notation_guide` | Numpad notation reference |
| `framesmith://rules_guide` | Embedded `docs/rules-spec.md` |

## Validation Behavior

- `create_move` and `update_move` validate using project rules (`<project>/framesmith.rules.json`) plus optional character rules (`<project>/characters/<id>/rules.json`).
- Registry-aware checks run (resources/events must match the rules registry).
- On validation errors, the tool returns `INVALID_PARAMS`.
