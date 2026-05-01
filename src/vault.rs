use crate::types::{AppConfig, Frontmatter, Note, NoteType};
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub config: AppConfig,
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        if !root.exists() { fs::create_dir_all(&root).map_err(|e| e.to_string())?; }
        let cfg_dir = root.join(".caderno");
        let _ = fs::create_dir_all(&cfg_dir);
        let _ = fs::create_dir_all(root.join("_attachments"));

        let config: AppConfig = fs::read_to_string(cfg_dir.join("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut v = Self { root, notes: Vec::new(), config };
        v.reload_notes();
        Ok(v)
    }

    pub fn reload_notes(&mut self) {
        self.notes.clear();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() { continue; }
            let path_str = path.to_string_lossy();
            if path_str.contains("/.caderno/") || path_str.contains("\\.caderno\\") { continue; }
            if path_str.contains("/_attachments/") || path_str.contains("\\_attachments\\") { continue; }
            if path.extension().and_then(|s| s.to_str()) != Some("md") { continue; }
            if let Ok(note) = self.read_note(path) {
                self.notes.push(note);
            }
        }
    }

    fn read_note(&self, path: &Path) -> Result<Note, String> {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let (frontmatter, content) = parse_frontmatter(&raw);
        let title = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sem título").to_string();
        let rel_path = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
        Ok(Note { path: path.to_path_buf(), rel_path, frontmatter, title, content })
    }

    pub fn save_note(&self, note: &Note) -> Result<(), String> {
        let mut out = String::from("---\n");
        out.push_str(&serde_yaml::to_string(&note.frontmatter).map_err(|e| e.to_string())?);
        out.push_str("---\n\n");
        out.push_str(&note.content);
        if let Some(parent) = note.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&note.path, out).map_err(|e| e.to_string())
    }

    pub fn create_note(&mut self, folder: Option<&Path>, title: &str, note_type: NoteType) -> Result<Note, String> {
        let folder_path = folder.map(|f| self.root.join(f)).unwrap_or_else(|| self.root.clone());
        fs::create_dir_all(&folder_path).map_err(|e| e.to_string())?;
        let safe_title = if title.is_empty() { "Nova nota".to_string() } else { sanitize_filename(title) };
        let mut path = folder_path.join(format!("{}.md", safe_title));
        let mut counter = 1;
        while path.exists() {
            path = folder_path.join(format!("{} ({}).md", safe_title, counter));
            counter += 1;
        }
        let id = format!("n_{}", uuid::Uuid::new_v4().simple());
        let frontmatter = Frontmatter {
            id, note_type, tags: vec![], source: String::new(), source_link: String::new(),
            linked_note: None, attachments: vec![],
            created: chrono::Utc::now().to_rfc3339(),
        };
        let title_str = path.file_stem().unwrap().to_string_lossy().to_string();
        let rel_path = path.strip_prefix(&self.root).unwrap().to_path_buf();
        let note = Note { path: path.clone(), rel_path, frontmatter, title: title_str, content: String::new() };
        self.save_note(&note)?;
        self.notes.push(note.clone());
        Ok(note)
    }

    pub fn rename_note(&mut self, idx: usize, new_title: &str) -> Result<(), String> {
        let safe = sanitize_filename(new_title);
        let parent = self.notes[idx].path.parent().unwrap().to_path_buf();
        let new_path = parent.join(format!("{}.md", safe));
        if new_path == self.notes[idx].path { return Ok(()); }
        fs::rename(&self.notes[idx].path, &new_path).map_err(|e| e.to_string())?;
        self.notes[idx].path = new_path.clone();
        self.notes[idx].rel_path = new_path.strip_prefix(&self.root).unwrap().to_path_buf();
        self.notes[idx].title = safe;
        Ok(())
    }

    pub fn delete_note(&mut self, idx: usize) -> Result<(), String> {
        fs::remove_file(&self.notes[idx].path).map_err(|e| e.to_string())?;
        self.notes.remove(idx);
        Ok(())
    }

    pub fn create_folder(&mut self, parent: Option<&Path>, name: &str) -> Result<PathBuf, String> {
        let safe = sanitize_filename(name);
        let parent_abs = parent.map(|p| self.root.join(p)).unwrap_or_else(|| self.root.clone());
        let path = parent_abs.join(safe);
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        Ok(path)
    }

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
        let mut out = Vec::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            if !entry.path().is_dir() || entry.path() == self.root { continue; }
            let rel = entry.path().strip_prefix(&self.root).unwrap();
            let s = rel.to_string_lossy();
            if s.starts_with(".caderno") || s.starts_with("_attachments") { continue; }
            out.push(rel.to_path_buf());
        }
        out
    }

    pub fn import_attachment(&self, src: &Path) -> Result<String, String> {
        let attach_dir = self.root.join("_attachments");
        fs::create_dir_all(&attach_dir).map_err(|e| e.to_string())?;
        let original_name = src.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        let safe = sanitize_filename(original_name);
        let mut dest = attach_dir.join(&safe);
        let mut counter = 1;
        while dest.exists() {
            let stem = Path::new(&safe).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = Path::new(&safe).extension().and_then(|s| s.to_str()).unwrap_or("");
            dest = attach_dir.join(format!("{}_{}.{}", stem, counter, ext));
            counter += 1;
        }
        fs::copy(src, &dest).map_err(|e| e.to_string())?;
        Ok(dest.file_name().unwrap().to_string_lossy().to_string())
    }

    pub fn save_config(&self) -> Result<(), String> {
        let p = self.root.join(".caderno").join("config.json");
        let s = serde_json::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        fs::write(p, s).map_err(|e| e.to_string())
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars().map(|c| match c {
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
        c => c,
    }).collect::<String>().trim().to_string()
}

pub fn parse_frontmatter(raw: &str) -> (Frontmatter, String) {
    if !raw.starts_with("---") {
        return (Frontmatter::default(), raw.to_string());
    }
    let after = &raw[3..];
    if let Some(end) = after.find("\n---") {
        let yaml = &after[..end].trim_start_matches('\n');
        let rest = &after[end + 4..].trim_start_matches('\n');
        let fm: Frontmatter = serde_yaml::from_str(yaml).unwrap_or_default();
        return (fm, rest.to_string());
    }
    (Frontmatter::default(), raw.to_string())
}
