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
//! - `daily_ensure` — create today's daily note (CAD-22)
//! - `template_list` — list templates under `<vault>/Templates/` (CAD-22)
//! - `template_apply` — render a template by name (CAD-22)
//! - `diary_append` — quick-append entry to DIARY.md (CAD-22)
//! - `human_ask` — open question in HUMAN.md (auto Q-NN) (CAD-22)
//! - `ticket_status` — find ticket in NOTION.md / JIRA.md (CAD-22)
//! - `discipline_show` — dump raw content of a discipline file (CAD-22)
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

// ───── CAD-22 — daily / templates / discipline params + outputs ─────

#[derive(Debug, Deserialize, JsonSchema)]
struct DailyEnsureParams {
    /// Date override in `YYYY-MM-DD`. Default: today (local TZ).
    #[serde(default)]
    date: Option<String>,
    /// Template name (no `.md`). Default: `daily`.
    #[serde(default)]
    template: Option<String>,
    /// Folder relative to vault. Default: `Daily`.
    #[serde(default)]
    folder: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DailyEnsureOutput {
    path: PathBuf,
    rel_path: PathBuf,
    created: bool,
    template_used: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TemplateMetaOut {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TemplateListOutput {
    templates: Vec<TemplateMetaOut>,
    count: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TemplateApplyParams {
    /// Template name (no `.md`).
    name: String,
    /// Title to substitute into `{{title}}`.
    #[serde(default)]
    title: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TemplateApplyOutput {
    rendered: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DiaryAppendParams {
    /// Entry text (single line — wrap multi-line in `\n`).
    text: String,
    /// Optional ticket reference (e.g. `CAD-22`).
    #[serde(default)]
    ticket: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DiaryAppendOutput {
    path: PathBuf,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct HumanAskParams {
    /// The question (pt-BR recommended).
    question: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct HumanAskOutput {
    path: PathBuf,
    q_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TicketStatusParams {
    /// Ticket ID like `CAD-22`, `SCRUM-157`. Word-bounded match.
    ticket_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TicketStatusOutput {
    ticket_id: String,
    file: PathBuf,
    line_no: usize,
    paragraph: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DisciplineShowParams {
    /// One of: `diary`, `sprint`, `human`, `plan`, `jira`, `notion`, `eternal`.
    file: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DisciplineShowOutput {
    file: String,
    content: String,
}

// ───── CAD-23.1 — vault_ask (RAG retrieve + optional LLM answer) ─────

#[derive(Debug, Deserialize, JsonSchema)]
struct VaultAskParams {
    /// The user's question.
    query: String,
    /// Top-k passages to retrieve. Default 5.
    #[serde(default = "default_top_k")]
    top_k: usize,
    /// If true, skip the LLM call and only return passages.
    #[serde(default)]
    no_llm: bool,
    /// Override the Anthropic model id (default from llm.toml).
    #[serde(default)]
    model: Option<String>,
}

fn default_top_k() -> usize {
    5
}

#[derive(Debug, Serialize, JsonSchema)]
struct AskPassage {
    note_id: String,
    chunk_idx: usize,
    wikilink: String,
    score: f32,
    text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct VaultAskOutput {
    query: String,
    passages: Vec<AskPassage>,
    /// Present when `no_llm=false` and at least one passage was found.
    answer: Option<String>,
    indexed_chunks: usize,
    embed_calls: usize,
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

    // ───── CAD-22 — daily + templates + discipline tools ─────

    #[tool(
        name = "daily_ensure",
        description = "Creates today's daily note `<vault>/<folder>/YYYY-MM-DD.md` if missing, rendered from `Templates/<template>.md` when available. Idempotent — returns `created: false` plus the path if the file already exists. Safe to call multiple times per day."
    )]
    async fn daily_ensure(
        &self,
        Parameters(params): Parameters<DailyEnsureParams>,
    ) -> Result<Json<DailyEnsureOutput>, ErrorData> {
        let date = params
            .date
            .as_deref()
            .map(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d"))
            .transpose()
            .map_err(|e| {
                ErrorData::invalid_params(
                    format!(
                        "invalid date '{:?}' (expected YYYY-MM-DD): {e}",
                        params.date
                    ),
                    None,
                )
            })?;
        let opts = omninote_core::daily::DailyOpts {
            date,
            template_name: params.template,
            folder: params.folder.unwrap_or_else(|| "Daily".into()),
        };
        let res = omninote_core::daily::ensure_daily(&self.vault_root, opts)
            .map_err(|e| ErrorData::internal_error(format!("daily_ensure: {e}"), None))?;
        Ok(Json(DailyEnsureOutput {
            path: res.path,
            rel_path: res.rel_path,
            created: res.created,
            template_used: res.template_used,
        }))
    }

    #[tool(
        name = "template_list",
        description = "Lists all `*.md` templates under `<vault>/Templates/`. Returns empty array if folder absent."
    )]
    async fn template_list(
        &self,
        Parameters(_): Parameters<EmptyParams>,
    ) -> Result<Json<TemplateListOutput>, ErrorData> {
        let list = omninote_core::templates::list_templates(&self.vault_root);
        let count = list.len();
        Ok(Json(TemplateListOutput {
            templates: list
                .into_iter()
                .map(|t| TemplateMetaOut {
                    name: t.name,
                    path: t.path,
                })
                .collect(),
            count,
        }))
    }

    #[tool(
        name = "template_apply",
        description = "Renders `<vault>/Templates/<name>.md` substituting `{{date}}`, `{{time}}`, `{{title}}`, plus arbitrary `{{KEY}}` placeholders. Returns the rendered text — does NOT write a file."
    )]
    async fn template_apply(
        &self,
        Parameters(params): Parameters<TemplateApplyParams>,
    ) -> Result<Json<TemplateApplyOutput>, ErrorData> {
        let body = omninote_core::templates::load_template(&self.vault_root, &params.name)
            .map_err(|e| ErrorData::invalid_params(format!("template: {e}"), None))?;
        let ctx = omninote_core::templates::TemplateContext::now(params.title);
        let rendered = omninote_core::templates::render(&body, &ctx);
        Ok(Json(TemplateApplyOutput { rendered }))
    }

    #[tool(
        name = "diary_append",
        description = "Appends a quick entry to `<vault>/discipline/DIARY.md` (or `<vault>/DIARY.md` as fallback). Entry is prepended to the top (newest first). Optional `ticket` reference adds a `**Tickets touched:** <ID>` line."
    )]
    async fn diary_append(
        &self,
        Parameters(params): Parameters<DiaryAppendParams>,
    ) -> Result<Json<DiaryAppendOutput>, ErrorData> {
        let path = omninote_core::discipline::diary_quick(
            &self.vault_root,
            &params.text,
            params.ticket.as_deref(),
        )
        .map_err(|e| ErrorData::internal_error(format!("diary_append: {e}"), None))?;
        Ok(Json(DiaryAppendOutput { path }))
    }

    #[tool(
        name = "human_ask",
        description = "Adds a new open question to `<vault>/discipline/HUMAN.md`, auto-numbering it Q-NN under '## Open questions'. Use for irreversible decisions, external contracts, security questions, or SPRINT conflicts that need human input."
    )]
    async fn human_ask(
        &self,
        Parameters(params): Parameters<HumanAskParams>,
    ) -> Result<Json<HumanAskOutput>, ErrorData> {
        let (path, q_id) = omninote_core::discipline::human_ask(&self.vault_root, &params.question)
            .map_err(|e| ErrorData::internal_error(format!("human_ask: {e}"), None))?;
        Ok(Json(HumanAskOutput { path, q_id }))
    }

    #[tool(
        name = "ticket_status",
        description = "Looks up a ticket ID (e.g. `CAD-22`, `SCRUM-157`) in `<vault>/discipline/NOTION.md` first, then `JIRA.md`. Word-bounded match — `CAD-2` won't match `CAD-22`. Returns the surrounding paragraph + line number, or an error if missing."
    )]
    async fn ticket_status(
        &self,
        Parameters(params): Parameters<TicketStatusParams>,
    ) -> Result<Json<TicketStatusOutput>, ErrorData> {
        let t = omninote_core::discipline::ticket_status(&self.vault_root, &params.ticket_id)
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("ticket not found: {}", params.ticket_id), None)
            })?;
        Ok(Json(TicketStatusOutput {
            ticket_id: t.ticket_id,
            file: t.file,
            line_no: t.line_no,
            paragraph: t.paragraph,
        }))
    }

