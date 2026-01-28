# Framesmith MCP Server Design

## Overview

Framesmith exposes an MCP (Model Context Protocol) server that allows any compatible LLM client (Claude Code, Codex CLI, OpenCode, etc.) to read and modify character data. This enables LLM-assisted workflows for content generation, balancing, documentation, and natural language automation.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Clients                              │
│         (Claude Code, Codex CLI, OpenCode, etc.)            │
└─────────────────────┬───────────────────────────────────────┘
                      │ HTTP/SSE (MCP Protocol)
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                  framesmith-mcp                             │
│              (Rust binary, runs on localhost)               │
│                                                             │
│   Tools: CRUD for characters, moves, cancels, hurtboxes    │
│   Utils: get_frame_data_table, get_summary, validate       │
│   Resources: notation_guide                                 │
└─────────────┬───────────────────────────────┬───────────────┘
              │ IPC (when managed)            │ Direct I/O (standalone)
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────────┐
│   Framesmith GUI        │     │   Character Files           │
│   (Tauri + Svelte)      │     │   (characters/*/*.json)     │
└─────────────────────────┘     └─────────────────────────────┘
```

### Operational Modes

**Managed mode** (launched by Tauri):
- Tauri spawns `framesmith-mcp` as a sidecar on app launch
- Communicates via IPC (JSON-RPC over stdin/stdout or local socket)
- State lives in Tauri, sidecar proxies MCP requests to the main app
- GUI updates instantly when LLM makes changes
- Sidecar shuts down when Framesmith closes

**Standalone mode** (power users, headless):
```bash
framesmith-mcp --port 9000 --characters ./characters
```
- Reads/writes directly to character JSON files on disk
- No GUI required
- File watcher detects external changes, updates internal state
- Useful for CI pipelines, batch processing, or headless workflows

## MCP Tools

### Character Operations

| Tool | Description |
|------|-------------|
| `list_characters` | Returns array of character IDs and names |
| `get_character` | Full character data (properties, all moves, cancels, hurtboxes) |
| `create_character` | New character (blank or copied from existing) |
| `update_character` | Modify properties (health, speeds, etc.) |
| `delete_character` | Remove character and all associated files |
| `duplicate_character` | Clone a character for iteration |

### Move Operations

| Tool | Description |
|------|-------------|
| `list_moves` | All moves for a character with basic stats |
| `get_move` | Full move data including hitboxes/hurtboxes |
| `create_move` | New move with frame data, optional hitbox defaults |
| `update_move` | Modify any move properties |
| `delete_move` | Remove move from character |
| `duplicate_move` | Copy a move as starting point for similar moves |
| `batch_update_moves` | Update multiple moves atomically (for bulk balancing) |

### Cancel Table

| Tool | Description |
|------|-------------|
| `get_cancel_table` | Full cancel relationships |
| `update_cancel_table` | Modify chains, special cancels, jump cancels |

### Utilities

| Tool | Description |
|------|-------------|
| `get_frame_data_table` | Compact view of all moves with just the numbers (startup, active, recovery, advantage, damage) |
| `search_moves` | Filter moves by properties (e.g., `startup < 10`, `advantage_on_block > 0`) |
| `get_summary` | Stats: move count, average startup, frame advantage spread, gaps in toolset |
| `validate_character` | Check for schema errors, missing data, inconsistencies |
| `export_character` | Generate final output (JSON blob, Breakpoint Rust) |

### MCP Resources

| Resource | Description |
|----------|-------------|
| `notation_guide` | Numpad notation reference (236 = QCF, etc.) |

## Real-time Sync (Managed Mode)

When the LLM modifies data through MCP, the GUI reflects changes instantly:

```
LLM Client                framesmith-mcp              Tauri GUI
    │                          │                          │
    │  update_move(5L, ...)    │                          │
    │─────────────────────────>│                          │
    │                          │  IPC: update_move        │
    │                          │─────────────────────────>│
    │                          │                          │ (updates Svelte store)
    │                          │                          │ (re-renders UI)
    │                          │         OK + new state   │
    │                          │<─────────────────────────│
    │        { success: true } │                          │
    │<─────────────────────────│                          │
```

**Key behaviors:**
- Tauri holds authoritative state in memory
- All MCP writes go through Tauri via IPC (sidecar never writes files directly in managed mode)
- Tauri persists to disk and notifies the sidecar of success/failure
- Svelte stores update reactively - user sees changes appear live
- If GUI has unsaved changes when MCP write comes in, Tauri merges or warns (configurable)

**Standalone mode:**
- Sidecar reads/writes JSON files directly
- No IPC, no live GUI sync
- If Framesmith GUI is also open, it can watch files for changes (optional enhancement)

## Configuration

### MCP Client Configuration (Claude Code example)

```json
{
  "mcpServers": {
    "framesmith": {
      "url": "http://localhost:9000/mcp"
    }
  }
}
```

### Server Config File (optional, for standalone)

```toml
# framesmith-mcp.toml
port = 9000
characters_path = "./characters"
log_level = "info"
```

## Error Handling

MCP error responses follow the spec with structured errors:

| Error Code | Meaning | Example |
|------------|---------|---------|
| `not_found` | Character or move doesn't exist | `get_move("glitch", "5XX")` - no such move |
| `validation_error` | Data fails schema validation | Negative startup frames, missing required fields |
| `conflict` | Concurrent edit conflict | GUI and LLM both edited same move (managed mode) |
| `io_error` | File system issue | Permissions, disk full, missing directory |

**Example error response:**
```json
{
  "error": {
    "code": "validation_error",
    "message": "startup must be positive integer",
    "details": {
      "field": "startup",
      "value": -5,
      "move": "5L"
    }
  }
}
```

**Validation runs on every write:**
- Frame data sanity (startup > 0, active > 0, etc.)
- Hitbox bounds within reasonable range
- Cancel table references only existing moves
- Required fields present

## Implementation Components

### New Code

| Component | Location | Description |
|-----------|----------|-------------|
| `framesmith-mcp` binary | `src-tauri/src/bin/mcp.rs` or separate crate | HTTP/SSE server implementing MCP protocol |
| MCP tool handlers | `src-tauri/src/mcp/tools/` | One module per tool category (characters, moves, cancels, utils) |
| IPC layer | `src-tauri/src/mcp/ipc.rs` | Communication with Tauri when running as sidecar |
| File I/O layer | `src-tauri/src/mcp/storage.rs` | Direct JSON read/write for standalone mode |
| Sidecar management | `src-tauri/src/main.rs` | Tauri spawns/manages the MCP server |

### Reusable from Existing Code

- Schema types (`src-tauri/src/schema/`) - already defined
- File I/O commands - `load_character`, `list_characters` logic can be shared

### Dependencies

- `axum` or `actix-web` - HTTP server
- `tokio` - async runtime
- MCP SDK for Rust if available, otherwise implement protocol directly (JSON-RPC over SSE)

### Frontend Changes

- Svelte stores need to handle external updates (from IPC)
- Optional: toast/notification when LLM makes changes

## Use Cases

### Content Generation
"Create a 3-hit rekka series starting from 236P" → LLM calls `create_move` three times with appropriate frame data

### Balance Analysis
LLM calls `get_frame_data_table`, analyzes the numbers, suggests adjustments: "5L is -2 on block but 7f startup - consider making it -1 or increasing startup to 8f"

### Documentation
LLM reads character data via `get_character`, generates move descriptions and strategy notes

### Workflow Automation
Natural language commands: "Make all light attacks 1 frame faster" → LLM calls `search_moves` to find lights, then `batch_update_moves` to adjust startup
