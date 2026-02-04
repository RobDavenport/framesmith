use anyhow::Result;
use clap::Parser;
use framesmith_lib::mcp::handlers::FramesmithMcp;
use rmcp::{ServiceExt, transport::stdio};

#[derive(Parser, Debug)]
#[command(name = "framesmith-mcp", about = "Framesmith MCP server for character data")]
struct Args {
    /// Path to the characters directory (overrides FRAMESMITH_CHARACTERS_DIR env var)
    #[arg(long, short = 'c')]
    characters_dir: Option<String>,
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
