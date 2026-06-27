use crate::resolver::VaultIndex;
use crate::types::{AppConfig, Frontmatter, Note, NoteType};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Result of a quick-capture write to `Inbox.md`. The shared seam between the
/// `omninote capture` CLI verb and the future global-hotkey daemon, so both
/// report the same `{path, bullet, total_lines}` shape from one tested path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureOutcome {
    /// Absolute path to the `Inbox.md` that received the line.
    pub path: PathBuf,
    /// The rendered bullet that was prepended, e.g. `- 2026-06-27 14:03 · buy milk`.
    pub bullet: String,
    /// Count of bullet lines (`- …`) in `Inbox.md` after the write.
    pub total_lines: usize,
}

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub config: AppConfig,
    /// Wikilink resolution index — rebuilt after every `reload_notes()`.
    /// CAD-20 (Phase 1 link parity).
    pub index: VaultIndex,
    /// Sorted folder list (rel paths), refreshed on reload and folder/note
    /// mutations. Cached so the sidebar never walks the disk on a render frame —
    /// a live walk per frame pegged a CPU core and kept egui from settling.
    folders: Vec<PathBuf>,
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        // Q-08: reject file paths early. Previous behavior returned `Ok` with an
        // empty vault because `fs::create_dir_all` uses `let _ =` below and
        // swallowed the error — confusing failure mode.
        if root.is_file() {
            return Err(format!(
                "vault path is a file, expected directory: {}",
                root.display()
            ));
        }
        if !root.exists() {
            fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        }
        let cfg_dir = root.join(".omninote");
        let _ = fs::create_dir_all(&cfg_dir);
        let _ = fs::create_dir_all(root.join("_attachments"));

        let config: AppConfig = fs::read_to_string(cfg_dir.join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut v = Self {
            root,
            notes: Vec::new(),
            config,
            index: VaultIndex::default(),
            folders: Vec::new(),
        };
        v.reload_notes();
        Ok(v)
    }

    pub fn reload_notes(&mut self) {
        self.notes.clear();
        // `filter_entry` PRUNES heavy directories before descending into them.
        // Critical for vaults that contain (or are) a code project: without this,
        // WalkDir walks every file under target/ node_modules/ .git/ (hundreds of
        // thousands), freezing the app on a real-world folder. Plain post-filtering
        // would still traverse those trees.
        let root = self.root.clone();
        let walker = WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| e.path() == root || !is_pruned_dir(e.path()));
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() || !is_loadable(path) {
                continue;
            }
            if let Ok(note) = self.read_note(path) {
                self.notes.push(note);
            }
        }
        self.rescan_folders();
        // Rebuild resolver index after every reload (CAD-20).
        self.index = VaultIndex::build(&self.notes);
    }

    /// Recompute the cached folder list (sorted rel paths, pruned dirs excluded).
    /// Called on reload and after folder/note mutations — never on a render frame.
    fn rescan_folders(&mut self) {
        let root = self.root.clone();
        let mut folders: Vec<PathBuf> = WalkDir::new(&self.root)
            .into_iter()
            .filter_entry(|e| e.path() == root || !is_pruned_dir(e.path()))
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir() && e.path() != root)
            .filter_map(|e| e.path().strip_prefix(&root).ok().map(Path::to_path_buf))
            .collect();
        folders.sort();
        self.folders = folders;
    }

    fn read_note(&self, path: &Path) -> Result<Note, String> {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let is_md = path.extension().and_then(|s| s.to_str()) == Some("md");
        // Only markdown carries OmniNote frontmatter. For .txt/.env, keep the
        // bytes verbatim so a later save can't inject a YAML header and corrupt
        // the file.
        let (mut frontmatter, content) = if is_md {
            parse_frontmatter(&raw)
        } else {
            (Frontmatter::default(), raw)
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sem título")
            .to_string();
        let rel_path = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
        // Foreign vaults (Obsidian, raw markdown, etc.) often have no `id:`
        // frontmatter — defaulting to "" would make every such note collide
        // on the same key in downstream HashMaps (CAD-23.1 RAG bug). Use the
        // rel_path as a stable derived id: unique per file, survives rename
        // because we re-read on every reload.
        if frontmatter.id.is_empty() {
            frontmatter.id = rel_path.to_string_lossy().to_string();
        }
        Ok(Note {
            path: path.to_path_buf(),
            rel_path,
            frontmatter,
            title,
            content,
        })
    }

    pub fn save_note(&self, note: &Note) -> Result<(), String> {
        let is_md = note.path.extension().and_then(|s| s.to_str()) == Some("md");
        let out = if is_md {
            let mut s = String::from("---\n");
            s.push_str(&serde_yaml::to_string(&note.frontmatter).map_err(|e| e.to_string())?);
            s.push_str("---\n\n");
            s.push_str(&note.content);
            s
        } else {
            // .txt/.env etc. are stored verbatim — no frontmatter header.
            note.content.clone()
        };
        if let Some(parent) = note.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&note.path, out).map_err(|e| e.to_string())
    }

    /// Set a note's type by id, persist it to frontmatter, and reload. Only
    /// markdown notes carry frontmatter, so this is rejected for .txt/.env.
    pub fn set_note_type(&mut self, id: &str, note_type: NoteType) -> Result<(), String> {
        let idx = self
            .notes
            .iter()
            .position(|n| n.frontmatter.id == id)
            .ok_or("nota não encontrada")?;
        let is_md = self.notes[idx].path.extension().and_then(|s| s.to_str()) == Some("md");
        if !is_md {
            return Err("categoria disponível só em notas .md".into());
        }
        self.notes[idx].frontmatter.note_type = note_type;
        let note = self.notes[idx].clone();
        self.save_note(&note)?;
        self.reload_notes();
        Ok(())
    }

    /// Set the type on every markdown note inside `folder` (recursively — the
    /// whole subtree). Non-md files are skipped. Returns how many notes changed.
    /// `folder` is a vault-relative path (as produced by `list_folders`).
    pub fn set_folder_note_type(
        &mut self,
        folder: &Path,
        note_type: NoteType,
    ) -> Result<usize, String> {
        let targets: Vec<usize> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                n.path.extension().and_then(|s| s.to_str()) == Some("md")
                    && n.rel_path.starts_with(folder)
            })
            .map(|(i, _)| i)
            .collect();
        // Pre-flight: a filesystem vault has no multi-file transaction, so a
        // mid-loop save failure would leave the folder half-categorized. Verify
        // every target exists and is writable BEFORE touching any of them, so the
        // common failure (read-only / missing file) aborts with zero writes.
        // Fail-safe, not atomic: a disk-full or a race after this check can still
        // partial-write; `reload_notes()` below re-syncs memory from disk regardless.
        for &i in &targets {
            let p = &self.notes[i].path;
            let meta = fs::metadata(p)
                .map_err(|e| format!("não posso categorizar a pasta: {} ({e})", p.display()))?;
            if meta.permissions().readonly() {
                return Err(format!(
                    "não posso categorizar a pasta: arquivo somente-leitura {}",
                    p.display()
                ));
            }
        }
        for &i in &targets {
            self.notes[i].frontmatter.note_type = note_type;
            let note = self.notes[i].clone();
            self.save_note(&note)?;
        }
        let changed = targets.len();
        if changed > 0 {
            self.reload_notes();
        }
        Ok(changed)
    }

    pub fn create_note(
        &mut self,
        folder: Option<&Path>,
        title: &str,
        note_type: NoteType,
    ) -> Result<Note, String> {
        let folder_path = folder
            .map(|f| self.root.join(f))
            .unwrap_or_else(|| self.root.clone());
        fs::create_dir_all(&folder_path).map_err(|e| e.to_string())?;
        let safe_title = if title.is_empty() {
            "Nova nota".to_string()
        } else {
            sanitize_filename(title)
        };
        let mut path = folder_path.join(format!("{}.md", safe_title));
        let mut counter = 1;
        while path.exists() {
            path = folder_path.join(format!("{} ({}).md", safe_title, counter));
            counter += 1;
        }
        let id = format!("n_{}", uuid::Uuid::new_v4().simple());
        let frontmatter = Frontmatter {
            id,
            note_type,
            tags: vec![],
            source: String::new(),
            source_link: String::new(),
            linked_note: None,
            attachments: vec![],
            created: chrono::Utc::now().to_rfc3339(),
            aliases: vec![],
            summary: String::new(),
            extra: Default::default(),
        };
        let title_str = path.file_stem().unwrap().to_string_lossy().to_string();
        let rel_path = path.strip_prefix(&self.root).unwrap().to_path_buf();
        let note = Note {
            path: path.clone(),
            rel_path,
            frontmatter,
            title: title_str,
            content: String::new(),
        };
        self.save_note(&note)?;
        self.notes.push(note.clone());
        Ok(note)
    }

    pub fn rename_note(&mut self, idx: usize, new_title: &str) -> Result<(), String> {
        let safe = sanitize_filename(new_title);
        let parent = self.notes[idx].path.parent().unwrap().to_path_buf();
        let new_path = parent.join(format!("{}.md", safe));
        if new_path == self.notes[idx].path {
            return Ok(());
        }
        fs::rename(&self.notes[idx].path, &new_path).map_err(|e| e.to_string())?;
        self.notes[idx].path = new_path.clone();
        self.notes[idx].rel_path = new_path.strip_prefix(&self.root).unwrap().to_path_buf();
        self.notes[idx].title = safe;
        Ok(())
    }

    pub fn rename_note_by_id(&mut self, id: &str, new_title: &str) -> Result<Note, String> {
        let idx = self
            .notes
            .iter()
            .position(|n| n.frontmatter.id == id)
            .ok_or_else(|| "note not found".to_string())?;
        self.rename_note(idx, new_title)?;
        Ok(self.notes[idx].clone())
    }

    pub fn delete_note(&mut self, idx: usize) -> Result<(), String> {
        fs::remove_file(&self.notes[idx].path).map_err(|e| e.to_string())?;
        self.notes.remove(idx);
        Ok(())
    }

    /// Move a note into a different folder (or root if `new_folder` is None).
    /// Filename stays the same; only the parent dir changes. Updates path/rel_path
    /// on the in-memory note. Errors if a file with the same name already exists at target.
    pub fn move_note_by_id(&mut self, id: &str, new_folder: Option<&Path>) -> Result<(), String> {
        let idx = self
            .notes
            .iter()
            .position(|n| n.frontmatter.id == id)
            .ok_or_else(|| "note not found".to_string())?;
        let old_path = self.notes[idx].path.clone();
        let filename = old_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "invalid filename".to_string())?
            .to_string();
        let new_dir = new_folder
            .map(|f| self.root.join(f))
            .unwrap_or_else(|| self.root.clone());
        fs::create_dir_all(&new_dir).map_err(|e| e.to_string())?;
        let new_path = new_dir.join(&filename);
        if old_path == new_path {
            return Ok(());
        }
        if new_path.exists() {
            return Err(format!("Já existe um arquivo \"{}\" no destino", filename));
        }
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        self.notes[idx].path = new_path.clone();
        self.notes[idx].rel_path = new_path.strip_prefix(&self.root).unwrap().to_path_buf();
        self.rescan_folders();
        Ok(())
    }

    pub fn create_folder(&mut self, parent: Option<&Path>, name: &str) -> Result<PathBuf, String> {
        let safe = sanitize_filename(name);
        let parent_abs = parent
            .map(|p| self.root.join(p))
            .unwrap_or_else(|| self.root.clone());
        let path = parent_abs.join(safe);
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        self.rescan_folders();
        Ok(path)
    }

    #[allow(dead_code)]
    pub fn rename_folder(&mut self, rel: &Path, new_name: &str) -> Result<(), String> {
        let abs = self.root.join(rel);
        let parent = abs.parent().unwrap();
        let safe = sanitize_filename(new_name);
        let new_abs = parent.join(safe);
        fs::rename(&abs, &new_abs).map_err(|e| e.to_string())?;
        self.reload_notes();
        Ok(())
    }

    pub fn delete_folder(&mut self, rel: &Path) -> Result<(), String> {
        fs::remove_dir_all(self.root.join(rel)).map_err(|e| e.to_string())?;
        self.reload_notes();
        Ok(())
    }

    pub fn list_folders(&self) -> Vec<PathBuf> {
        self.folders.clone()
    }

    /// Quick-capture: prepend a timestamped bullet to `Inbox.md` (Q-06 — plain
    /// bullets at the top of the file, newest first, append-friendly). Creates
    /// the file with an `# Inbox` heading on first capture.
    ///
    /// Newest-first means inserting at the TOP after the heading, so a plain
    /// O_APPEND write can't be used: it's read-modify-write. The write is made
    /// crash-safe by staging the new content in a sibling temp file, fsyncing it,
    /// then atomically renaming over `Inbox.md` — so a crash mid-write can never
    /// leave a truncated or empty inbox; a reader sees either the old or the new
    /// file, never a torn one.
    ///
    /// A cross-process advisory lock for concurrent WRITERS (two simultaneous
    /// captures racing the read-modify-write) is intentionally deferred to the
    /// future hotkey daemon, where concurrent capture becomes real. For the
    /// single-user CLI today, atomic-rename is the right level.
    pub fn append_inbox_line(&self, text: &str) -> Result<(), String> {
        let line = text.trim();
        if line.is_empty() {
            return Err("linha vazia".into());
        }
        let inbox = self.root.join("Inbox.md");
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
        let bullet = format!("- {stamp} · {line}\n");
        let existing = std::fs::read_to_string(&inbox).unwrap_or_default();
        let new = if existing.is_empty() {
            format!("# Inbox\n\n{bullet}")
        } else if let Some(rest) = existing.strip_prefix("# Inbox\n") {
            // Insert the bullet right after the heading (newest first).
            format!("# Inbox\n{}", insert_after_blank(rest, &bullet))
        } else {
            format!("{bullet}{existing}")
        };
        write_atomic(&inbox, new.as_bytes())
    }

    /// Quick-capture with a reportable outcome — the shared seam for the
    /// `omninote capture` CLI verb and the future hotkey daemon. Wraps
    /// [`append_inbox_line`](Self::append_inbox_line) (same trim/empty rules,
    /// same newest-first bullet), then reads `Inbox.md` back to report the exact
    /// bullet written and the running bullet count.
    ///
    /// Reading the file back (rather than re-rendering the bullet) avoids a
    /// second `Local::now()` that could disagree with the persisted timestamp
    /// across a minute boundary.
    pub fn capture_line(&self, text: &str) -> Result<CaptureOutcome, String> {
        self.append_inbox_line(text)?;
        let inbox = self.root.join("Inbox.md");
        let content = std::fs::read_to_string(&inbox).map_err(|e| e.to_string())?;
        let bullet = content
            .lines()
            .find(|l| l.starts_with("- "))
            .map(str::to_string)
            .ok_or_else(|| "captured bullet not found in Inbox.md".to_string())?;
        let total_lines = content.lines().filter(|l| l.starts_with("- ")).count();
        Ok(CaptureOutcome {
            path: inbox,
            bullet,
            total_lines,
        })
    }

    /// Resolve an embed filename (`![[foo.png]]`) to an absolute path **inside**
    /// this vault's `_attachments/`, or `None` if it doesn't exist or would
    /// escape the directory.
    ///
    /// Security (CWE-22): `filename` comes from note content, which in a shared/
    /// downloaded vault may be authored by a third party. `Path::join` does NOT
    /// contain traversal (`join("../x")` escapes), and `starts_with` compares
    /// raw components (so `_attachments/../etc` "starts with" `_attachments`).
    /// So we reject any separator/`..`/root component up front, then canonicalize
    /// and re-verify the result stays under the canonical `_attachments` dir.
    pub fn attachment_path(&self, filename: &str) -> Option<PathBuf> {
        let name = filename.trim();
        if name.is_empty() {
            return None;
        }
        // A legitimate attachment name is a single path component — no dir parts.
        let mut comps = Path::new(name).components();
        match (comps.next(), comps.next()) {
            (Some(std::path::Component::Normal(_)), None) => {}
            _ => return None, // multi-component, `..`, absolute, etc.
        }
        let attach_dir = self.root.join("_attachments");
        let candidate = attach_dir.join(name);
        // Canonicalize both and confirm containment (defends against symlinks).
        let base = attach_dir.canonicalize().ok()?;
        let real = candidate.canonicalize().ok()?;
        real.starts_with(&base).then_some(real)
    }

    pub fn import_attachment(&self, src: &Path) -> Result<String, String> {
        let attach_dir = self.root.join("_attachments");
        fs::create_dir_all(&attach_dir).map_err(|e| e.to_string())?;
        let original_name = src.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        let safe = sanitize_filename(original_name);
        let mut dest = attach_dir.join(&safe);
        let mut counter = 1;
        while dest.exists() {
            let stem = Path::new(&safe)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = Path::new(&safe)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            dest = attach_dir.join(format!("{}_{}.{}", stem, counter, ext));
            counter += 1;
        }
        fs::copy(src, &dest).map_err(|e| e.to_string())?;
        Ok(dest.file_name().unwrap().to_string_lossy().to_string())
    }

    pub fn save_config(&self) -> Result<(), String> {
        let p = self.root.join(".omninote").join("config.json");
        let s = serde_json::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        fs::write(p, s).map_err(|e| e.to_string())
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Files OmniNote treats as notes: markdown, plus plain-text (`.txt`) and env
/// files (`.env`, `.env.*`) the user wants to read. Everything else (binaries,
/// source, configs) is ignored so a vault can point at a code repo and surface
/// only the textual files.
fn is_loadable(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if matches!(ext, "md" | "txt" | "env") {
            return true;
        }
    }
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    name == ".env" || name.starts_with(".env.")
}

