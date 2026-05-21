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
    /// Full-text search across notes.
    Search {
        query: String,
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
            NoteAction::Search { query, json: _ } => {
                anyhow::bail!(
                    "note search not implemented yet (CAD-21 Phase B). Query was: {query}"
                )
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
            LinkAction::Backlinks { file, json: _ } => {
                anyhow::bail!(
                    "link backlinks not implemented yet (CAD-21 Phase B). File was: {file}"
                )
            }
        },
    }

    Ok(())
}
