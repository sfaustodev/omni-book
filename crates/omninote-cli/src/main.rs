//! OmniNote CLI — vault ops from the terminal.
//!
//! Vault resolution order: `--vault <PATH>` → `OMNINOTE_VAULT` env →
//! `~/.config/omninote/last_vault` file. Mirrors the GUI's vault picker.
//!
//! Verbs (CAD-22 added daily/template/diary/human/ticket/discipline):
//! ```text
//! omninote-cli vault info
//! omninote-cli note search QUERY [--case] [--limit N] [--titles-only]
//! omninote-cli link unresolved
//! omninote-cli link backlinks FILE
//! omninote-cli daily [--date YYYY-MM-DD] [--template NAME] [--folder Daily]
//! omninote-cli template list
//! omninote-cli template apply NAME [--title TITLE] [--out PATH]
//! omninote-cli diary append TEXT [--ticket CAD-XX]
//! omninote-cli human ask QUESTION
//! omninote-cli ticket ID
//! omninote-cli discipline show FILE
//! ```
//! Every verb accepts `--json` for machine-readable output (envelope:
//! `{ok, data, meta?}` or `{ok: false, error}`).

use clap::{Parser, Subcommand};
use omninote_ai::LlmProvider; // brings trait `complete` method into scope
use omninote_core::discipline::DisciplineFile;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "omninote", version, about = "OmniNote CLI", long_about = None)]
struct Cli {
    /// Vault root directory.
    #[arg(long, env = "OMNINOTE_VAULT", global = true)]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Vault inspection.
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },
    /// Note operations.
    Note {
        #[command(subcommand)]
        action: NoteAction,
    },
    /// Link analysis.
    Link {
        #[command(subcommand)]
        action: LinkAction,
    },
    /// Open/create today's daily note (CAD-22).
    Daily {
        /// Override date (YYYY-MM-DD). Default: today.
        #[arg(long)]
        date: Option<String>,
        /// Template name to render (without `.md`). Default: `daily`.
        #[arg(long)]
        template: Option<String>,
        /// Folder relative to vault. Default: `Daily`.
        #[arg(long, default_value = "Daily")]
        folder: String,
        #[arg(long)]
        json: bool,
    },
    /// Template operations (CAD-22).
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Append entry to discipline DIARY.md (CAD-22).
    Diary {
        #[command(subcommand)]
        action: DiaryAction,
    },
    /// Open question in discipline HUMAN.md (CAD-22).
    Human {
        #[command(subcommand)]
        action: HumanAction,
    },
    /// Look up ticket status in NOTION.md / JIRA.md (CAD-22).
    Ticket {
        ticket_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Show raw content of a discipline file (CAD-22).
    Discipline {
        #[command(subcommand)]
        action: DisciplineAction,
    },
    /// Auto-suggest tags + 1-line summary via LLM (CAD-23.2).
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },
    /// Ask the vault — semantic retrieval + LLM completion (CAD-23.1).
    Ask {
        /// The question — wrap in quotes for multi-word queries.
        query: String,
        /// Top-k passages to retrieve from the local embedding index.
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        /// Skip the LLM call and just print the retrieved passages.
        /// Useful for debugging retrieval quality without spending tokens.
        #[arg(long)]
        no_llm: bool,
        /// Anthropic model id to use (overrides config).
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum VaultAction {
    Info {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum NoteAction {
    Search {
        query: String,
        #[arg(long)]
        case: bool,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        titles_only: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum LinkAction {
    Unresolved {
        #[arg(long)]
        json: bool,
    },
    Backlinks {
        file: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TemplateAction {
    /// List available templates under `<vault>/Templates/`.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Render a template with optional title. Prints rendered text to
    /// stdout (or writes to `--out` path).
    Apply {
        name: String,
        #[arg(long, default_value = "")]
        title: String,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DiaryAction {
    /// Append a quick entry to DIARY.md (prepended to top).
    Append {
        /// Entry body (single line; wrap in quotes for spaces).
        text: String,
        /// Optional ticket reference, e.g. `CAD-22`.
        #[arg(long)]
        ticket: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum HumanAction {
    /// Ask a new question (auto-numbers Q-NN under "Open questions").
    Ask {
        question: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DisciplineAction {
    /// Dump raw content. FILE is one of: diary|sprint|human|plan|jira|notion|eternal.
    Show {
        file: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TagAction {
    /// Suggest tags + summary for FILE via LLM. Without `--apply` only prints
    /// the proposed diff. FILE is resolved by filename / path / alias (same
    /// rules as wikilinks).
    Auto {
        file: String,
        /// Write the suggested frontmatter back to disk.
        #[arg(long)]
        apply: bool,
        /// Max total tags (current + suggested merged + capped). Default 5.
        #[arg(long, default_value_t = 5)]
        max_tags: usize,
        /// Truncate the note body to this many chars before sending to LLM.
        /// Default 6000.
        #[arg(long, default_value_t = 6000)]
        max_input_chars: usize,
        /// Replace existing tags entirely instead of merging additively.
        #[arg(long)]
        replace: bool,
        /// Override the LLM model id (else uses llm.toml).
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

fn format_anchor(a: &Option<omninote_core::wikilinks::Anchor>) -> Option<String> {
    use omninote_core::wikilinks::Anchor;
    a.as_ref().map(|x| match x {
        Anchor::Heading(h) => format!("#{h}"),
        Anchor::Block(b) => format!("#^{b}"),
    })
}

fn resolve_vault(arg: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = arg {
        return Some(p);
    }
    let last = dirs::config_dir()?.join("omninote").join("last_vault");
    std::fs::read_to_string(last)
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| p.exists())
}

fn parse_naive_date(s: &str) -> anyhow::Result<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("invalid date '{s}' (expected YYYY-MM-DD): {e}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let vault_root = resolve_vault(cli.vault.clone()).ok_or_else(|| {
        anyhow::anyhow!("no vault: pass --vault, set OMNINOTE_VAULT, or open the GUI once")
    })?;

    match cli.command {
        Command::Vault { action } => match action {
            VaultAction::Info { json } => {
                let vault = omninote_core::vault::Vault::open(vault_root.clone())
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let stats = vault.index.stats();
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": {
                            "path": vault.root,
                            "files": vault.notes.len(),
                            "index_files": stats.files,
                            "index_paths": stats.paths,
                            "index_aliases": stats.aliases,
                        }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    println!("vault: {}", vault.root.display());
                    println!("notes: {}", vault.notes.len());
                    println!(
                        "index: {} files / {} paths / {} aliases",
                        stats.files, stats.paths, stats.aliases
                    );
                }
            }
        },
        Command::Note { action } => match action {
            NoteAction::Search {
                query,
                case,
                limit,
                titles_only,
                json,
            } => {
                let vault = omninote_core::vault::Vault::open(vault_root.clone())
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let opts = omninote_core::search::SearchOpts {
                    case_sensitive: case,
                    limit,
                };
                let hits = if titles_only {
                    omninote_core::search::search_titles(&vault.notes, &query, opts)
                } else {
                    omninote_core::search::search(&vault.notes, &query, opts)
                };
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": hits.iter().map(|h| serde_json::json!({
                            "rel_path": h.rel_path,
                            "title": h.title,
                            "line_no": h.line_no,
                            "snippet": h.snippet,
                        })).collect::<Vec<_>>(),
                        "meta": { "count": hits.len(), "query": query }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    if hits.is_empty() {
                        println!("no matches for: {query}");
                    }
                    for h in &hits {
                        if h.line_no == 0 {
                            println!("{}", h.rel_path.display());
                        } else {
                            println!("{}:{}: {}", h.rel_path.display(), h.line_no, h.snippet);
                        }
                    }
                }
            }
        },
        Command::Link { action } => match action {
            LinkAction::Unresolved { json } => {
                let vault = omninote_core::vault::Vault::open(vault_root.clone())
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let unresolved = vault.index.unresolved_links(&vault.notes);
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": unresolved.iter().map(|u| serde_json::json!({
                            "target": u.target,
                            "source": u.source,
                        })).collect::<Vec<_>>(),
                        "meta": { "count": unresolved.len() }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    println!("{} unresolved", unresolved.len());
                    for u in &unresolved {
                        println!("  {} ← {}", u.target, u.source.display());
                    }
                }
            }
            LinkAction::Backlinks { file, json } => {
                let vault = omninote_core::vault::Vault::open(vault_root.clone())
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let target = vault.index.resolve(&file).cloned().ok_or_else(|| {
                    anyhow::anyhow!("file does not match any note in vault: {file}")
                })?;
                let backlinks = vault.index.backlinks_to(&target, &vault.notes);
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": backlinks.iter().map(|b| serde_json::json!({
                            "source": b.source,
                            "is_embed": b.is_embed,
                            "anchor": format_anchor(&b.anchor),
                        })).collect::<Vec<_>>(),
                        "meta": { "count": backlinks.len(), "target": target }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    println!("{} backlinks → {}", backlinks.len(), target.display());
                    for b in &backlinks {
                        let kind = if b.is_embed { "![[ ]]" } else { "[[ ]]" };
                        let anch = format_anchor(&b.anchor);
                        println!(
                            "  {} {} {}",
                            b.source.display(),
                            kind,
                            anch.unwrap_or_default()
                        );
                    }
                }
            }
        },

        // ────────── CAD-22 verbs ──────────
        Command::Daily {
            date,
            template,
            folder,
            json,
        } => {
            let opts = omninote_core::daily::DailyOpts {
                date: date.as_deref().map(parse_naive_date).transpose()?,
                template_name: template,
                folder,
            };
            let res = omninote_core::daily::ensure_daily(&vault_root, opts)
                .map_err(|e| anyhow::anyhow!("daily: {e}"))?;
            if json {
                let out = serde_json::json!({
                    "ok": true,
                    "data": {
                        "path": res.path,
                        "rel_path": res.rel_path,
                        "created": res.created,
                        "template_used": res.template_used,
                    }
                });
                println!("{}", serde_json::to_string(&out)?);
            } else {
                println!(
                    "{} {}",
                    if res.created { "created" } else { "exists" },
                    res.path.display()
                );
                if let Some(t) = &res.template_used {
                    println!("template: {t}");
                }
            }
        }
        Command::Template { action } => match action {
            TemplateAction::List { json } => {
                let list = omninote_core::templates::list_templates(&vault_root);
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": list.iter().map(|t| serde_json::json!({
                            "name": t.name,
                            "path": t.path,
                        })).collect::<Vec<_>>(),
                        "meta": { "count": list.len() }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    if list.is_empty() {
                        println!("no templates in <vault>/Templates/");
                    }
                    for t in &list {
                        println!("{}  {}", t.name, t.path.display());
                    }
                }
            }
            TemplateAction::Apply {
                name,
                title,
                out,
                json,
            } => {
                let body = omninote_core::templates::load_template(&vault_root, &name)
                    .map_err(|e| anyhow::anyhow!("template: {e}"))?;
                let ctx = omninote_core::templates::TemplateContext::now(title);
                let rendered = omninote_core::templates::render(&body, &ctx);
                if let Some(path) = out.as_ref() {
                    std::fs::write(path, &rendered)
                        .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
                }
                if json {
                    let out_json = serde_json::json!({
                        "ok": true,
                        "data": {
                            "rendered": rendered,
                            "wrote_to": out,
                        }
                    });
                    println!("{}", serde_json::to_string(&out_json)?);
                } else if out.is_none() {
                    print!("{rendered}");
                } else {
                    println!("wrote: {}", out.as_ref().unwrap().display());
                }
            }
        },
        Command::Diary { action } => match action {
            DiaryAction::Append { text, ticket, json } => {
                let path =
                    omninote_core::discipline::diary_quick(&vault_root, &text, ticket.as_deref())
                        .map_err(|e| anyhow::anyhow!("diary append: {e}"))?;
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": { "path": path }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    println!("appended to {}", path.display());
                }
            }
        },
        Command::Human { action } => match action {
            HumanAction::Ask { question, json } => {
                let (path, qn) = omninote_core::discipline::human_ask(&vault_root, &question)
                    .map_err(|e| anyhow::anyhow!("human ask: {e}"))?;
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": { "path": path, "q_id": qn }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    println!("{} added to {}", qn, path.display());
                }
            }
        },
        Command::Ticket { ticket_id, json } => {
            match omninote_core::discipline::ticket_status(&vault_root, &ticket_id) {
                Some(t) => {
                    if json {
                        let out = serde_json::json!({
                            "ok": true,
                            "data": {
                                "ticket_id": t.ticket_id,
                                "file": t.file,
                                "line_no": t.line_no,
                                "paragraph": t.paragraph,
                            }
                        });
                        println!("{}", serde_json::to_string(&out)?);
                    } else {
                        println!("{} ({}:{})", t.ticket_id, t.file.display(), t.line_no);
                        println!("{}", t.paragraph);
                    }
                }
                None => {
                    if json {
                        let out = serde_json::json!({
                            "ok": false,
                            "error": format!("ticket not found: {ticket_id}")
                        });
                        println!("{}", serde_json::to_string(&out)?);
                    } else {
                        eprintln!("ticket not found: {ticket_id}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Command::Discipline { action } => match action {
            DisciplineAction::Show { file, json } => {
                let f = DisciplineFile::from_slug(&file).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown discipline file '{file}' — try: diary|sprint|human|plan|jira|notion|eternal"
                    )
                })?;
                let raw = omninote_core::discipline::read_raw(&vault_root, f)
                    .map_err(|e| anyhow::anyhow!("show: {e}"))?;
                if json {
                    let out = serde_json::json!({
                        "ok": true,
                        "data": { "file": f.filename(), "content": raw }
                    });
                    println!("{}", serde_json::to_string(&out)?);
                } else {
                    print!("{raw}");
                }
            }
        },
        Command::Ask {
            query,
            top_k,
            no_llm,
            model,
            json,
        } => {
            cmd_ask(&vault_root, &query, top_k, no_llm, model, json).await?;
        }
        Command::Tag { action } => match action {
            TagAction::Auto {
                file,
                apply,
                max_tags,
                max_input_chars,
                replace,
                model,
                json,
            } => {
                cmd_tag_auto(
                    &vault_root,
                    &file,
                    apply,
                    max_tags,
                    max_input_chars,
                    replace,
                    model,
                    json,
                )
                .await?;
            }
        },
    }

    Ok(())
}

