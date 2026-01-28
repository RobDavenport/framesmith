# Framesmith MCP Server Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement an MCP server that exposes Framesmith character data for LLM-assisted workflows (content generation, balancing, documentation).

**Architecture:** Standalone binary (`framesmith-mcp`) using the `rmcp` crate with stdio transport. Reads/writes character JSON files directly. Later phases will add Tauri sidecar integration for live GUI sync.

**Tech Stack:** Rust, rmcp 0.14.x, tokio, serde, schemars

---

## Phase 1: Minimal Viable MCP Server

### Task 1: Add MCP Binary Scaffold

**Files:**
- Create: `src-tauri/src/bin/mcp.rs`
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add dependencies to Cargo.toml**

Add after line 24 in `src-tauri/Cargo.toml`:

```toml
# MCP server dependencies
rmcp = { version = "0.14", features = ["server", "transport-io", "macros"] }
tokio = { version = "1", features = ["full"] }
schemars = "1.0"
anyhow = "1.0"
```

**Step 2: Run cargo check to verify dependencies resolve**

Run: `cd src-tauri && cargo check`
Expected: Dependencies download and compile successfully

**Step 3: Create minimal MCP binary**

Create `src-tauri/src/bin/mcp.rs`:

```rust
use anyhow::Result;
use rmcp::{
    handler::server::tool::ToolRouter,
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, Error as McpError, ServerHandler, ServiceExt,
    transport::stdio,
};
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct FramesmithMcp {
    characters_dir: String,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl FramesmithMcp {
    pub fn new(characters_dir: String) -> Self {
        Self {
            characters_dir,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Returns a greeting to verify the MCP server is working")]
    async fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "Framesmith MCP server is running!",
        )]))
    }
}

#[tool_handler]
impl ServerHandler for FramesmithMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Framesmith MCP server for reading and modifying fighting game character data. \
                 Use list_characters to see available characters, then get_character or get_move \
                 to read data, and update_move to make changes.".to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Default to ./characters, can be overridden via env
    let characters_dir = std::env::var("FRAMESMITH_CHARACTERS_DIR")
        .unwrap_or_else(|_| "./characters".to_string());

    let service = FramesmithMcp::new(characters_dir).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

**Step 4: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add minimal MCP server scaffold with ping tool"
```

---

### Task 2: Add list_characters Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add list_characters tool**

Add to the `#[tool_router] impl FramesmithMcp` block, after `ping`:

```rust
    #[tool(description = "List all available characters with their IDs, names, and move counts")]
    async fn list_characters(&self) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::list_characters as list_chars;

        let summaries = list_chars(self.characters_dir.clone()).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&summaries).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 3: Test manually with Claude Code**

Create test MCP config at project root `.mcp.json`:

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

Run: In Claude Code, call `list_characters` tool
Expected: Returns JSON array of character summaries

**Step 4: Commit**

```bash
git add src-tauri/src/bin/mcp.rs .mcp.json
git commit -m "feat(mcp): add list_characters tool"
```

---

### Task 3: Add get_character Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add parameter struct**

Add near the top of `mcp.rs`, after the imports:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CharacterIdParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
}
```

**Step 2: Add get_character tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Get complete character data including properties, all moves, and cancel table")]
    async fn get_character(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<CharacterIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&data).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 4: Test manually**

Run: In Claude Code, call `get_character` with `character_id: "glitch"`
Expected: Returns complete character JSON

**Step 5: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add get_character tool"
```

---

### Task 4: Add get_move Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add parameter struct**

Add after `CharacterIdParam`:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveIdParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
    #[schemars(description = "The move input notation (e.g., '5L', '236P')")]
    pub move_input: String,
}
```

**Step 2: Add get_move tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Get a single move's complete data including hitboxes and frame data")]
    async fn get_move(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<MoveIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id.clone()).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let mv = data.moves.iter().find(|m| m.input == params.move_input).ok_or_else(|| McpError {
            code: rmcp::model::ErrorCode::INVALID_PARAMS,
            message: Cow::from(format!("Move '{}' not found for character '{}'", params.move_input, params.character_id)),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&mv).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 4: Test manually**

Run: In Claude Code, call `get_move` with `character_id: "glitch", move_input: "5L"`
Expected: Returns the 5L move JSON

**Step 5: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add get_move tool"
```

---

### Task 5: Add update_move Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add parameter struct**

Add after `MoveIdParam`:

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateMoveParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
    #[schemars(description = "Complete move data as JSON object")]
    pub move_data: d_developmentnethercore_projectframesmith_lib::schema::Move,
}
```

**Step 2: Add update_move tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Update a move's data. Provide complete move object - it will overwrite the existing move file.")]
    async fn update_move(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<UpdateMoveParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::save_move;

        save_move(self.characters_dir.clone(), params.character_id.clone(), params.move_data.clone()).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully updated move '{}' for character '{}'",
            params.move_data.input, params.character_id
        ))]))
    }
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 4: Test manually**

