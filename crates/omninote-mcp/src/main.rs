//! OmniNote MCP server — exposes vault tools via stdio. CAD-21 Phase C.
//!
//! Wire up in Claude Desktop config:
//!
//! ```jsonc
//! {
//!   "mcpServers": {
//!     "omninote": {
//!       "command": "/abs/path/to/omninote-mcp",
//!       "env": { "OMNINOTE_VAULT": "/abs/path/to/vault" }
//!     }
//!   }
//! }
//! ```
//!
//! Tools exposed:
//! - `vault_info` — vault path, note count, index counts
//! - `note_search` — substring search across notes (or titles only)
//! - `link_unresolved` — list wikilinks that don't resolve
//! - `link_backlinks` — list notes that link to a given target
//!
//! Vault resolved at startup: `OMNINOTE_VAULT` env → `~/.config/omninote/last_vault`.
//! Vault is re-opened per tool call (fresh scan) — Phase 4 may cache.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::schemars::JsonSchema;
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ErrorData, Json, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
struct OmniNoteMcp {
    vault_root: Arc<PathBuf>,
    // Used by `#[tool_handler]` macro via field access — `#[allow(dead_code)]`
    // because the macro-generated reads aren't visible to the dead-code lint.
    #[allow(dead_code)]
    tool_router: ToolRouter<OmniNoteMcp>,
}

impl OmniNoteMcp {
    fn new(vault_root: PathBuf) -> Self {
        Self {
            vault_root: Arc::new(vault_root),
            tool_router: Self::tool_router(),
        }
    }

    fn open_vault(&self) -> Result<omninote_core::vault::Vault, ErrorData> {
        omninote_core::vault::Vault::open((*self.vault_root).clone())
            .map_err(|e| ErrorData::internal_error(format!("vault open failed: {e}"), None))
    }
}

// -------- tool parameter / output types --------

#[derive(Debug, Deserialize, JsonSchema, Default)]
struct EmptyParams {}