/// CAD-23.2 tag --auto flow: resolve FILE → LLM suggestion → optional apply.
#[allow(clippy::too_many_arguments)]
async fn cmd_tag_auto(
    vault_root: &std::path::Path,
    file: &str,
    apply: bool,
    max_tags: usize,
    max_input_chars: usize,
    replace: bool,
    model: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let mut vault = omninote_core::vault::Vault::open(vault_root.to_path_buf())
        .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
    let rel = vault
        .index
        .resolve(file)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("note not found in vault: {file}"))?;
    let note = vault
        .notes
        .iter()
        .find(|n| n.rel_path == rel)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("resolved rel_path missing from vault.notes: {rel:?}"))?;

    let cfg = omninote_ai::LlmConfig::load().map_err(|e| anyhow::anyhow!("load llm.toml: {e}"))?;
    let key = cfg
        .anthropic_key()
        .map_err(|e| anyhow::anyhow!("API key: {e}"))?;
    let provider = omninote_ai::AnthropicProvider::new(key);

    let opts = omninote_ai::SuggestOpts {
        max_tags,
        max_input_chars,
        merge_existing: !replace,
        model: model.or(Some(cfg.provider.model.clone())),
    };
    let diff = omninote_ai::suggest_tags(&provider, &note, opts)
        .await
        .map_err(|e| anyhow::anyhow!("suggest_tags: {e}"))?;

    let applied = if apply && diff.has_changes() {
        omninote_ai::apply_diff(&mut vault, &diff)
            .map_err(|e| anyhow::anyhow!("apply_diff: {e}"))?;
        true
    } else {
        false
    };

    if json {
        let out = serde_json::json!({
            "ok": true,
            "data": {
                "rel_path": diff.note_rel_path,
                "current": {
                    "tags": diff.current_tags,
                    "summary": diff.current_summary,
                },
                "suggested": {
                    "tags": diff.suggested_tags,
                    "summary": diff.suggested_summary,
                },
                "added": { "tags": diff.added_tags },
                "applied": applied,
                "has_changes": diff.has_changes(),
            }
        });
        println!("{}", serde_json::to_string(&out)?);
    } else {
        print!("{}", diff.pretty());
        if applied {
            println!("APPLIED — frontmatter written to {}", rel.display());
        } else if diff.has_changes() {
            println!("DRY RUN — pass --apply to write changes.");
        } else {
            println!("no changes to apply.");
        }
    }

    Ok(())
}

