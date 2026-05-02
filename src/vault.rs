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
        let cfg_dir = root.join(".omninote");
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
            if path_str.contains("/.omninote/") || path_str.contains("\\.omninote\\") { continue; }
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

    pub fn rename_note_by_id(&mut self, id: &str, new_title: &str) -> Result<Note, String> {
        let idx = self.notes.iter().position(|n| n.frontmatter.id == id)
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
    pub fn move_note_by_id(
        &mut self,
        id: &str,
        new_folder: Option<&Path>,
    ) -> Result<(), String> {
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
            return Err(format!(
                "Já existe um arquivo \"{}\" no destino",
                filename
            ));
        }
        fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
        self.notes[idx].path = new_path.clone();
        self.notes[idx].rel_path = new_path
            .strip_prefix(&self.root)
            .unwrap()
            .to_path_buf();
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
            if s.starts_with(".omninote") || s.starts_with("_attachments") { continue; }
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
        let p = self.root.join(".omninote").join("config.json");
        let s = serde_json::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        fs::write(p, s).map_err(|e| e.to_string())
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars().map(|c| match c {
        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
        c => c,
    }).collect::<String>().trim().to_string()
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
        let yaml = &after[..end].trim_start_matches('\n');
        let rest = &after[end + 4..].trim_start_matches('\n');
        let fm: Frontmatter = serde_yaml::from_str(yaml).unwrap_or_default();
        return (fm, rest.to_string());
    }
    (Frontmatter::default(), raw.to_string())
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
        let loaded = v.notes.iter().find(|n| n.frontmatter.id == note.frontmatter.id).unwrap();
        assert_eq!(loaded.frontmatter.note_type, NoteType::Citacao);
        assert_eq!(loaded.frontmatter.source, "Livro X");
        assert_eq!(loaded.frontmatter.tags, vec!["rust"]);
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
        let note = v.create_note(Some(&rel), "Volta", NoteType::Resumo).unwrap();
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
        assert!(v.list_folders().iter().any(|f| f.to_string_lossy().contains("Estudos")));
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
