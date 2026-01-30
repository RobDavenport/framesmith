use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(name = "framesmith-mcp", about = "Framesmith MCP server for character data")]
struct Args {
    /// Path to the characters directory (overrides FRAMESMITH_CHARACTERS_DIR env var)
    #[arg(long, short = 'c')]
    characters_dir: Option<String>,
}

/// The rules specification documentation (SSOT).
const RULES_SPEC_MD: &str = include_str!("../../../docs/rules-spec.md");

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CharacterIdParam {
    #[schemars(description = "The character ID (e.g., 'test_char')")]
    pub character_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveIdParam {
    #[schemars(description = "The character ID (e.g., 'test_char')")]
    pub character_id: String,
    #[schemars(description = "The move input notation (e.g., '5L', '236P')")]
    pub move_input: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateMoveParam {
    #[schemars(description = "The character ID (e.g., 'test_char')")]
    pub character_id: String,
    #[schemars(description = "Complete move data as JSON object")]
    pub move_data: d_developmentnethercore_projectframesmith_lib::schema::Move,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportCharacterParam {
    #[schemars(description = "The character ID (folder name under characters dir)")]
    pub character_id: String,
    #[schemars(description = "Export adapter: 'zx-fspack' (default) or 'json-blob'")]
    pub adapter: Option<String>,
    #[schemars(description = "Output file path, relative to the project root or absolute under the project root")]
    pub output_path: String,
    #[schemars(description = "Pretty JSON output (json-blob only)")]
    pub pretty: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExportAllCharactersParam {
    #[schemars(description = "Export adapter: 'zx-fspack' (default) or 'json-blob'")]
    pub adapter: Option<String>,
    #[schemars(description = "Output directory, relative to the project root or absolute under the project root")]
    pub out_dir: String,
    #[schemars(description = "Pretty JSON output (json-blob only)")]
    pub pretty: Option<bool>,
    #[schemars(description = "Continue exporting others after an error")]
    pub keep_going: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
pub struct ExportResultRow {
    pub character_id: String,
    pub ok: bool,
    pub output_path: String,
    pub error: Option<String>,
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

    #[tool(description = "Export a character to a file (runs validation + rules). Supports zx-fspack (.fspk) and json-blob (.json).")]
    async fn export_character(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ExportCharacterParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::export_character;

        let adapter = params.adapter.unwrap_or_else(|| "zx-fspack".to_string());
        let pretty = params.pretty.unwrap_or(false);
        if adapter == "zx-fspack" && pretty {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from("pretty=true is only supported for json-blob"),
                data: None,
            });
        }

        let project_root = project_root_from_characters_dir(&self.characters_dir);
        let output_path = resolve_output_path_under_project(&project_root, &params.output_path)
            .map_err(|e| McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(e),
                data: None,
            })?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| McpError {
                code: rmcp::model::ErrorCode::INTERNAL_ERROR,
                message: Cow::from(format!(
                    "Failed to create output directory {}: {}",
                    parent.display(),
                    e
                )),
                data: None,
            })?;
        }

        export_character(
            self.characters_dir.clone(),
            params.character_id,
            adapter,
            output_path.to_string_lossy().to_string(),
            pretty,
        )
        .map_err(|e| {
            let code = if e.starts_with("Invalid ") || e.contains("Validation") {
                rmcp::model::ErrorCode::INVALID_PARAMS
            } else {
                rmcp::model::ErrorCode::INTERNAL_ERROR
            };
            McpError {
                code,
                message: Cow::from(e),
                data: None,
            }
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Exported character to {}",
            output_path.display()
        ))]))
    }

    #[tool(description = "Export all characters to a directory (runs validation + rules). Returns a JSON array of per-character results.")]
    async fn export_all_characters(
        &self,
        rmcp::handler::server::wrapper::Parameters(params): rmcp::handler::server::wrapper::Parameters<ExportAllCharactersParam>,
    ) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::commands::export_character;

        let adapter = params.adapter.unwrap_or_else(|| "zx-fspack".to_string());
        let pretty = params.pretty.unwrap_or(false);
        let keep_going = params.keep_going.unwrap_or(false);
        if adapter == "zx-fspack" && pretty {
            return Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from("pretty=true is only supported for json-blob"),
                data: None,
            });
        }

        let project_root = project_root_from_characters_dir(&self.characters_dir);
        let out_dir = resolve_output_path_under_project(&project_root, &params.out_dir).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INVALID_PARAMS,
            message: Cow::from(e),
            data: None,
        })?;

        std::fs::create_dir_all(&out_dir).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!(
                "Failed to create output directory {}: {}",
                out_dir.display(),
                e
            )),
            data: None,
        })?;

        let ids = find_character_dir_names(&self.characters_dir).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(e),
            data: None,
        })?;

        let ext = adapter_default_ext(&adapter);
        let mut results: Vec<ExportResultRow> = Vec::new();
        for id in ids {
            let out_path = out_dir.join(format!("{}{}", id, ext));
            let res = export_character(
                self.characters_dir.clone(),
                id.clone(),
                adapter.clone(),
                out_path.to_string_lossy().to_string(),
                pretty,
            );

            match res {
                Ok(()) => results.push(ExportResultRow {
                    character_id: id,
                    ok: true,
                    output_path: out_path.to_string_lossy().to_string(),
                    error: None,
                }),
                Err(e) => {
                    results.push(ExportResultRow {
                        character_id: id.clone(),
                        ok: false,
                        output_path: out_path.to_string_lossy().to_string(),
                        error: Some(e),
                    });
                    if !keep_going {
                        break;
                    }
                }
            }
        }

        let json = serde_json::to_string_pretty(&results).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
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
        save_move(
            self.characters_dir.clone(),
            params.character_id.clone(),
            params.move_data.clone(),
        )
        .map_err(|e| {
            let code = if e.starts_with("Validation errors:") || e.starts_with("Invalid ") {
                rmcp::model::ErrorCode::INVALID_PARAMS
            } else {
                rmcp::model::ErrorCode::INTERNAL_ERROR
            };
            McpError {
                code,
                message: Cow::from(e),
                data: None,
            }
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

        save_move(
            self.characters_dir.clone(),
            params.character_id.clone(),
            params.move_data.clone(),
        )
        .map_err(|e| {
            let code = if e.starts_with("Validation errors:") || e.starts_with("Invalid ") {
                rmcp::model::ErrorCode::INVALID_PARAMS
            } else {
                rmcp::model::ErrorCode::INTERNAL_ERROR
            };
            McpError {
                code,
                message: Cow::from(e),
                data: None,
            }
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

    #[tool(description = "Get the JSON Schema for rules files. Use this schema for IDE autocomplete when editing framesmith.rules.json files.")]
    async fn get_rules_schema(&self) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::rules::generate_rules_schema;

        let schema = generate_rules_schema();
        let json = serde_json::to_string_pretty(&schema).map_err(|e| McpError {
            code: rmcp::model::ErrorCode::INTERNAL_ERROR,
            message: Cow::from(format!("Serialization error: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get the list of built-in validation rules that always run on moves. These cannot be disabled.")]
    async fn get_builtin_validations(&self) -> Result<CallToolResult, McpError> {
        use d_developmentnethercore_projectframesmith_lib::rules::get_builtin_validations;

        let validations = get_builtin_validations();
        let json = serde_json::to_string_pretty(&validations).map_err(|e| McpError {
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
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Framesmith MCP server for reading and modifying fighting game character data. \
                 Use list_characters to see available characters, then get_character or get_move \
                 to read data, and update_move to make changes. \
                 Resources: notation_guide explains numpad notation, rules_guide documents the validation rules system. \
                 Tools: get_rules_schema returns JSON Schema for rules files, get_builtin_validations lists always-enforced validations.".to_string(),
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
                }.no_annotation(),
                RawResource {
                    uri: "framesmith://rules_guide".to_string(),
                    name: "Rules Specification".to_string(),
                    title: None,
                    description: Some("Complete documentation for Framesmith's validation rules system (apply rules, validate rules, match criteria, constraints)".to_string()),
                    mime_type: Some("text/markdown".to_string()),
                    size: None,
                    icons: None,
                    meta: None,
                }.no_annotation(),
            ],
        }))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        std::future::ready(match request.uri.as_str() {
            "framesmith://notation_guide" => {
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
            }
            "framesmith://rules_guide" => {
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(RULES_SPEC_MD, &request.uri)],
                })
            }
            _ => Err(McpError {
                code: rmcp::model::ErrorCode::INVALID_PARAMS,
                message: Cow::from(format!("Unknown resource: {}", request.uri)),
                data: None,
            })
        })
    }
}