/// Resolve `note_id` → `[[wikilink]]` target via `vault.notes`. Falls back
/// to the bare id if no matching note found (shouldn't happen but defensive).
fn note_id_to_wikilink(notes: &[omninote_core::types::Note], id: &str) -> String {
    notes
        .iter()
        .find(|n| n.frontmatter.id == id)
        .map(|n| format!("[[{}]]", n.title))
        .unwrap_or_else(|| format!("[[{id}]]"))
}

/// CAD-23.1 RAG flow: incremental index → retrieve → optional LLM answer.
async fn cmd_ask(
    vault_root: &std::path::Path,
    query: &str,
    top_k: usize,
    no_llm: bool,
    model_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let started = std::time::Instant::now();

    // 1. Vault scan.
    let vault = omninote_core::vault::Vault::open(vault_root.to_path_buf())
        .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;

    // 2. Embedder + index. First call downloads the BGE small model
    //    (~100MB → ~/.cache/huggingface) and takes ~15s to init; subsequent
    //    calls reuse the cache and start in ~1s.
    eprintln!("loading embedder (first run downloads ~100MB)...");
    let embedder = omninote_ai::FastEmbedder::bge_small()
        .map_err(|e| anyhow::anyhow!("fastembed init: {e}"))?;

    let index_path = omninote_ai::EmbeddingIndex::default_path(vault_root);
    let existing = omninote_ai::EmbeddingIndex::load(&index_path)
        .map_err(|e| anyhow::anyhow!("load embeddings: {e}"))?;
    let mut rag = omninote_ai::Rag::with_index(embedder, existing);

    // 3. Incremental refresh — skips notes whose chunks haven't changed.
    let mut embed_calls = 0usize;
    for note in &vault.notes {
        match rag.upsert_note(&note.frontmatter.id, &note.content) {
            Ok(n) => embed_calls += n,
            Err(e) => eprintln!("warn: upsert {} failed: {e}", note.title),
        }
    }
    // Drop chunks for notes that no longer exist in the vault.
    let alive_ids: std::collections::HashSet<&str> = vault
        .notes
        .iter()
        .map(|n| n.frontmatter.id.as_str())
        .collect();
    let stale_ids: Vec<String> = rag
        .index
        .entries
        .iter()
        .map(|c| c.note_id.clone())
        .filter(|id| !alive_ids.contains(id.as_str()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    for id in &stale_ids {
        rag.forget_note(id);
    }

    rag.index
        .save(&index_path)
        .map_err(|e| anyhow::anyhow!("save embeddings: {e}"))?;

    // 4. Retrieve.
    let hits = rag
        .retrieve(query, top_k)
        .map_err(|e| anyhow::anyhow!("retrieve: {e}"))?;
    let retrieved_ms = started.elapsed().as_millis();

    // Resolve note_id → wikilink for output.
    let hits_with_links: Vec<_> = hits
        .iter()
        .map(|h| (note_id_to_wikilink(&vault.notes, &h.note_id), h.clone()))
        .collect();

    // 5. Optionally call the LLM.
    let answer = if no_llm || hits.is_empty() {
        None
    } else {
        let cfg =
            omninote_ai::LlmConfig::load().map_err(|e| anyhow::anyhow!("load llm.toml: {e}"))?;
        let key = cfg
            .anthropic_key()
            .map_err(|e| anyhow::anyhow!("API key: {e}"))?;
        let provider = omninote_ai::AnthropicProvider::new(key);
        let opts = omninote_ai::CompleteOpts {
            model: model_override.unwrap_or(cfg.provider.model.clone()),
            max_tokens: cfg.provider.max_tokens,
            ..Default::default()
        };

        let mut passages = String::new();
        for (idx, (link, h)) in hits_with_links.iter().enumerate() {
            passages.push_str(&format!(
                "{}. {} (score {:.2})\n{}\n\n",
                idx + 1,
                link,
                h.score,
                h.chunk_text
            ));
        }
        let system = "You answer questions about the user's personal note vault. \
                      Cite supporting notes inline as [[wikilinks]] using the labels in the passages. \
                      If the passages don't answer the question, say so plainly.";
        let user_prompt = format!("Passages:\n\n{passages}---\n\nQuestion: {query}");
        let text = provider
            .complete(system, &user_prompt, opts)
            .await
            .map_err(|e| anyhow::anyhow!("llm: {e}"))?;
        Some(text)
    };

    let total_ms = started.elapsed().as_millis();

    if json {
        let out = serde_json::json!({
            "ok": true,
            "data": {
                "query": query,
                "passages": hits_with_links.iter().map(|(link, h)| serde_json::json!({
                    "note_id": h.note_id,
                    "chunk_idx": h.chunk_idx,
                    "wikilink": link,
                    "score": h.score,
                    "text": h.chunk_text,
                })).collect::<Vec<_>>(),
                "answer": answer,
            },
            "meta": {
                "embed_calls": embed_calls,
                "stale_dropped": stale_ids.len(),
                "retrieved_ms": retrieved_ms,
                "total_ms": total_ms,
                "indexed_chunks": rag.index.entries.len(),
            }
        });
        println!("{}", serde_json::to_string(&out)?);
    } else {
        eprintln!(
            "[indexed {} chunks · embedded {} new · dropped {} stale · {} hits in {}ms]",
            rag.index.entries.len(),
            embed_calls,
            stale_ids.len(),
            hits.len(),
            retrieved_ms
        );
        if hits.is_empty() {
            println!("no matches for: {query}");
            return Ok(());
        }
        for (idx, (link, h)) in hits_with_links.iter().enumerate() {
            println!("{}. {} (score {:.2})", idx + 1, link, h.score);
            for line in h.chunk_text.lines().take(3) {
                println!("   {line}");
            }
            println!();
        }
        if let Some(ans) = answer {
            println!("---");
            println!("{ans}");
        }
    }

    Ok(())
}
