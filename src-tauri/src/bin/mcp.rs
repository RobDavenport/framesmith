use anyhow::Result;
use rmcp::{
    handler::server::tool::ToolRouter,
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
    transport::stdio,
};

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