Run: In Claude Code, get a move with `get_move`, modify a field, then call `update_move`
Expected: Move file is updated on disk

**Step 5: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add update_move tool"
```

---

### Task 6: Add get_frame_data_table Utility Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add frame data table struct**

Add after `UpdateMoveParam`:

```rust
#[derive(Debug, serde::Serialize)]
pub struct FrameDataRow {
    pub input: String,
    pub name: String,
    pub startup: u8,
    pub active: u8,
    pub recovery: u8,
    pub total: u8,
    pub damage: u16,
    pub hitstun: u8,
    pub blockstun: u8,
    pub advantage_on_hit: i16,
    pub advantage_on_block: i16,
    pub guard: String,
}
```

**Step 2: Add get_frame_data_table tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Get a compact frame data table for a character - shows startup, active, recovery, damage, and advantage for all moves")]
    async fn get_frame_data_table(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<CharacterIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let rows: Vec<FrameDataRow> = data.moves.iter().map(|m| {
            let total = m.startup + m.active + m.recovery;
            let advantage_on_hit = m.hitstun as i16 - m.recovery as i16;
            let advantage_on_block = m.blockstun as i16 - m.recovery as i16;
            FrameDataRow {
                input: m.input.clone(),
                name: m.name.clone(),
                startup: m.startup,
                active: m.active,
                recovery: m.recovery,
                total,
                damage: m.damage,
                hitstun: m.hitstun,
                blockstun: m.blockstun,
                advantage_on_hit,
                advantage_on_block,
                guard: format!("{:?}", m.guard).to_lowercase(),
            }
        }).collect();

        let json = serde_json::to_string_pretty(&rows).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 4: Test manually**

Run: In Claude Code, call `get_frame_data_table` with `character_id: "glitch"`
Expected: Returns compact table with computed advantage values

**Step 5: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add get_frame_data_table utility tool"
```

---

## Phase 2: Extended Tools

### Task 7: Add list_moves Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add list_moves tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "List all moves for a character with basic stats (input, name, startup, damage)")]
    async fn list_moves(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<CharacterIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        #[derive(serde::Serialize)]
        struct MoveSummary {
            input: String,
            name: String,
            startup: u8,
            damage: u16,
        }

        let summaries: Vec<MoveSummary> = data.moves.iter().map(|m| MoveSummary {
            input: m.input.clone(),
            name: m.name.clone(),
            startup: m.startup,
            damage: m.damage,
        }).collect();

        let json = serde_json::to_string_pretty(&summaries).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add list_moves tool"
```

---

### Task 8: Add get_cancel_table Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add get_cancel_table tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Get the cancel table showing all cancel relationships (chains, special cancels, super cancels, jump cancels)")]
    async fn get_cancel_table(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<CharacterIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let json = serde_json::to_string_pretty(&data.cancel_table).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add get_cancel_table tool"
```

---

### Task 9: Add create_move Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add create_move tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Create a new move for a character. Provide complete move data.")]
    async fn create_move(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<UpdateMoveParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::{load_character, save_move};
        use std::path::Path;

        // Check if move already exists
        let move_path = Path::new(&self.characters_dir)
            .join(&params.character_id)
            .join("moves")
            .join(format!("{}.json", params.move_data.input));

        if move_path.exists() {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!(
                    "Move '{}' already exists for character '{}'. Use update_move instead.",
                    params.move_data.input, params.character_id
                )),
                data: None,
            });
        }

        save_move(self.characters_dir.clone(), params.character_id.clone(), params.move_data.clone()).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully created move '{}' for character '{}'",
            params.move_data.input, params.character_id
        ))]))
    }
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add create_move tool"
```

---

### Task 10: Add delete_move Tool

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add delete_move tool**

Add to the `#[tool_router] impl FramesmithMcp` block:

```rust
    #[tool(description = "Delete a move from a character")]
    async fn delete_move(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<MoveIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use std::path::Path;

        // Validate character_id
        if params.character_id.contains("..") || params.character_id.contains('/') || params.character_id.contains('\\') {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from("Invalid character ID"),
                data: None,
            });
        }

        let move_path = Path::new(&self.characters_dir)
            .join(&params.character_id)
            .join("moves")
            .join(format!("{}.json", params.move_input));

        if !move_path.exists() {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!(
                    "Move '{}' not found for character '{}'",
                    params.move_input, params.character_id
                )),
                data: None,
            });
        }

        std::fs::remove_file(&move_path).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Failed to delete move: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully deleted move '{}' from character '{}'",
            params.move_input, params.character_id
        ))]))
    }
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add delete_move tool"
```

