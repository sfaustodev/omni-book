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
//! ```
//! Every verb accepts `--json` for machine-readable output. The envelope is
//! `{ok: true, data, meta?}` on success or `{ok: false, error}` on failure
//! (see [`envelope`]).

mod envelope;

use clap::{Parser, Subcommand};
use envelope::Envelope;
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
    /// Open/create today's daily note.
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
    /// Template operations.
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Append entry to discipline DIARY.md.
    Diary {
        #[command(subcommand)]
        action: DiaryAction,
    },
    /// Open question in discipline HUMAN.md.
    Human {
        #[command(subcommand)]
        action: HumanAction,
    },
    /// Look up ticket status in NOTION.md / JIRA.md.
    Ticket {
        ticket_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Show raw content of a discipline file.
    Discipline {
        #[command(subcommand)]
        action: DisciplineAction,
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

fn format_anchor(a: &Option<omninote_core::wikilinks::Anchor>) -> Option<String> {
    use omninote_core::wikilinks::Anchor;
    a.as_ref().map(|x| match x {
        Anchor::Heading(h) => format!("#{h}"),
        Anchor::Block(b) => format!("#^{b}"),
    })
}

/// Resolve the vault root: `--vault`/env first (handled by clap), then the
/// active entry in `vaults.toml`, then the legacy `last_vault` file.
fn resolve_vault(arg: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = arg {
        return Some(p);
    }
    if let Some(p) = omninote_core::vaults::registry_path()
        .and_then(|rp| omninote_core::vaults::load(&rp).ok())
        .and_then(|reg| reg.active_entry().map(|e| e.path.clone()))
        .filter(|p| p.exists())
    {
        return Some(p);
    }
    let last = dirs::config_dir()?.join("omninote").join("last_vault");
    std::fs::read_to_string(last)
        .ok()
        .map(|s| PathBuf::from(s.trim()))
        .filter(|p| p.exists())
}

/// Resolve or bail with the standard "no vault" error.
fn require_vault(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    resolve_vault(arg).ok_or_else(|| {
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

fn main() -> anyhow::Result<()> {
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
                            std::process::exit(1);
                        }
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
                        std::process::exit(1);
                    }
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
    }

    Ok(())
}