/// Directories WalkDir must not descend into: OmniNote internals, attachments,
/// and the heavy build/VCS/dependency trees that explode a scan when a vault
/// happens to contain a code project. Matched by directory name only (so a note
/// literally named `target.md` is unaffected).
fn is_pruned_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    // Hidden dotfolders (.git, .obsidian, .omninote, .vscode, .venv, …) never
    // belong in the tree; plus heavy build/dependency dirs that would otherwise
    // make WalkDir traverse hundreds of thousands of files and freeze the app.
    name.starts_with('.')
        || matches!(
            name,
            "_attachments"
                | "target"
                | "node_modules"
                | "venv"
                | "__pycache__"
                | "dist"
                | "build"
                | "vendor"
        )
}

/// Insert `bullet` after the leading blank line of `rest` (the body after the
/// `# Inbox` heading), so captures land newest-first under the heading.
fn insert_after_blank(rest: &str, bullet: &str) -> String {
    if let Some(after) = rest.strip_prefix('\n') {
        format!("\n{bullet}{after}")
    } else {
        format!("\n{bullet}{rest}")
    }
}

/// Write `bytes` to `dest` crash-safely: stage in a sibling temp file, fsync the
/// data and the file, then atomically `rename` over `dest`. A crash at any point
/// leaves either the prior file or the complete new one — never a torn/empty
/// result. The temp file shares `dest`'s directory so the rename stays on one
/// filesystem (cross-device rename would fail). A unique temp name avoids
/// clobbering a concurrent writer's staging file.
fn write_atomic(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let dir = dest.parent().unwrap_or_else(|| Path::new("."));
    let stem = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Inbox.md");
    let tmp = dir.join(format!(".{stem}.{}.tmp", std::process::id()));
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("create temp: {e}"))?;
        f.write_all(bytes).map_err(|e| format!("write temp: {e}"))?;
        f.sync_all().map_err(|e| format!("sync temp: {e}"))?;
    }
    fs::rename(&tmp, dest).map_err(|e| {
        // Best-effort cleanup so a failed rename doesn't litter the vault.
        let _ = fs::remove_file(&tmp);
        format!("rename temp: {e}")
    })
}