#[derive(Debug, Serialize, JsonSchema)]
struct VaultInfoOutput {
    path: PathBuf,
    notes: usize,
    index_files: usize,
    index_paths: usize,
    index_aliases: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NoteSearchParams {
    /// Search query (substring match).
    query: String,
    /// Case-sensitive match (default false).
    #[serde(default)]
    case_sensitive: bool,
    /// Max hits (default 50).
    #[serde(default = "default_limit")]
    limit: usize,
    /// Search only note titles (default false = search bodies).
    #[serde(default)]
    titles_only: bool,
}

fn default_limit() -> usize {
    50
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchHit {
    rel_path: PathBuf,
    title: String,
    line_no: usize,
    snippet: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct NoteSearchOutput {
    hits: Vec<SearchHit>,
    count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UnresolvedLink {
    target: String,
    source: PathBuf,
}

#[derive(Debug, Serialize, JsonSchema)]
struct LinkUnresolvedOutput {
    unresolved: Vec<UnresolvedLink>,
    count: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct LinkBacklinksParams {
    /// File reference: filename, path, or alias. Resolved like a `[[wikilink]]`.
    file: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Backlink {
    source: PathBuf,
    anchor: Option<String>,
    is_embed: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct LinkBacklinksOutput {
    target: PathBuf,
    backlinks: Vec<Backlink>,
    count: usize,
}

// -------- tool implementations --------

#[tool_router]
impl OmniNoteMcp {
    #[tool(
        name = "vault_info",
        description = "Returns vault path, note count, and resolver index statistics for the configured OmniNote vault."
    )]
    async fn vault_info(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<Json<VaultInfoOutput>, ErrorData> {
        let vault = self.open_vault()?;
        let stats = vault.index.stats();
        Ok(Json(VaultInfoOutput {
            path: vault.root.clone(),
            notes: vault.notes.len(),
            index_files: stats.files,
            index_paths: stats.paths,
            index_aliases: stats.aliases,
        }))
    }

    #[tool(
        name = "note_search",
        description = "Substring search across note content. Returns matching lines with snippet, file path, and line number. Use `titles_only=true` for quick-switcher style title-only search."
    )]
    async fn note_search(
        &self,
        Parameters(params): Parameters<NoteSearchParams>,
    ) -> Result<Json<NoteSearchOutput>, ErrorData> {
        let vault = self.open_vault()?;
        let opts = omninote_core::search::SearchOpts {
            case_sensitive: params.case_sensitive,
            limit: Some(params.limit),
        };
        let hits = if params.titles_only {
            omninote_core::search::search_titles(&vault.notes, &params.query, opts)
        } else {
            omninote_core::search::search(&vault.notes, &params.query, opts)
        };
        let count = hits.len();
        Ok(Json(NoteSearchOutput {
            hits: hits
                .into_iter()
                .map(|h| SearchHit {
                    rel_path: h.rel_path,
                    title: h.title,
                    line_no: h.line_no,
                    snippet: h.snippet,
                })
                .collect(),
            count,
        }))
    }

    #[tool(
        name = "link_unresolved",
        description = "Lists every `[[wikilink]]` in the vault whose target does not match any existing note. Useful for finding broken refs."
    )]
    async fn link_unresolved(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<Json<LinkUnresolvedOutput>, ErrorData> {
        let vault = self.open_vault()?;
        let unresolved = vault.index.unresolved_links(&vault.notes);
        let count = unresolved.len();
        Ok(Json(LinkUnresolvedOutput {
            unresolved: unresolved
                .into_iter()
                .map(|u| UnresolvedLink {
                    target: u.target,
                    source: u.source,
                })
                .collect(),
            count,
        }))
    }

    #[tool(
        name = "link_backlinks",
        description = "Lists every note that links TO the given file via `[[wikilink]]` or `![[embed]]`. File is resolved using the same rules as wikilinks (filename / path / frontmatter alias / case-insensitive)."
    )]
    async fn link_backlinks(
        &self,
        Parameters(params): Parameters<LinkBacklinksParams>,
    ) -> Result<Json<LinkBacklinksOutput>, ErrorData> {
        let vault = self.open_vault()?;
        let target = vault.index.resolve(&params.file).cloned().ok_or_else(|| {
            ErrorData::invalid_params(
                format!("file does not match any note in vault: {}", params.file),
                None,
            )
        })?;
        let backlinks = vault.index.backlinks_to(&target, &vault.notes);
        let count = backlinks.len();
        Ok(Json(LinkBacklinksOutput {
            target,
            backlinks: backlinks
                .into_iter()
                .map(|b| Backlink {
                    source: b.source,
                    anchor: format_anchor(&b.anchor),
                    is_embed: b.is_embed,
                })
                .collect(),
            count,
        }))
    }
}

fn format_anchor(a: &Option<omninote_core::wikilinks::Anchor>) -> Option<String> {
    use omninote_core::wikilinks::Anchor;
    a.as_ref().map(|x| match x {
        Anchor::Heading(h) => format!("#{h}"),
        Anchor::Block(b) => format!("#^{b}"),
    })
}

#[tool_handler]
impl ServerHandler for OmniNoteMcp {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` is `#[non_exhaustive]`; mutate Default's fields.
        let mut info = ServerInfo::default();
        info.instructions = Some(format!(
            "OmniNote MCP server. Vault: {}. Tools: vault_info, note_search, link_unresolved, link_backlinks. Vault is re-scanned per call.",
            self.vault_root.display()
        ));
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

fn resolve_vault() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OMNINOTE_VAULT") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let last = dirs::config_dir()?.join("omninote").join("last_vault");
    std::fs::read_to_string(last)
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| p.exists())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let vault_root = resolve_vault().ok_or_else(|| {
        anyhow::anyhow!(
            "no vault: set OMNINOTE_VAULT env or open the GUI once to seed ~/.config/omninote/last_vault"
        )
    })?;
    eprintln!("omninote-mcp starting · vault: {}", vault_root.display());
    let server = OmniNoteMcp::new(vault_root);
    let transport = stdio();
    let running = server.serve(transport).await?;
    running.waiting().await?;
    Ok(())
}
