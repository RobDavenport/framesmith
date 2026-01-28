use std::borrow::Cow;

use anyhow::Result;
use serde::Deserialize;
use rmcp::{
    handler::server::tool::ToolRouter,
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
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