pub fn sanitize_filename_pub(name: &str) -> String {
    sanitize_filename(name)
}

pub fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    if !raw.starts_with("---") {
        return (Frontmatter::default(), raw.to_string());
    }
    let after = &raw[3..];
    if let Some(end) = after.find("\n---") {
        let yaml = after[..end].trim_start_matches('\n');
        let rest = after[end + 4..].trim_start_matches('\n');
        return (frontmatter_from_yaml(yaml), rest.to_string());
    }
    (Frontmatter::default(), raw.to_string())
}

/// Parse a frontmatter YAML block DEFENSIVELY. `from_str::<Frontmatter>(yaml)`
/// fails — and the old `unwrap_or_default` then DROPPED EVERYTHING — the moment
/// any modeled field has an unexpected shape: a foreign Obsidian note with
/// `type: book`, no `id:`, or a scalar `tags: rust` (all valid YAML that Obsidian
/// writes). Routing through a permissive `Mapping` and extracting each modeled
/// field tolerantly means a hostile/foreign frontmatter never wipes the note's
/// other keys — they land in `extra` and round-trip. An unknown `type:` value
/// normalizes to the default (OmniNote's type enum is fixed at 6 variants and the
/// original string can't coexist with the modeled `type` key on save), but every
/// non-modeled key survives. CAD-25b Slice 4 (data-loss fix — real failure paths).
fn frontmatter_from_yaml(yaml: &str) -> Frontmatter {
    use serde_yaml::Value;

    // A bare scalar / list (not a key:value mapping) has nothing structured to
    // preserve — fall back to default, same as an empty frontmatter.
    let mut map: serde_yaml::Mapping = match serde_yaml::from_str(yaml) {
        Ok(m) => m,
        // serde_yaml rejects DUPLICATE keys outright. Rather than wipe the whole
        // note (the round-1 total-loss class), retry once on a de-duplicated copy
        // (keep-last, YAML-lenient). This only runs after a strict parse already
        // failed, so a well-formed note never reaches it — downside-protected.
        Err(_) => match serde_yaml::from_str(&dedup_top_level_keys(yaml)) {
            Ok(m) => m,
            Err(_) => return Frontmatter::default(),
        },
    };

    fn take(m: &mut serde_yaml::Mapping, k: &str) -> Option<Value> {
        m.remove(Value::String(k.to_string()))
    }
    fn as_string(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }
    // Obsidian commonly writes a single tag/alias as a bare scalar (`tags: rust`),
    // which is valid YAML but not a sequence — accept both shapes.
    fn as_string_vec(v: Value) -> Vec<String> {
        match v {
            Value::Sequence(seq) => seq.iter().filter_map(as_string).collect(),
            other => as_string(&other).into_iter().collect(),
        }
    }

    let id = take(&mut map, "id")
        .as_ref()
        .and_then(as_string)
        .unwrap_or_default();
    // Unknown enum value (`type: book`) → None → default Resumo. The key is still
    // consumed so it does not also re-emit via `extra` and collide with `type`.
    let note_type = take(&mut map, "type")
        .and_then(|v| serde_yaml::from_value::<NoteType>(v).ok())
        .unwrap_or_default();
    let tags = take(&mut map, "tags")
        .map(as_string_vec)
        .unwrap_or_default();
    let source = take(&mut map, "source")
        .as_ref()
        .and_then(as_string)
        .unwrap_or_default();
    let source_link = take(&mut map, "source_link")
        .as_ref()
        .and_then(as_string)
        .unwrap_or_default();
    let linked_note = take(&mut map, "linked_note").as_ref().and_then(as_string);
    let attachments = take(&mut map, "attachments")
        .map(as_string_vec)
        .unwrap_or_default();
    let created = take(&mut map, "created")
        .as_ref()
        .and_then(as_string)
        .unwrap_or_default();
    let aliases = take(&mut map, "aliases")
        .map(as_string_vec)
        .unwrap_or_default();
    let summary = take(&mut map, "summary")
        .as_ref()
        .and_then(as_string)
        .unwrap_or_default();

    // Everything left is an unmodeled key — preserve it verbatim. A non-string key
    // (`2024:`, `true:`) is coerced to its string form rather than dropped, matching
    // the pre-fix serde-flatten behavior (`extra` is keyed by String, so it must be
    // a String either way).
    let extra = map
        .into_iter()
        .map(|(k, v)| match k {
            Value::String(s) => (s, v),
            other => (
                serde_yaml::to_string(&other)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                v,
            ),
        })
        .collect();

    Frontmatter {
        id,
        note_type,
        tags,
        source,
        source_link,
        linked_note,
        attachments,
        created,
        aliases,
        summary,
        extra,
    }
}