---

### Task 11: Add notation_guide Resource

**Files:**
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Add resource handler imports and implementation**

Add to `ServerHandler` implementation, after `get_info`:

```rust
    async fn list_resources(&self) -> Result<Vec<rmcp::model::Resource>, McpError> {
        Ok(vec![rmcp::model::Resource {
            uri: "framesmith://notation_guide".to_string(),
            name: "Numpad Notation Guide".to_string(),
            description: Some("Reference for fighting game numpad notation (236 = QCF, etc.)".to_string()),
            mime_type: Some("text/markdown".to_string()),
        }])
    }

    async fn read_resource(&self, uri: &str) -> Result<rmcp::model::ResourceContents, McpError> {
        if uri == "framesmith://notation_guide" {
            let guide = r#"# Fighting Game Numpad Notation

## Directional Inputs (Numpad Layout)
```
7 8 9    ↖ ↑ ↗
4 5 6    ← N →
1 2 3    ↙ ↓ ↘
```

## Common Motions
- **236** = Quarter Circle Forward (QCF) = ↓↘→
- **214** = Quarter Circle Back (QCB) = ↓↙←
- **623** = Dragon Punch (DP) = →↓↘
- **421** = Reverse DP = ←↓↙
- **41236** = Half Circle Forward (HCF) = ←↙↓↘→
- **63214** = Half Circle Back (HCB) = →↘↓↙←
- **360** = Full Circle = 63214789 or similar
- **22** = Double Down = ↓↓

## Button Notation
- **L** = Light attack
- **M** = Medium attack
- **H** = Heavy attack
- **P** = Punch (for games with P/K distinction)
- **K** = Kick

## Standing/Crouching/Jumping
- **5X** = Standing (neutral) attack (e.g., 5L = standing light)
- **2X** = Crouching attack (e.g., 2M = crouching medium)
- **j.X** = Jumping attack (e.g., j.H = jumping heavy)

## Examples
- **5L** = Standing light attack
- **2M** = Crouching medium attack
- **236P** = Quarter circle forward + punch (fireball motion)
- **623H** = Dragon punch motion + heavy (uppercut)
- **j.236K** = Air quarter circle forward + kick
"#;
            Ok(rmcp::model::ResourceContents {
                uri: uri.to_string(),
                mime_type: Some("text/markdown".to_string()),
                text: Some(guide.to_string()),
                blob: None,
            })
        } else {
            Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Unknown resource: {}", uri)),
                data: None,
            })
        }
    }
```

**Step 2: Update ServerCapabilities**

Modify `get_info` to enable resources:

```rust
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Framesmith MCP server for reading and modifying fighting game character data. \
                 Use list_characters to see available characters, then get_character or get_move \
                 to read data, and update_move to make changes. \
                 The notation_guide resource explains numpad notation.".to_string(),
            ),
        }
    }
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 4: Commit**

```bash
git add src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add notation_guide resource"
```

---

## Phase 3: Validation and Error Handling

### Task 12: Add Move Validation

**Files:**
- Create: `src-tauri/src/mcp/validation.rs`
- Modify: `src-tauri/src/bin/mcp.rs`

**Step 1: Create validation module**

Create `src-tauri/src/mcp/validation.rs`:

```rust
use crate::schema::Move;

#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

