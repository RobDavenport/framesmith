//! Portable authoring build: the desktop and browser compile the same source.
//! File-loading entrypoints remain optional; the lab uses only in-memory data.
pub mod codegen;
pub mod globals;
pub mod rules;
pub mod schema;
pub mod variant;
// These modules are pure validators, not the MCP server or transport.
pub mod mcp {
    pub mod validation;
    pub mod validators;
}
