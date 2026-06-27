//! OmniNote CLI — vault ops from the terminal.
//!
//! Vault resolution order: `--vault <PATH>` → `OMNINOTE_VAULT` env →
//! active entry in `~/.config/omninote/vaults.toml` → legacy
//! `~/.config/omninote/last_vault` file. Mirrors the GUI's vault picker.
//!
//! Verbs:
//! ```text
//! omninote-cli vault info
//! omninote-cli vault list
//! omninote-cli vault add NAME PATH
//! omninote-cli vault switch NAME
//! omninote-cli capture TEXT
//! omninote-cli note search QUERY [--case] [--limit N] [--titles-only]
//! omninote-cli link unresolved
//! omninote-cli link backlinks FILE
//! omninote-cli diff [--since 1d|7d]
//! omninote-cli daily [--date YYYY-MM-DD] [--template NAME] [--folder Daily]
//! omninote-cli template list
//! omninote-cli template apply NAME [--title TITLE] [--out PATH]
//! omninote-cli diary append TEXT [--ticket CAD-XX]
//! omninote-cli human ask QUESTION
//! omninote-cli ticket ID
//! omninote-cli discipline show FILE
//! omninote-cli ask QUERY [--top-k N] [--no-llm] [--model ID]
//! omninote-cli tag auto FILE [--apply] [--max-tags N] [--replace] [--model ID]
//! ```
//! Every verb accepts `--json` for machine-readable output. The envelope is
//! `{ok: true, data, meta?}` on success or `{ok: false, error}` on failure
//! (see [`envelope`]).

mod envelope;

