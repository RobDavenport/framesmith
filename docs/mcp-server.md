# Framesmith MCP Server

**Status:** Active
**Last reviewed:** 2026-01-30

Framesmith ships an MCP server binary at `src-tauri/src/bin/mcp.rs`. It exposes tools for inspecting and editing character data on disk, with the same validation pipeline used by the app/exporters.

## Run It

From repo root:

```bash
cd src-tauri
cargo run --bin mcp -- --characters-dir ../characters
```

The server finds the characters folder via (in priority order):
1. `--characters-dir` / `-c` CLI argument
2. `FRAMESMITH_CHARACTERS_DIR` environment variable
3. Default: `./characters` relative to cwd

Paths are canonicalized on startup, so relative paths work correctly.

## Configuration (.mcp.json)

Build the MCP binary first:

```bash
cd framesmith/src-tauri
cargo build --release --bin mcp
```

All paths in the config are relative to your project root (where `.mcp.json` lives). No `cwd` manipulation needed.

### Framesmith repo

```json
{
  "mcpServers": {
    "framesmith": {
      "command": "./src-tauri/target/release/mcp.exe",
      "args": ["--characters-dir", "./characters"]
    }
  }
}
```

### External project (same workspace)

For a project alongside framesmith (e.g., `workspace/my-game` next to `workspace/framesmith`):

```json
{
  "mcpServers": {
    "framesmith": {
      "command": "../framesmith/src-tauri/target/release/mcp.exe",
      "args": ["--characters-dir", "./characters"]
    }
  }
}
```

Both paths are relative to your project root:
- `command`: path to the framesmith binary
- `--characters-dir`: path to your project's characters folder

### External project (different location)

For projects outside the workspace, use absolute paths:

```json
{
  "mcpServers": {
    "framesmith": {
      "command": "C:/tools/framesmith/mcp.exe",
      "args": ["--characters-dir", "./characters"]
    }
  }
}
```

Or install the binary to your PATH and just use `"command": "framesmith-mcp"`.

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
| `get_builtin_validations` | List built-in validations that always run |
| `export_character` | Export a character to a file (runs rules + validation) |
| `export_all_characters` | Export all characters to a directory (runs rules + validation) |

## Resources

| Resource | Description |
|----------|-------------|
| `framesmith://notation_guide` | Numpad notation reference |
| `framesmith://rules_guide` | Embedded `docs/rules-spec.md` |

## Validation Behavior

- `create_move` and `update_move` validate using project rules (`<project>/framesmith.rules.json`) plus optional character rules (`<project>/characters/<id>/rules.json`).
- Registry-aware checks run (resources/events must match the rules registry).
- On validation errors, the tool returns `INVALID_PARAMS`.

## Export Tools

- `export_character` and `export_all_characters` run the same validation + rules pipeline as the app export.
- For safety, output paths must be under the project root (the parent directory of `FRAMESMITH_CHARACTERS_DIR`).

Example usage (conceptual):

```text
export_character({
  "character_id": "test_char",
  "adapter": "zx-fspack",
  "output_path": "exports/test_char.fspk"
})
```
