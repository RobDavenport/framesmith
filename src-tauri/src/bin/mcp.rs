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
