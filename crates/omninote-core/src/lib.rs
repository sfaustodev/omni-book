//! OmniNote core library — vault operations shared by GUI, CLI, and MCP.
//!
//! No UI dependencies (no `egui`, no `eframe`, no `egui_commonmark`). Pure
//! filesystem + serde + markdown parsing. CAD-21 Phase A extracted these
//! modules from the original single-binary crate.
//!
//! Consumers:
//! - `omninote-gui` (egui app)
//! - `omninote-cli` (clap binary `omninote`)
//! - `omninote-mcp` (rmcp stdio server `omninote-mcp`)

pub mod autoformat;
pub mod import;
pub mod pdf;
pub mod resolver;
pub mod types;
pub mod vault;
pub mod wikilinks;
