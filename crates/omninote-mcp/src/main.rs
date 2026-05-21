//! OmniNote MCP server stub (CAD-21 Phase C).
//!
//! Wired in Phase C: exposes `vault_info`, `note_search`, `link_unresolved`,
//! `link_backlinks` as MCP tools via `rmcp` (Anthropic Rust MCP SDK). For now
//! this is a placeholder binary that compiles and prints a helpful message.

fn main() -> anyhow::Result<()> {
    eprintln!("omninote-mcp: scaffold only — Phase C wires rmcp tool handlers");
    eprintln!("see Notion CAD-21 for the verb list");
    std::process::exit(0);
}