/// Drop earlier duplicates of a top-level YAML key (keep-last, YAML-lenient), so a
/// hand-edited note with two `tags:`/`cssclass:` lines parses instead of being
/// rejected by serde_yaml (and then wiped). Only a column-0 `key:` line counts as
/// top-level; indented continuation lines (block scalars, list items) ride with
/// their parent block. Best-effort text pass used SOLELY on the strict-parse error
/// path — a well-formed note never reaches it, so it can only recover, never harm.
fn dedup_top_level_keys(yaml: &str) -> String {
    // Group lines into blocks: a column-0 `key:` line plus its indented body.
    let mut blocks: Vec<(Option<String>, Vec<&str>)> = Vec::new();
    for line in yaml.lines() {
        let starts_top_key = line
            .chars()
            .next()
            .is_some_and(|c| c != ' ' && c != '\t' && c != '-' && c != '#')
            && line.contains(':');
        if starts_top_key {
            let key = line.split(':').next().unwrap_or("").trim().to_string();
            blocks.push((Some(key), vec![line]));
        } else if let Some(last) = blocks.last_mut() {
            last.1.push(line);
        } else {
            blocks.push((None, vec![line]));
        }
    }
    // Index of the LAST block for each key; earlier blocks of a recurring key drop.
    let mut last_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (i, (k, _)) in blocks.iter().enumerate() {
        if let Some(k) = k {
            last_idx.insert(k.clone(), i);
        }
    }
    let mut out: Vec<&str> = Vec::new();
    for (i, (k, lines)) in blocks.iter().enumerate() {
        if let Some(k) = k {
            if last_idx.get(k).copied() != Some(i) {
                continue;
            }
        }
        out.extend(lines.iter().copied());
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NoteType;

    fn temp_vault() -> (Vault, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path().to_path_buf()).unwrap();
        (vault, dir)
    }

    #[test]
    fn reload_prunes_heavy_dirs() {
        // Regression: a vault that contains a code project must NOT walk target/
        // node_modules/ .git/ — only real notes get indexed, fast.
        let (mut vault, _dir) = temp_vault();
        std::fs::write(vault.root.join("real.md"), "# Real\nnota de verdade").unwrap();
        for heavy in ["target", "node_modules", ".git"] {
            let d = vault.root.join(heavy).join("deep");
            std::fs::create_dir_all(&d).unwrap();
            // A .md hidden inside a pruned dir must be ignored.
            std::fs::write(d.join("buried.md"), "# Buried").unwrap();
        }
        vault.reload_notes();
        let titles: Vec<&str> = vault.notes.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["real"], "só a nota fora dos dirs podados");
        assert!(is_pruned_dir(&vault.root.join("target")));
        assert!(!is_pruned_dir(&vault.root.join("real.md"))); // arquivo, não dir
    }

    #[test]
    fn is_loadable_accepts_text_kinds() {
        for ok in ["a.md", "b.txt", "c.env", ".env", ".env.local"] {
            assert!(is_loadable(Path::new(ok)), "{ok} deveria carregar");
        }
        for no in ["x.png", "y.rs", "z.json"] {
            assert!(!is_loadable(Path::new(no)), "{no} não deveria carregar");
        }
    }

    #[test]
    fn is_pruned_dir_prunes_dotfolders() {
        // is_pruned_dir checks is_dir(), so the dirs must exist on disk.
        let dir = tempfile::tempdir().unwrap();
        for d in [".obsidian", ".vscode", "_attachments", "target", "Notes"] {
            std::fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        assert!(is_pruned_dir(&dir.path().join(".obsidian")));
        assert!(is_pruned_dir(&dir.path().join(".vscode")));
        assert!(is_pruned_dir(&dir.path().join("_attachments")));
        assert!(is_pruned_dir(&dir.path().join("target")));
        assert!(!is_pruned_dir(&dir.path().join("Notes")));
    }

    #[test]
    fn reload_loads_txt_and_env_ignores_binary() {
        let (mut vault, _dir) = temp_vault();
        std::fs::write(vault.root.join("note.md"), "# Note").unwrap();
        std::fs::write(vault.root.join("readme.txt"), "plain text").unwrap();
        std::fs::write(vault.root.join(".env"), "KEY=value").unwrap();
        std::fs::write(vault.root.join("logo.png"), b"\x89PNG binary").unwrap();
        vault.reload_notes();
        assert_eq!(vault.notes.len(), 3, "md + txt + env, sem o png");
        let rels: Vec<String> = vault
            .notes
            .iter()
            .map(|n| n.rel_path.to_string_lossy().to_string())
            .collect();
        assert!(rels.iter().any(|r| r == "note.md"));
        assert!(rels.iter().any(|r| r == "readme.txt"));
        assert!(rels.iter().any(|r| r == ".env"));
        assert!(
            !rels.iter().any(|r| r.contains("logo.png")),
            "png não pode carregar; got {rels:?}"
        );
    }

    #[test]
    fn list_folders_excludes_dotfolders() {
        let (mut vault, _dir) = temp_vault();
        for d in ["Notes", ".obsidian", ".omninote"] {
            std::fs::create_dir_all(vault.root.join(d)).unwrap();
        }
        // `list_folders` reads a cache refreshed on reload/mutation, not the live
        // disk — reload after the external mkdir so the cache reflects it.
        vault.reload_notes();
        let folders: Vec<String> = vault
            .list_folders()
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        assert!(folders.iter().any(|f| f == "Notes"));
        assert!(
            !folders.iter().any(|f| f.contains(".obsidian")),
            "dotfolder leaked: {folders:?}"
        );
        assert!(
            !folders.iter().any(|f| f.contains(".omninote")),
            "dotfolder leaked: {folders:?}"
        );
    }

    #[test]
    fn list_folders_is_sorted_and_cached() {
        let (mut vault, _dir) = temp_vault();
        for d in ["Zebra", "Alpha", "Mango"] {
            std::fs::create_dir_all(vault.root.join(d)).unwrap();
        }
        vault.reload_notes();
        let folders: Vec<String> = vault
            .list_folders()
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        assert_eq!(folders, ["Alpha", "Mango", "Zebra"]);
        // create_folder refreshes the cache without an explicit reload.
        vault.create_folder(None, "Beta").unwrap();
        let after: Vec<String> = vault
            .list_folders()
            .iter()
            .map(|f| f.to_string_lossy().to_string())
            .collect();
        assert_eq!(after, ["Alpha", "Beta", "Mango", "Zebra"]);
    }

    #[test]
    fn save_note_txt_is_raw() {
        let (vault, _dir) = temp_vault();
        let path = vault.root.join("config.txt");
        let note = Note {
            path: path.clone(),
            rel_path: PathBuf::from("config.txt"),
            frontmatter: Frontmatter::default(),
            title: "config".into(),
            content: "KEY=value\nFOO=bar\n".into(),
        };
        vault.save_note(&note).unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "KEY=value\nFOO=bar\n");
        assert!(
            !on_disk.contains("---"),
            ".txt não pode ter header de frontmatter; got:\n{on_disk}"
        );
    }

    #[test]
    fn set_note_type_persists_for_md() {
        let (mut v, _d) = temp_vault();
        let md = v
            .create_note(None, "Categorizar", NoteType::Resumo)
            .unwrap();
        v.set_note_type(&md.frontmatter.id, NoteType::Codigo)
            .unwrap();
        let reloaded = v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == md.frontmatter.id)
            .expect("nota recarregada");
        assert_eq!(reloaded.frontmatter.note_type, NoteType::Codigo);

        // Non-md note: set_note_type is rejected.
        std::fs::write(v.root.join("settings.env"), "A=1").unwrap();
        v.reload_notes();
        let env_id = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "settings.env")
            .expect("env note loaded")
            .frontmatter
            .id
            .clone();
        assert!(v.set_note_type(&env_id, NoteType::Codigo).is_err());
    }

    #[test]
    fn set_folder_note_type_recurses_subtree() {
        let (mut v, _d) = temp_vault();
        v.create_note(Some(Path::new("Projetos")), "A", NoteType::Resumo)
            .unwrap();
        v.create_note(Some(Path::new("Projetos/Sub")), "B", NoteType::Resumo)
            .unwrap();
        v.create_note(None, "Raiz", NoteType::Resumo).unwrap();

        let changed = v
            .set_folder_note_type(Path::new("Projetos"), NoteType::Codigo)
            .unwrap();
        assert_eq!(changed, 2, "só as duas notas sob Projetos");

        // ids são regenerados no create; casa por título/rel_path.
        let a = v.notes.iter().find(|n| n.title == "A").unwrap();
        let b = v.notes.iter().find(|n| n.title == "B").unwrap();
        let raiz = v.notes.iter().find(|n| n.title == "Raiz").unwrap();
        assert_eq!(a.frontmatter.note_type, NoteType::Codigo);
        assert_eq!(b.frontmatter.note_type, NoteType::Codigo);
        assert_eq!(raiz.frontmatter.note_type, NoteType::Resumo);
    }

    #[test]
    fn set_folder_note_type_skips_non_md() {
        let (mut v, _d) = temp_vault();
        v.create_note(Some(Path::new("Docs")), "Guia", NoteType::Resumo)
            .unwrap();
        std::fs::write(v.root.join("Docs/readme.txt"), "x").unwrap();
        v.reload_notes();

        let changed = v
            .set_folder_note_type(Path::new("Docs"), NoteType::Citacao)
            .unwrap();
        assert_eq!(changed, 1, "só o .md conta");

        let txt = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "Docs/readme.txt")
            .expect("txt note loaded");
        assert_eq!(txt.frontmatter.note_type, NoteType::Resumo);
    }

    #[test]
    fn set_folder_note_type_empty_folder_returns_zero() {
        let (mut v, _d) = temp_vault();
        let changed = v
            .set_folder_note_type(Path::new("Inexistente"), NoteType::Duvida)
            .unwrap();
        assert_eq!(changed, 0);
    }

    #[cfg(unix)]
    #[test]
    fn set_folder_note_type_preflight_rejects_readonly() {
        // Atomicity guard (CAD-25b Slice 4): one read-only target must abort the
        // whole bulk op BEFORE any writable sibling is rewritten (validate-all-
        // then-write). Unix-only: relies on chmod semantics.
        use std::os::unix::fs::PermissionsExt;
        let (mut v, _d) = temp_vault();
        v.create_note(Some(Path::new("Bulk")), "Editavel", NoteType::Resumo)
            .unwrap();
        let locked = v
            .create_note(Some(Path::new("Bulk")), "Travada", NoteType::Resumo)
            .unwrap();
        let mut perms = std::fs::metadata(&locked.path).unwrap().permissions();
        perms.set_mode(0o444);
        std::fs::set_permissions(&locked.path, perms).unwrap();

        let res = v.set_folder_note_type(Path::new("Bulk"), NoteType::Codigo);
        assert!(res.is_err(), "alvo read-only deve abortar o bulk inteiro");

        // The writable sibling must NOT have been flipped — pre-flight ran first.
        v.reload_notes();
        let editavel = v.notes.iter().find(|n| n.title == "Editavel").unwrap();
        assert_eq!(
            editavel.frontmatter.note_type,
            NoteType::Resumo,
            "irmão gravável não pode mudar quando o bulk aborta"
        );

        // Restore perms so the TempDir cleanup succeeds.
        let mut perms = std::fs::metadata(&locked.path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&locked.path, perms).unwrap();
    }

    #[test]
    fn append_inbox_prepends_newest_first_under_heading() {
        let (vault, _dir) = temp_vault();
        vault.append_inbox_line("primeira").unwrap();
        vault.append_inbox_line("segunda").unwrap();
        let content = std::fs::read_to_string(vault.root.join("Inbox.md")).unwrap();
        assert!(content.starts_with("# Inbox\n"));
        let pos_seg = content.find("segunda").unwrap();
        let pos_pri = content.find("primeira").unwrap();
        // Newest (segunda) appears before primeira.
        assert!(pos_seg < pos_pri);
        // Empty input rejected.
        assert!(vault.append_inbox_line("   ").is_err());
    }

    #[test]
    fn capture_line_appends_bullet_and_reports_total() {
        let (vault, _dir) = temp_vault();
        let out = vault.capture_line("comprar leite").unwrap();
        assert_eq!(out.path, vault.root.join("Inbox.md"));
        assert!(out.bullet.starts_with("- "));
        assert!(out.bullet.ends_with("· comprar leite"));
        assert_eq!(out.total_lines, 1);
        // The reported bullet is exactly what landed on disk.
        let content = std::fs::read_to_string(&out.path).unwrap();
        assert!(content.contains(&out.bullet));

        let out2 = vault.capture_line("comprar pão").unwrap();
        assert_eq!(out2.total_lines, 2);
    }

    #[test]
    fn capture_line_prepends_newest_first() {
        let (vault, _dir) = temp_vault();
        vault.capture_line("primeira").unwrap();
        let newest = vault.capture_line("segunda").unwrap();
        // The outcome's bullet is the just-written (newest) line.
        assert!(newest.bullet.contains("segunda"));
        let content = std::fs::read_to_string(&newest.path).unwrap();
        assert!(content.find("segunda").unwrap() < content.find("primeira").unwrap());
    }

    #[test]
    fn capture_line_rejects_empty_and_whitespace() {
        let (vault, _dir) = temp_vault();
        assert_eq!(vault.capture_line(""), Err("linha vazia".into()));
        assert_eq!(vault.capture_line("   \t  "), Err("linha vazia".into()));
        // A rejected capture must not create the inbox.
        assert!(!vault.root.join("Inbox.md").exists());
    }

    #[test]
    fn capture_line_creates_inbox_when_absent() {
        let (vault, _dir) = temp_vault();
        assert!(!vault.root.join("Inbox.md").exists());
        let out = vault.capture_line("first ever").unwrap();
        let content = std::fs::read_to_string(&out.path).unwrap();
        assert!(content.starts_with("# Inbox\n"));
        assert_eq!(out.total_lines, 1);
    }

    #[test]
    fn capture_line_preserves_existing_non_heading_body() {
        let (vault, _dir) = temp_vault();
        // A pre-existing Inbox with prose but no recognized `# Inbox` heading:
        // the bullet is prepended and the prior body is preserved below it.
        std::fs::write(vault.root.join("Inbox.md"), "prosa antiga\nmais prosa\n").unwrap();
        let out = vault.capture_line("nova captura").unwrap();
        let content = std::fs::read_to_string(&out.path).unwrap();
        assert!(content.contains("prosa antiga"));
        assert!(content.contains("mais prosa"));
        assert!(content.contains("nova captura"));
        // Bullet precedes the preserved prose.
        assert!(content.find("nova captura").unwrap() < content.find("prosa antiga").unwrap());
    }

    #[test]
    fn capture_outcome_total_lines_counts_only_bullet_lines() {
        let (vault, _dir) = temp_vault();
        vault.capture_line("um").unwrap();
        vault.capture_line("dois").unwrap();
        let out = vault.capture_line("tres").unwrap();
        // Three captures → three bullets; the `# Inbox` heading + blank line do
        // not count toward total_lines.
        assert_eq!(out.total_lines, 3);
        let content = std::fs::read_to_string(&out.path).unwrap();
        let non_bullet = content
            .lines()
            .filter(|l| !l.starts_with("- ") && !l.is_empty())
            .count();
        // Only the heading line is a non-bullet, non-empty line.
        assert_eq!(non_bullet, 1);
    }

    #[test]
    fn append_inbox_never_leaves_partial_and_preserves_existing() {
        let (vault, _dir) = temp_vault();
        // Seed an existing inbox with a prior bullet.
        vault.append_inbox_line("primeira").unwrap();
        let before = std::fs::read_to_string(vault.root.join("Inbox.md")).unwrap();
        assert!(before.contains("primeira"));

        // A second capture: the atomic rewrite must keep the file whole.
        vault.append_inbox_line("segunda").unwrap();
        let inbox = vault.root.join("Inbox.md");
        let after = std::fs::read_to_string(&inbox).unwrap();
        // Never empty/truncated; the heading and the prior bullet survive.
        assert!(!after.is_empty());
        assert!(after.starts_with("# Inbox\n"));
        assert!(after.contains("primeira"));
        assert!(after.contains("segunda"));

        // No staging temp file is left behind in the vault dir.
        let leftover: Vec<_> = std::fs::read_dir(&vault.root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftover.is_empty(), "stray temp file left in vault");
    }

    #[test]
    fn attachment_path_blocks_traversal_and_accepts_legit() {
        let (vault, _dir) = temp_vault();
        let attach = vault.root.join("_attachments");
        std::fs::create_dir_all(&attach).unwrap();
        std::fs::write(attach.join("foto.png"), b"img").unwrap();
        // Also drop a file OUTSIDE _attachments to try to escape to.
        std::fs::write(vault.root.join("secret.md"), b"top secret").unwrap();

        // Legit single-component filename resolves.
        assert!(vault.attachment_path("foto.png").is_some());

        // Traversal attempts are all rejected (CWE-22).
        assert!(vault.attachment_path("../secret.md").is_none());
        assert!(vault.attachment_path("../../etc/passwd").is_none());
        assert!(vault.attachment_path("sub/foto.png").is_none());
        assert!(vault.attachment_path("/etc/passwd").is_none());
        assert!(vault.attachment_path("..").is_none());
        assert!(vault.attachment_path("").is_none());
        // Nonexistent legit name → None (doesn't exist), not a panic.
        assert!(vault.attachment_path("ghost.png").is_none());
    }

    #[test]
    fn open_rejects_file_path() {
        // Q-08: passing a file (not a directory) used to return Ok with an empty
        // vault — confusing. Now rejected explicitly.
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("not-a-vault.txt");
        std::fs::write(&file_path, "this is a file, not a vault").unwrap();
        match Vault::open(file_path.clone()) {
            Err(msg) => assert!(
                msg.contains("file"),
                "error should mention 'file', got: {msg}"
            ),
            Ok(_) => panic!(
                "Vault::open should reject file path: {}",
                file_path.display()
            ),
        }
    }

    #[test]
    fn foreign_note_without_frontmatter_id_gets_rel_path_fallback() {
        // CAD-23.1 RAG bug regression. Notes from foreign vaults (Obsidian,
        // raw markdown) lack OmniNote `id:` frontmatter. They used to all
        // collide on id="" in downstream HashMap-keyed flows. Now each gets
        // a unique id derived from rel_path.
        let dir = tempfile::tempdir().unwrap();
        // Plain markdown — no frontmatter at all.
        std::fs::write(dir.path().join("alpha.md"), "# Alpha\n\nbody").unwrap();
        std::fs::write(dir.path().join("beta.md"), "# Beta\n\nbody").unwrap();
        let v = Vault::open(dir.path().to_path_buf()).unwrap();
        let ids: std::collections::HashSet<&str> =
            v.notes.iter().map(|n| n.frontmatter.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            v.notes.len(),
            "every foreign note must get a unique fallback id, got ids={ids:?}"
        );
        for n in &v.notes {
            assert!(
                !n.frontmatter.id.is_empty(),
                "no note should have empty id; got note {}",
                n.title
            );
            assert_eq!(
                n.frontmatter.id,
                n.rel_path.to_string_lossy().to_string(),
                "fallback id should match rel_path"
            );
        }
    }

    #[test]
    fn omninote_native_note_keeps_uuid_id() {
        // Notes created via Vault::create_note get a `n_<uuid>` id and the
        // fallback should NOT overwrite it.
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Nativa", NoteType::Resumo).unwrap();
        let original_id = note.frontmatter.id.clone();
        assert!(
            original_id.starts_with("n_"),
            "expected n_<uuid>, got {original_id}"
        );
        v.reload_notes();
        let reloaded = v
            .notes
            .iter()
            .find(|n| n.title == "Nativa")
            .expect("note should be reloaded");
        assert_eq!(reloaded.frontmatter.id, original_id);
    }

    #[test]
    fn create_and_reload_note() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "Teste", NoteType::Resumo).unwrap();
        assert_eq!(v.notes.len(), 1);
        assert_eq!(v.notes[0].title, "Teste");
        v.reload_notes();
        assert_eq!(v.notes.len(), 1);
    }

    #[test]
    fn frontmatter_roundtrip() {
        let (mut v, _d) = temp_vault();
        let mut note = v.create_note(None, "Cita", NoteType::Citacao).unwrap();
        note.frontmatter.source = "Livro X".into();
        note.frontmatter.tags = vec!["rust".into()];
        v.save_note(&note).unwrap();
        v.reload_notes();
        let loaded = v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap();
        assert_eq!(loaded.frontmatter.note_type, NoteType::Citacao);
        assert_eq!(loaded.frontmatter.source, "Livro X");
        assert_eq!(loaded.frontmatter.tags, vec!["rust"]);
    }

    #[test]
    fn frontmatter_summary_roundtrip() {
        // CAD-23.2: summary field survives save+reload.
        let (mut v, _d) = temp_vault();
        let mut note = v.create_note(None, "Sumarizada", NoteType::Resumo).unwrap();
        note.frontmatter.summary = "Uma frase resumindo a nota.".into();
        v.save_note(&note).unwrap();
        v.reload_notes();
        let loaded = v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap();
        assert_eq!(loaded.frontmatter.summary, "Uma frase resumindo a nota.");
    }

    #[test]
    fn frontmatter_summary_omitted_when_empty() {
        // CAD-23.2: empty summary must not appear in serialized YAML so
        // existing notes stay byte-clean.
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "SemResumo", NoteType::Resumo).unwrap();
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&note.path).unwrap();
        assert!(
            !raw.contains("summary:"),
            "empty summary should NOT appear in YAML; got:\n{raw}"
        );
    }

    #[test]
    fn frontmatter_flatten_roundtrip_mixed_value_types() {
        // Probe (CAD-25b Slice 4): serde(flatten) into BTreeMap<_, serde_yaml::Value>
        // must round-trip mixed YAML value types — string, bool, integer, list —
        // not just strings. Guards the known serde_yaml flatten-typing quirk where
        // non-string scalars can mis-deserialize. If this fails, the flatten approach
        // is unviable and the fix must switch to a raw serde_yaml::Mapping merge.
        let yaml =
            "id: x1\ntype: resumo\ncssclass: wide\npublish: true\nrating: 5\nareas:\n- a\n- b\n";
        let fm: Frontmatter = serde_yaml::from_str(yaml).unwrap();
        assert!(fm.extra.contains_key("cssclass"), "extra={:?}", fm.extra);
        assert!(fm.extra.contains_key("publish"), "extra={:?}", fm.extra);
        assert!(fm.extra.contains_key("rating"), "extra={:?}", fm.extra);
        assert!(fm.extra.contains_key("areas"), "extra={:?}", fm.extra);
        let out = serde_yaml::to_string(&fm).unwrap();
        assert!(out.contains("publish: true"), "bool preserved:\n{out}");
        assert!(out.contains("rating: 5"), "integer preserved:\n{out}");
        assert!(out.contains("cssclass: wide"), "string preserved:\n{out}");
        // Assert the ITEMS, not just the `areas:` key — a key-only contains()
        // passes even if the list items are dropped. serde_yaml emits a block
        // list (`areas:\n- a\n- b`), so check the items directly.
        assert!(out.contains("- a"), "list item a preserved:\n{out}");
        assert!(out.contains("- b"), "list item b preserved:\n{out}");
    }

    #[test]
    fn frontmatter_preserves_unknown_obsidian_fields() {
        // CAD-25b Slice 4 (data-loss fix): custom Obsidian keys must survive a
        // parse → reload → save round-trip instead of being silently dropped by
        // the fixed serde struct.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("custom.md");
        std::fs::write(
            &path,
            "---\nid: x1\ntype: resumo\ncssclass: wide-page\npublish: true\n\
             cover: img/banner.png\nrating: 5\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "custom.md")
            .expect("nota carregada")
            .clone();
        for k in ["cssclass", "publish", "cover", "rating"] {
            assert!(note.frontmatter.extra.contains_key(k), "extra perdeu '{k}'");
        }
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("cssclass: wide-page"), "got:\n{raw}");
        assert!(raw.contains("publish: true"), "got:\n{raw}");
        assert!(raw.contains("cover: img/banner.png"), "got:\n{raw}");
        assert!(raw.contains("rating: 5"), "got:\n{raw}");
    }

    #[test]
    fn foreign_note_unknown_type_preserves_other_keys() {
        // Review finding (major): a foreign note whose `type:` is not an OmniNote
        // variant used to wipe the ENTIRE frontmatter (strict deserialize failed →
        // unwrap_or_default). Now the unknown type normalizes to the default but
        // sibling keys survive in `extra`.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("book.md");
        std::fs::write(
            &path,
            "---\nid: b1\ntype: book\nauthor: Tolkien\nrating: 5\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "book.md")
            .expect("nota carregada")
            .clone();
        assert!(
            note.frontmatter.extra.contains_key("author"),
            "author perdido"
        );
        assert!(
            note.frontmatter.extra.contains_key("rating"),
            "rating perdido"
        );
        assert_eq!(
            note.frontmatter.note_type,
            NoteType::Resumo,
            "type desconhecido normaliza p/ default"
        );
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("author: Tolkien"), "got:\n{raw}");
        assert!(raw.contains("rating: 5"), "got:\n{raw}");
    }

    #[test]
    fn foreign_note_missing_id_preserves_custom_keys() {
        // Review finding (major): a `---` block with no `id:` (Obsidian dashboard /
        // folder notes) used to drop cssclass/publish/cover. Now they survive; the
        // id is derived from rel_path by read_note.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("dash.md");
        std::fs::write(
            &path,
            "---\ntype: resumo\ncssclass: dashboard\npublish: false\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "dash.md")
            .expect("nota carregada")
            .clone();
        assert!(
            note.frontmatter.extra.contains_key("cssclass"),
            "cssclass perdido"
        );
        assert!(
            note.frontmatter.extra.contains_key("publish"),
            "publish perdido"
        );
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("cssclass: dashboard"), "got:\n{raw}");
        assert!(raw.contains("publish: false"), "got:\n{raw}");
    }

    #[test]
    fn foreign_note_scalar_tags_preserves_other_keys() {
        // Review finding (major): a scalar `tags: rust` (valid YAML that Obsidian
        // writes) used to fail Vec<String> deserialization and wipe everything.
        // Now the scalar is accepted as a single-element list; custom keys survive.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("scalar.md");
        std::fs::write(
            &path,
            "---\nid: s1\ntype: resumo\ntags: rust\ncssclass: note\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "scalar.md")
            .expect("nota carregada")
            .clone();
        assert_eq!(
            note.frontmatter.tags,
            vec!["rust"],
            "tag escalar vira lista de um elemento"
        );
        assert!(
            note.frontmatter.extra.contains_key("cssclass"),
            "cssclass perdido"
        );
    }

    #[test]
    fn foreign_note_duplicate_key_keeps_last_and_preserves_others() {
        // Round-2 finding (major): serde_yaml rejects duplicate keys, which used to
        // wipe the WHOLE note. The dedup retry (keep-last) recovers it — modeled and
        // custom keys survive; the duplicated key takes its last value.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("dup.md");
        std::fs::write(
            &path,
            "---\nid: keepme\ntype: resumo\ncssclass: a\ncssclass: b\nauthor: Tolkien\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "dup.md")
            .expect("nota carregada")
            .clone();
        assert_eq!(note.frontmatter.id, "keepme", "id explícito não pode sumir");
        assert!(
            note.frontmatter.extra.contains_key("author"),
            "author perdido"
        );
        assert_eq!(
            note.frontmatter
                .extra
                .get("cssclass")
                .and_then(|x| x.as_str()),
            Some("b"),
            "duplicata fica com o último valor (keep-last)"
        );
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("author: Tolkien"), "got:\n{raw}");
    }

    #[test]
    fn foreign_note_non_string_key_preserved_as_string() {
        // Round-2 finding (regression): a non-string YAML key (`2024:`, `true:`) was
        // dropped by the new parse though the old code coerced+kept it. Now it is
        // coerced to its string form and survives in `extra`.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("numkey.md");
        std::fs::write(
            &path,
            "---\nid: n1\ntype: resumo\n2024: retro\ntrue: yes\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "numkey.md")
            .expect("nota carregada")
            .clone();
        assert!(
            note.frontmatter.extra.contains_key("2024"),
            "chave numérica perdida: {:?}",
            note.frontmatter.extra
        );
        assert!(
            note.frontmatter.extra.contains_key("true"),
            "chave bool perdida"
        );
    }

    #[test]
    fn frontmatter_flatten_does_not_duplicate_known_keys() {
        // serde(flatten) must route modeled keys to their named fields, never
        // ALSO into `extra` — a duplicate would double-emit and corrupt the YAML.
        let (mut v, _d) = temp_vault();
        let path = v.root.join("known.md");
        std::fs::write(
            &path,
            "---\nid: k1\ntype: codigo\ntags:\n- rust\nsource: Livro\n\
             summary: uma frase\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let note = v
            .notes
            .iter()
            .find(|n| n.rel_path.to_string_lossy() == "known.md")
            .unwrap()
            .clone();
        for k in ["id", "type", "tags", "source", "summary"] {
            assert!(
                !note.frontmatter.extra.contains_key(k),
                "chave conhecida '{k}' vazou para extra: {:?}",
                note.frontmatter.extra
            );
        }
        assert_eq!(note.frontmatter.source, "Livro");
        assert_eq!(note.frontmatter.summary, "uma frase");
        assert_eq!(note.frontmatter.tags, vec!["rust"]);
    }

    #[test]
    fn frontmatter_empty_extra_omitted_from_yaml() {
        // No custom fields → serialized YAML stays byte-clean (flatten of an empty
        // map emits nothing), so OmniNote-native notes are unchanged.
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Limpa", NoteType::Resumo).unwrap();
        v.save_note(&note).unwrap();
        let raw = std::fs::read_to_string(&note.path).unwrap();
        assert!(!raw.contains("extra:"), "no extra: key; got:\n{raw}");
        assert!(!raw.contains("{}"), "no empty map literal; got:\n{raw}");
    }

    #[test]
    fn set_folder_note_type_preserves_custom_fields() {
        // The bulk type-flip must not strip Obsidian custom keys while it runs.
        let (mut v, _d) = temp_vault();
        let dir = v.root.join("Area");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("doc.md"),
            "---\nid: a1\ntype: resumo\ncssclass: special\n---\n\nCorpo.",
        )
        .unwrap();
        v.reload_notes();
        let changed = v
            .set_folder_note_type(Path::new("Area"), NoteType::Codigo)
            .unwrap();
        assert_eq!(changed, 1);
        let raw = std::fs::read_to_string(dir.join("doc.md")).unwrap();
        assert!(raw.contains("type: codigo"), "type flipped; got:\n{raw}");
        assert!(
            raw.contains("cssclass: special"),
            "custom key kept; got:\n{raw}"
        );
    }

    #[test]
    fn delete_note_removes_from_disk() {
        let (mut v, _d) = temp_vault();
        v.create_note(None, "Del", NoteType::Resumo).unwrap();
        v.delete_note(0).unwrap();
        v.reload_notes();
        assert_eq!(v.notes.len(), 0);
    }

    #[test]
    fn move_note_to_folder() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Mover", NoteType::Resumo).unwrap();
        v.create_folder(None, "Destino").unwrap();
        v.move_note_by_id(&note.frontmatter.id, Some(Path::new("Destino")))
            .unwrap();
        let moved = v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap();
        assert!(moved.rel_path.starts_with("Destino"));
        assert!(moved.path.exists());
        assert!(!note.path.exists());
    }

    #[test]
    fn move_note_to_root() {
        let (mut v, _d) = temp_vault();
        let abs = v.create_folder(None, "Sub").unwrap();
        let rel = abs.strip_prefix(&v.root).unwrap().to_path_buf();
        let note = v
            .create_note(Some(&rel), "Volta", NoteType::Resumo)
            .unwrap();
        v.move_note_by_id(&note.frontmatter.id, None).unwrap();
        let moved = v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap();
        // rel_path at root has no parent component
        assert_eq!(moved.rel_path.components().count(), 1);
    }

    #[test]
    fn move_note_collision_errors() {
        let (mut v, _d) = temp_vault();
        let n1 = v.create_note(None, "Igual", NoteType::Resumo).unwrap();
        v.create_folder(None, "Pasta").unwrap();
        v.create_note(Some(Path::new("Pasta")), "Igual", NoteType::Resumo)
            .unwrap();
        // n1 at root has filename "Igual.md"; Pasta also has "Igual.md" → collision
        let result = v.move_note_by_id(&n1.frontmatter.id, Some(Path::new("Pasta")));
        assert!(result.is_err());
    }

    #[test]
    fn create_folder_appears_in_list() {
        let (mut v, _d) = temp_vault();
        v.create_folder(None, "Estudos").unwrap();
        assert!(v
            .list_folders()
            .iter()
            .any(|f| f.to_string_lossy().contains("Estudos")));
    }

    #[test]
    fn note_in_subfolder() {
        let (mut v, _d) = temp_vault();
        let abs = v.create_folder(None, "Sub").unwrap();
        let rel = abs.strip_prefix(&v.root).unwrap().to_path_buf();
        v.create_note(Some(&rel), "Deep", NoteType::Resumo).unwrap();
        let note = v.notes.iter().find(|n| n.title == "Deep").unwrap();
        assert!(note.rel_path.starts_with("Sub"));
    }

    #[test]
    fn parse_frontmatter_extracts_fields() {
        let raw = "---\nid: abc\ntype: codigo\ntags:\n- rust\nsource: ''\nsource_link: ''\ncreated: ''\n---\n\nBody.";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "abc");
        assert_eq!(fm.note_type, NoteType::Codigo);
        assert_eq!(body.trim(), "Body.");
    }
}
