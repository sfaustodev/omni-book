//! OmniNote CLI — scaffold (CAD-21 Phase B will wire verbs).
//!
//! Vault resolution order: `--vault <PATH>` → `OMNINOTE_VAULT` env →
//! `~/.config/omninote/last_vault` file. Mirrors the GUI's vault picker.

use clap::{Parser, Subcommand};
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
}

#[derive(Subcommand, Debug)]
enum VaultAction {
    /// Print vault metadata (path, file count, folder count).
    Info {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum NoteAction {
    /// Full-text search across notes (line-level substring match).
    Search {
        query: String,
        /// Match case exactly. Default false (case-insensitive).
        #[arg(long)]
        case: bool,
        /// Max hits. Default unlimited.
        #[arg(long)]
        limit: Option<usize>,
        /// Search note titles only (skip body).
        #[arg(long)]
        titles_only: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum LinkAction {
    /// List wikilinks that don't resolve to any note.
    Unresolved {
        #[arg(long)]
        json: bool,
    },
    /// List notes that link TO the given file.
    Backlinks {
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
                // Resolve `file` via the same rules as wikilinks (filename/path/alias).
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
    }

    Ok(())
}
