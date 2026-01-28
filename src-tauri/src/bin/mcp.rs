use std::borrow::Cow;

use anyhow::Result;
use serde::Deserialize;
use rmcp::{
    handler::server::tool::ToolRouter,
    model::{
        AnnotateAble, CallToolResult, Content, Implementation, ListResourcesResult,
        PaginatedRequestParams, ProtocolVersion, RawResource, ReadResourceRequestParams,
        ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
    },
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
    transport::stdio,
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CharacterIdParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveIdParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
    #[schemars(description = "The move input notation (e.g., '5L', '236P')")]
    pub move_input: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateMoveParam {
    #[schemars(description = "The character ID (e.g., 'glitch')")]
    pub character_id: String,
    #[schemars(description = "Complete move data as JSON object")]
    pub move_data: d_developmentnethercore_projectframesmith_lib::schema::Move,
}

#[derive(Debug, serde::Serialize)]
pub struct FrameDataRow {
    pub input: String,
    pub name: String,
    pub startup: u8,
    pub active: u8,
    pub recovery: u8,
    pub total: u16,
    pub damage: u16,
    pub hitstun: u8,
    pub blockstun: u8,
    pub advantage_on_hit: i16,
    pub advantage_on_block: i16,
    pub guard: String,
}

#[derive(Debug, Clone)]
pub struct FramesmithMcp {
    #[allow(dead_code)] // Will be used by future tools
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

    #[tool(description = "Get complete character data including properties, all moves, and cancel table")]
    async fn get_character(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<CharacterIdParam>,
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

    #[tool(description = "Get a single move's complete data including hitboxes and frame data")]
    async fn get_move(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<MoveIdParam>,
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

    #[tool(description = "Update a move's data. Provide complete move object - it will overwrite the existing move file.")]
    async fn update_move(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<UpdateMoveParam>,
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

    #[tool(description = "Create a new move for a character. Provide complete move data.")]
    async fn create_move(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<UpdateMoveParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::save_move;
        use d_developmentnethercore_projectframesmith_lib::mcp::validation::validate_move;
        use std::path::Path;

        // Validate move data before checking existence
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

    #[tool(description = "Get a compact frame data table for a character - shows startup, active, recovery, damage, and advantage for all moves")]
    async fn get_frame_data_table(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<CharacterIdParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::load_character;

        let data = load_character(self.characters_dir.clone(), params.character_id).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let rows: Vec<FrameDataRow> = data.moves.iter().map(|m| {
            let total = m.startup as u16 + m.active as u16 + m.recovery as u16;
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

    #[tool(description = "List all moves for a character with basic stats (input, name, startup, damage)")]
    async fn list_moves(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<CharacterIdParam>,
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

    #[tool(description = "Get the cancel table showing all cancel relationships (chains, special cancels, super cancels, jump cancels)")]
    async fn get_cancel_table(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<CharacterIdParam>,
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

    #[tool(description = "Delete a move from a character")]
    async fn delete_move(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<MoveIdParam>,
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

        // Validate move_input
        if params.move_input.contains("..") || params.move_input.contains('/') || params.move_input.contains('\\') {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from("Invalid move input"),
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
}

#[tool_handler]
impl ServerHandler for FramesmithMcp {
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

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult {
            meta: None,
            next_cursor: None,
            resources: vec![
                RawResource {
                    uri: "framesmith://notation_guide".to_string(),
                    name: "Numpad Notation Guide".to_string(),
                    title: None,
                    description: Some("Reference for fighting game numpad notation (236 = QCF, etc.)".to_string()),
                    mime_type: Some("text/markdown".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                }.no_annotation()
            ],
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        std::future::ready(if request.uri == "framesmith://notation_guide" {
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
            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(guide, &request.uri)],
            })
        } else {
            Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Unknown resource: {}", request.uri)),
                data: None,
            })
        })
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