    #[tool(
        name = "discipline_show",
        description = "Dumps the raw markdown of a discipline file. `file` is one of: diary, sprint, human, plan, jira, notion, eternal. Searches `<vault>/discipline/<FILE>` first, then `<vault>/<FILE>`."
    )]
    async fn discipline_show(
        &self,
        Parameters(params): Parameters<DisciplineShowParams>,
    ) -> Result<Json<DisciplineShowOutput>, ErrorData> {
        let f = omninote_core::discipline::DisciplineFile::from_slug(&params.file).ok_or_else(
            || {
                ErrorData::invalid_params(
                    format!(
                        "unknown discipline file '{}' — try: diary|sprint|human|plan|jira|notion|eternal",
                        params.file
                    ),
                    None,
                )
            },
        )?;
        let content = omninote_core::discipline::read_raw(&self.vault_root, f)
            .map_err(|e| ErrorData::internal_error(format!("discipline_show: {e}"), None))?;
        Ok(Json(DisciplineShowOutput {
            file: f.filename().to_string(),
            content,
        }))
    }

    #[tool(
        name = "vault_ask",
        description = "Semantic search over the vault using local embeddings (fastembed BGE small, 384d). Returns top-k passages with `[[wikilink]]` citations. By default also calls Claude with the passages to synthesize an answer; set `no_llm=true` to return passages only and save tokens. The embedding index is cached at `<vault>/.omninote/embeddings.bin` and refreshed incrementally — only changed chunks are re-embedded."
    )]
    async fn vault_ask(
        &self,
        Parameters(params): Parameters<VaultAskParams>,
    ) -> Result<Json<VaultAskOutput>, ErrorData> {
        use omninote_ai::LlmProvider;

        let vault = self.open_vault()?;

        let embedder = omninote_ai::FastEmbedder::bge_small()
            .map_err(|e| ErrorData::internal_error(format!("fastembed init: {e}"), None))?;
        let index_path = omninote_ai::EmbeddingIndex::default_path(&self.vault_root);
        let existing = omninote_ai::EmbeddingIndex::load(&index_path)
            .map_err(|e| ErrorData::internal_error(format!("load embeddings: {e}"), None))?;
        let mut rag = omninote_ai::Rag::with_index(embedder, existing);

        let mut embed_calls = 0usize;
        for note in &vault.notes {
            if let Ok(n) = rag.upsert_note(&note.frontmatter.id, &note.content) {
                embed_calls += n;
            }
        }
        let alive: std::collections::HashSet<&str> = vault
            .notes
            .iter()
            .map(|n| n.frontmatter.id.as_str())
            .collect();
        let stale: Vec<String> = rag
            .index
            .entries
            .iter()
            .map(|c| c.note_id.clone())
            .filter(|id| !alive.contains(id.as_str()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        for id in &stale {
            rag.forget_note(id);
        }
        rag.index
            .save(&index_path)
            .map_err(|e| ErrorData::internal_error(format!("save embeddings: {e}"), None))?;

        let hits = rag
            .retrieve(&params.query, params.top_k)
            .map_err(|e| ErrorData::internal_error(format!("retrieve: {e}"), None))?;

        let passages: Vec<AskPassage> = hits
            .iter()
            .map(|h| {
                let wikilink = vault
                    .notes
                    .iter()
                    .find(|n| n.frontmatter.id == h.note_id)
                    .map(|n| format!("[[{}]]", n.title))
                    .unwrap_or_else(|| format!("[[{}]]", h.note_id));
                AskPassage {
                    note_id: h.note_id.clone(),
                    chunk_idx: h.chunk_idx,
                    wikilink,
                    score: h.score,
                    text: h.chunk_text.clone(),
                }
            })
            .collect();

        let answer = if params.no_llm || passages.is_empty() {
            None
        } else {
            let cfg = omninote_ai::LlmConfig::load()
                .map_err(|e| ErrorData::internal_error(format!("load llm.toml: {e}"), None))?;
            let key = cfg
                .anthropic_key()
                .map_err(|e| ErrorData::invalid_params(format!("api key: {e}"), None))?;
            let provider = omninote_ai::AnthropicProvider::new(key);
            let opts = omninote_ai::CompleteOpts {
                model: params.model.clone().unwrap_or(cfg.provider.model.clone()),
                max_tokens: cfg.provider.max_tokens,
                ..Default::default()
            };
            let mut passages_str = String::new();
            for (idx, p) in passages.iter().enumerate() {
                passages_str.push_str(&format!(
                    "{}. {} (score {:.2})\n{}\n\n",
                    idx + 1,
                    p.wikilink,
                    p.score,
                    p.text
                ));
            }
            let system = "You answer questions about the user's personal note vault. \
                          Cite supporting notes inline as [[wikilinks]] using the labels in the passages. \
                          If the passages don't answer the question, say so plainly.";
            let user = format!(
                "Passages:\n\n{passages_str}---\n\nQuestion: {}",
                params.query
            );
            let text = provider
                .complete(system, &user, opts)
                .await
                .map_err(|e| ErrorData::internal_error(format!("llm: {e}"), None))?;
            Some(text)
        };

        Ok(Json(VaultAskOutput {
            query: params.query,
            passages,
            answer,
            indexed_chunks: rag.index.entries.len(),
            embed_calls,
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
            "OmniNote MCP server. Vault: {}. Tools: vault_info, note_search, link_unresolved, link_backlinks, daily_ensure, template_list, template_apply, diary_append, human_ask, ticket_status, discipline_show, vault_ask. Vault is re-scanned per call.",
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