fn project_root_from_characters_dir(characters_dir: &str) -> PathBuf {
    Path::new(characters_dir)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

fn resolve_output_path_under_project(project_root: &Path, user_path: &str) -> Result<PathBuf, String> {
    if user_path.trim().is_empty() {
        return Err("output path cannot be empty".to_string());
    }

    let p = Path::new(user_path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        project_root.join(p)
    };

    // Canonicalize a stable project root.
    let root_canon = project_root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve project root {}: {}", project_root.display(), e))?;

    // Canonicalize the output parent dir (file may not exist yet).
    let parent = abs
        .parent()
        .ok_or_else(|| "Invalid output path".to_string())?;
    let parent_canon = parent.canonicalize().or_else(|_| {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory {}: {}", parent.display(), e))?;
        parent.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve output directory {}: {}",
                parent.display(),
                e
            )
        })
    })?;

    if !parent_canon.starts_with(&root_canon) {
        return Err(format!(
            "Output path must be under the project root {}",
            root_canon.display()
        ));
    }

    Ok(abs)
}

fn adapter_default_ext(adapter: &str) -> &'static str {
    match adapter {
        "zx-fspack" => ".fspk",
        "json-blob" => ".json",
        _ => ".bin",
    }
}

fn find_character_dir_names(characters_dir: &str) -> Result<Vec<String>, String> {
    let mut ids: Vec<String> = Vec::new();
    let rd = std::fs::read_dir(characters_dir)
        .map_err(|e| format!("Failed to read characters directory {}: {}", characters_dir, e))?;
    for entry in rd {
        let entry = entry.map_err(|e| format!("Failed to read characters directory entry: {}", e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("character.json").exists() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if !name.is_empty() {
                ids.push(name.to_string());
            }
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Priority: CLI arg > env var > default
    let characters_dir = args
        .characters_dir
        .or_else(|| std::env::var("FRAMESMITH_CHARACTERS_DIR").ok())
        .unwrap_or_else(|| "./characters".to_string());

    // Canonicalize the path so relative paths work correctly from any cwd
    let characters_dir = std::fs::canonicalize(&characters_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(characters_dir);

    let service = FramesmithMcp::new(characters_dir).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
