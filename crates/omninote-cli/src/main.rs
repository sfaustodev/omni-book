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

fn main() -> anyhow::Result<()> {
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
    }

    Ok(())
}