use clap::{Parser, Subcommand};
use envelope::Envelope;
use omninote_ai::LlmProvider; // brings trait `complete` method into scope
use omninote_core::discipline::DisciplineFile;
use serde_json::json;
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
    /// Vault inspection and the multi-vault registry.
    Vault {
        #[command(subcommand)]
        action: VaultAction,
    },
    /// Quick-capture one line to `Inbox.md` (CAD-24). Prepends a timestamped
    /// bullet, newest-first; creates the inbox on first capture.
    Capture {
        /// The line to capture — wrap in quotes for multi-word text.
        text: String,
        #[arg(long)]
        json: bool,
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
    /// Summarize recent vault changes via git (`--since 1d|7d`).
    Diff {
        /// Window: `1d`, `7d`, `2w`, a bare integer (days), or a git phrase.
        #[arg(long, default_value = "7d")]
        since: String,
        #[arg(long)]
        json: bool,
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
    /// Index + note counts for the resolved vault.
    Info {
        #[arg(long)]
        json: bool,
    },
    /// List registered vaults from `vaults.toml`.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Register (or update) a vault path under a name.
    Add {
        name: String,
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Set the active vault by name.
    Switch {
        name: String,
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

/// Resolve or bail with the standard "no vault" error. Delegates the
/// precedence ladder (`--vault`/env → registry active → legacy `last_vault`)
/// to [`omninote_core::vaults::resolve_active`] — the single source of truth so
/// other consumers (the future capture daemon) share one tested resolver.
fn require_vault(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    omninote_core::vaults::resolve_active(arg).ok_or_else(|| {
        anyhow::anyhow!(
            "no vault: pass --vault, set OMNINOTE_VAULT, run `vault add`, or open the GUI once"
        )
    })
}

fn parse_naive_date(s: &str) -> anyhow::Result<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("invalid date '{s}' (expected YYYY-MM-DD): {e}"))
}

/// Emit a JSON envelope built from a `serde_json::Value` payload.
fn emit(data: serde_json::Value) -> anyhow::Result<()> {
    Envelope::ok(data).print()?;
    Ok(())
}

/// Emit a JSON envelope with a `meta` object.
fn emit_meta(data: serde_json::Value, meta: serde_json::Value) -> anyhow::Result<()> {
    Envelope::ok_meta(data, meta).print()?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let vault_arg = cli.vault.clone();

    match cli.command {
        Command::Vault { action } => match action {
            VaultAction::Info { json } => {
                let vault_root = require_vault(vault_arg)?;
                let vault = omninote_core::vault::Vault::open(vault_root)
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let stats = vault.index.stats();
                if json {
                    emit(json!({
                        "path": vault.root,
                        "files": vault.notes.len(),
                        "index_files": stats.files,
                        "index_paths": stats.paths,
                        "index_aliases": stats.aliases,
                    }))?;
                } else {
                    println!("vault: {}", vault.root.display());
                    println!("notes: {}", vault.notes.len());
                    println!(
                        "index: {} files / {} paths / {} aliases",
                        stats.files, stats.paths, stats.aliases
                    );
                }
            }
            VaultAction::List { json } => {
                let path = omninote_core::vaults::registry_path()
                    .ok_or_else(|| anyhow::anyhow!("no config dir on this platform"))?;
                let reg = omninote_core::vaults::load(&path)
                    .map_err(|e| anyhow::anyhow!("load vaults.toml: {e}"))?;
                if json {
                    let data = reg
                        .vaults
                        .iter()
                        .map(|v| {
                            json!({
                                "name": v.name,
                                "path": v.path,
                                "active": reg.active.as_deref() == Some(v.name.as_str()),
                            })
                        })
                        .collect::<Vec<_>>();
                    emit_meta(
                        json!(data),
                        json!({ "count": reg.vaults.len(), "active": reg.active }),
                    )?;
                } else if reg.vaults.is_empty() {
                    println!("no vaults registered — run `vault add <NAME> <PATH>`");
                } else {
                    for v in &reg.vaults {
                        let marker = if reg.active.as_deref() == Some(v.name.as_str()) {
                            "*"
                        } else {
                            " "
                        };
                        println!("{marker} {}  {}", v.name, v.path.display());
                    }
                }
            }
            VaultAction::Add { name, path, json } => {
                let reg_path = omninote_core::vaults::registry_path()
                    .ok_or_else(|| anyhow::anyhow!("no config dir on this platform"))?;
                let mut reg = omninote_core::vaults::load(&reg_path)
                    .map_err(|e| anyhow::anyhow!("load vaults.toml: {e}"))?;
                // Store an absolute path: the registry is consulted from any cwd,
                // so a relative path would later resolve against the wrong dir.
                let path = std::path::absolute(&path).unwrap_or(path);
                let inserted = reg
                    .add(&name, path.clone())
                    .map_err(|e| anyhow::anyhow!("add vault: {e}"))?;
                omninote_core::vaults::save(&reg_path, &reg)
                    .map_err(|e| anyhow::anyhow!("save vaults.toml: {e}"))?;
                if json {
                    emit(json!({
                        "name": name,
                        "path": path,
                        "inserted": inserted,
                        "active": reg.active,
                    }))?;
                } else {
                    let verb = if inserted { "added" } else { "updated" };
                    println!("{verb} {name} → {}", path.display());
                }
            }
            VaultAction::Switch { name, json } => {
                let reg_path = omninote_core::vaults::registry_path()
                    .ok_or_else(|| anyhow::anyhow!("no config dir on this platform"))?;
                let mut reg = omninote_core::vaults::load(&reg_path)
                    .map_err(|e| anyhow::anyhow!("load vaults.toml: {e}"))?;
                match reg.switch(&name) {
                    Ok(()) => {
                        omninote_core::vaults::save(&reg_path, &reg)
                            .map_err(|e| anyhow::anyhow!("save vaults.toml: {e}"))?;
                        let path = reg.active_entry().map(|e| e.path.clone());
                        if json {
                            emit(json!({ "active": name, "path": path }))?;
                        } else {
                            println!("active vault: {name}");
                            if let Some(p) = path {
                                println!("{}", p.display());
                            }
                        }
                    }
                    Err(e) => {
                        if json {
                            Envelope::<serde_json::Value>::error(e).print()?;
                        } else {
                            eprintln!("{e}");
                        }
                        std::process::exit(1);
                    }
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
                let vault_root = require_vault(vault_arg)?;
                let vault = omninote_core::vault::Vault::open(vault_root)
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
                    let data = hits
                        .iter()
                        .map(|h| {
                            json!({
                                "rel_path": h.rel_path,
                                "title": h.title,
                                "line_no": h.line_no,
                                "snippet": h.snippet,
                            })
                        })
                        .collect::<Vec<_>>();
                    emit_meta(json!(data), json!({ "count": hits.len(), "query": query }))?;
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
                let vault_root = require_vault(vault_arg)?;
                let vault = omninote_core::vault::Vault::open(vault_root)
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let unresolved = vault.index.unresolved_links(&vault.notes);
                if json {
                    let data = unresolved
                        .iter()
                        .map(|u| json!({ "target": u.target, "source": u.source }))
                        .collect::<Vec<_>>();
                    emit_meta(json!(data), json!({ "count": unresolved.len() }))?;
                } else {
                    println!("{} unresolved", unresolved.len());
                    for u in &unresolved {
                        println!("  {} ← {}", u.target, u.source.display());
                    }
                }
            }
            LinkAction::Backlinks { file, json } => {
                let vault_root = require_vault(vault_arg)?;
                let vault = omninote_core::vault::Vault::open(vault_root)
                    .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
                let target = vault.index.resolve(&file).cloned().ok_or_else(|| {
                    anyhow::anyhow!("file does not match any note in vault: {file}")
                })?;
                let backlinks = vault.index.backlinks_to(&target, &vault.notes);
                if json {
                    let data = backlinks
                        .iter()
                        .map(|b| {
                            json!({
                                "source": b.source,
                                "is_embed": b.is_embed,
                                "anchor": format_anchor(&b.anchor),
                            })
                        })
                        .collect::<Vec<_>>();
                    emit_meta(
                        json!(data),
                        json!({ "count": backlinks.len(), "target": target }),
                    )?;
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
        Command::Diff { since, json } => {
            let vault_root = require_vault(vault_arg)?;
            let report = omninote_core::snapshot::diff_since(&vault_root, &since)
                .map_err(|e| anyhow::anyhow!("diff: {e}"))?;
            if json {
                emit(serde_json::to_value(&report)?)?;
            } else if !report.is_git {
                println!(
                    "{} is not a git repo — `omninote diff` needs git history",
                    vault_root.display()
                );
            } else if report.changed.is_empty() {
                println!(
                    "no changes since {} ({} commits)",
                    report.since, report.commits
                );
            } else {
                println!(
                    "{} changed file(s) since {} ({} commits):",
                    report.changed.len(),
                    report.since,
                    report.commits
                );
                for c in &report.changed {
                    match &c.old_path {
                        Some(old) => println!("  {} {} → {}", c.status, old, c.path),
                        None => println!("  {} {}", c.status, c.path),
                    }
                }
            }
        }

        Command::Capture { text, json } => {
            // Vault resolution is itself a failure mode here: under `--json` it
            // must surface as an error envelope (not a propagated anyhow exit),
            // so resolve explicitly rather than via the `?` helper.
            let vault_root = match omninote_core::vaults::resolve_active(vault_arg) {
                Some(p) => p,
                None => {
                    let msg = "no vault: pass --vault, set OMNINOTE_VAULT, run `vault add`, or open the GUI once";
                    if json {
                        Envelope::<serde_json::Value>::error(msg).print()?;
                    } else {
                        eprintln!("{msg}");
                    }
                    std::process::exit(1);
                }
            };
            let vault = omninote_core::vault::Vault::open(vault_root)
                .map_err(|e| anyhow::anyhow!("vault open failed: {e}"))?;
            match vault.capture_line(&text) {
                Ok(out) => {
                    if json {
                        emit_meta(
                            json!({ "path": out.path, "line_appended": out.bullet }),
                            json!({ "total_lines": out.total_lines }),
                        )?;
                    } else {
                        println!("✓ Inbox.md  (+1 line)");
                        println!("{}", out.bullet);
                    }
                }
                Err(e) => {
                    if json {
                        Envelope::<serde_json::Value>::error(e).print()?;
                    } else {
                        eprintln!("{e}");
                    }
                    std::process::exit(1);
                }
            }
        }

        // ────────── CAD-22 verbs ──────────
        Command::Daily {
            date,
            template,
            folder,
            json,
        } => {
            let vault_root = require_vault(vault_arg)?;
            let opts = omninote_core::daily::DailyOpts {
                date: date.as_deref().map(parse_naive_date).transpose()?,
                template_name: template,
                folder,
            };
            let res = omninote_core::daily::ensure_daily(&vault_root, opts)
                .map_err(|e| anyhow::anyhow!("daily: {e}"))?;
            if json {
                emit(json!({
                    "path": res.path,
                    "rel_path": res.rel_path,
                    "created": res.created,
                    "template_used": res.template_used,
                }))?;
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
                let vault_root = require_vault(vault_arg)?;
                let list = omninote_core::templates::list_templates(&vault_root);
                if json {
                    let data = list
                        .iter()
                        .map(|t| json!({ "name": t.name, "path": t.path }))
                        .collect::<Vec<_>>();
                    emit_meta(json!(data), json!({ "count": list.len() }))?;
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
                let vault_root = require_vault(vault_arg)?;
                let body = omninote_core::templates::load_template(&vault_root, &name)
                    .map_err(|e| anyhow::anyhow!("template: {e}"))?;
                let ctx = omninote_core::templates::TemplateContext::now(title);
                let rendered = omninote_core::templates::render(&body, &ctx);
                if let Some(path) = out.as_ref() {
                    std::fs::write(path, &rendered)
                        .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
                }
                if json {
                    emit(json!({ "rendered": rendered, "wrote_to": out }))?;
                } else if out.is_none() {
                    print!("{rendered}");
                } else {
                    println!("wrote: {}", out.as_ref().unwrap().display());
                }
            }
        },
        Command::Diary { action } => match action {
            DiaryAction::Append { text, ticket, json } => {
                let vault_root = require_vault(vault_arg)?;
                let path =
                    omninote_core::discipline::diary_quick(&vault_root, &text, ticket.as_deref())
                        .map_err(|e| anyhow::anyhow!("diary append: {e}"))?;
                if json {
                    emit(json!({ "path": path }))?;
                } else {
                    println!("appended to {}", path.display());
                }
            }
        },
        Command::Human { action } => match action {
            HumanAction::Ask { question, json } => {
                let vault_root = require_vault(vault_arg)?;
                let (path, qn) = omninote_core::discipline::human_ask(&vault_root, &question)
                    .map_err(|e| anyhow::anyhow!("human ask: {e}"))?;
                if json {
                    emit(json!({ "path": path, "q_id": qn }))?;
                } else {
                    println!("{} added to {}", qn, path.display());
                }
            }
        },
        Command::Ticket { ticket_id, json } => {
            let vault_root = require_vault(vault_arg)?;
            match omninote_core::discipline::ticket_status(&vault_root, &ticket_id) {
                Some(t) => {
                    if json {
                        emit(json!({
                            "ticket_id": t.ticket_id,
                            "file": t.file,
                            "line_no": t.line_no,
                            "paragraph": t.paragraph,
                        }))?;
                    } else {
                        println!("{} ({}:{})", t.ticket_id, t.file.display(), t.line_no);
                        println!("{}", t.paragraph);
                    }
                }
                None => {
                    if json {
                        Envelope::<serde_json::Value>::error(format!(
                            "ticket not found: {ticket_id}"
                        ))
                        .print()?;
                    } else {
                        eprintln!("ticket not found: {ticket_id}");
                    }
                    std::process::exit(1);
                }
            }
        }
        Command::Discipline { action } => match action {
            DisciplineAction::Show { file, json } => {
                let vault_root = require_vault(vault_arg)?;
                let f = DisciplineFile::from_slug(&file).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown discipline file '{file}' — try: diary|sprint|human|plan|jira|notion|eternal"
                    )
                })?;
                let raw = omninote_core::discipline::read_raw(&vault_root, f)
                    .map_err(|e| anyhow::anyhow!("show: {e}"))?;
                if json {
                    emit(json!({ "file": f.filename(), "content": raw }))?;
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
            let vault_root = require_vault(vault_arg)?;
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
                let vault_root = require_vault(vault_arg)?;
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
        emit(json!({
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
        }))?;
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
        let data = json!({
            "query": query,
            "passages": hits_with_links.iter().map(|(link, h)| json!({
                "note_id": h.note_id,
                "chunk_idx": h.chunk_idx,
                "wikilink": link,
                "score": h.score,
                "text": h.chunk_text,
            })).collect::<Vec<_>>(),
            "answer": answer,
        });
        let meta = json!({
            "embed_calls": embed_calls,
            "stale_dropped": stale_ids.len(),
            "retrieved_ms": retrieved_ms,
            "total_ms": total_ms,
            "indexed_chunks": rag.index.entries.len(),
        });
        emit_meta(data, meta)?;
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