pub fn validate_move(mv: &Move) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Frame data sanity
    if mv.startup == 0 {
        errors.push(ValidationError {
            field: "startup".to_string(),
            message: "startup must be at least 1 frame".to_string(),
        });
    }

    if mv.active == 0 {
        errors.push(ValidationError {
            field: "active".to_string(),
            message: "active must be at least 1 frame".to_string(),
        });
    }

    // Input validation
    if mv.input.is_empty() {
        errors.push(ValidationError {
            field: "input".to_string(),
            message: "input cannot be empty".to_string(),
        });
    }

    // Hitbox frame range validation
    let total_frames = mv.startup + mv.active + mv.recovery;
    for (i, hitbox) in mv.hitboxes.iter().enumerate() {
        if hitbox.frames.0 > hitbox.frames.1 {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: "start frame cannot be after end frame".to_string(),
            });
        }
        if hitbox.frames.1 > total_frames {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: format!("end frame {} exceeds total frames {}", hitbox.frames.1, total_frames),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

**Step 2: Create mcp module directory**

Create `src-tauri/src/mcp/mod.rs`:

```rust
pub mod validation;
```

**Step 3: Add mcp module to lib.rs**

Modify `src-tauri/src/lib.rs` to add:

```rust
pub mod mcp;
```

**Step 4: Update update_move and create_move to use validation**

In `src-tauri/src/bin/mcp.rs`, update both tools to validate before saving:

```rust
    #[tool(description = "Update a move's data. Provide complete move object - it will overwrite the existing move file.")]
    async fn update_move(
        &self,
        rmcp::handler::server::tool::Parameters(params): rmcp::handler::server::tool::Parameters<UpdateMoveParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::save_move;
        use d_developmentnethercore_projectframesmith_lib::mcp::validation::validate_move;

        // Validate move data
        if let Err(errors) = validate_move(&params.move_data) {
            let error_messages: Vec<String> = errors
                .iter()
                .map(|e| format!("{}: {}", e.field, e.message))
                .collect();
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Validation errors: {}", error_messages.join("; "))),
                data: None,
            });
        }

        save_move(self.characters_dir.clone(), params.character_id.clone(), params.move_data.clone()).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Successfully updated move '{}' for character '{}'",
            params.move_data.input, params.character_id
        ))]))
    }
```

**Step 5: Verify it compiles**

Run: `cd src-tauri && cargo build --bin mcp`
Expected: Binary compiles successfully

**Step 6: Commit**

```bash
git add src-tauri/src/mcp/ src-tauri/src/lib.rs src-tauri/src/bin/mcp.rs
git commit -m "feat(mcp): add move validation before save"
```

---

## Phase 4: Testing

### Task 13: Add Unit Tests for Validation

**Files:**
- Modify: `src-tauri/src/mcp/validation.rs`

**Step 1: Add test module**

Add at the end of `src-tauri/src/mcp/validation.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FrameHitbox, GuardType, MeterGain, Move, Pushback, Rect};

    fn make_valid_move() -> Move {
        Move {
            input: "5L".to_string(),
            name: "Standing Light".to_string(),
            startup: 7,
            active: 3,
            recovery: 8,
            damage: 30,
            hitstun: 17,
            blockstun: 11,
            hitstop: 6,
            guard: GuardType::Mid,
            hitboxes: vec![FrameHitbox {
                frames: (7, 9),
                r#box: Rect { x: 0, y: -40, w: 30, h: 16 },
            }],
            hurtboxes: vec![],
            pushback: Pushback { hit: 2, block: 2 },
            meter_gain: MeterGain { hit: 5, whiff: 2 },
            animation: "stand_light".to_string(),
        }
    }

    #[test]
    fn test_valid_move_passes() {
        let mv = make_valid_move();
        assert!(validate_move(&mv).is_ok());
    }

    #[test]
    fn test_zero_startup_fails() {
        let mut mv = make_valid_move();
        mv.startup = 0;
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "startup"));
    }

    #[test]
    fn test_zero_active_fails() {
        let mut mv = make_valid_move();
        mv.active = 0;
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "active"));
    }

    #[test]
    fn test_empty_input_fails() {
        let mut mv = make_valid_move();
        mv.input = "".to_string();
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "input"));
    }

    #[test]
    fn test_hitbox_frame_order_fails() {
        let mut mv = make_valid_move();
        mv.hitboxes[0].frames = (10, 5); // End before start
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("hitboxes")));
    }

    #[test]
    fn test_hitbox_exceeds_total_frames_fails() {
        let mut mv = make_valid_move();
        mv.hitboxes[0].frames = (7, 100); // Way beyond total
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("exceeds total frames")));
    }
}
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src-tauri/src/mcp/validation.rs
git commit -m "test(mcp): add unit tests for move validation"
```

---

## Summary

This plan implements a standalone MCP server for Framesmith with:

**Phase 1 (Core):**
- Minimal scaffold with ping tool
- list_characters, get_character, get_move (read operations)
- update_move (write operation)
- get_frame_data_table (utility)

**Phase 2 (Extended):**
- list_moves for quick overview
- get_cancel_table for cancel relationships
- create_move, delete_move for full CRUD
- notation_guide resource

**Phase 3 (Quality):**
- Move validation before save

**Phase 4 (Testing):**
- Unit tests for validation

**Future Phases (not in this plan):**
- Tauri sidecar integration for live GUI sync
- batch_update_moves for bulk operations
- search_moves with filter expressions
- validate_character for full schema validation
- Character CRUD (create, delete, duplicate)
- HTTP/SSE transport for remote access
