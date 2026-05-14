use crate::types::{AppConfig, Frontmatter, Note, NoteType};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub config: AppConfig,
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self, String> {
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
        };
        v.reload_notes();
        Ok(v)
    }

    pub fn reload_notes(&mut self) {
        self.notes.clear();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let path_str = path.to_string_lossy();
            if path_str.contains("/.omninote/") || path_str.contains("\\.omninote\\") {
                continue;
            }
            if path_str.contains("/_attachments/") || path_str.contains("\\_attachments\\") {
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            if let Ok(note) = self.read_note(path) {
                self.notes.push(note);
            }
        }
    }

    fn read_note(&self, path: &Path) -> Result<Note, String> {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let (frontmatter, content) = parse_frontmatter(&raw);
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sem título")
            .to_string();
        let rel_path = path.strip_prefix(&self.root).unwrap_or(path).to_path_buf();
        Ok(Note {
            path: path.to_path_buf(),
            rel_path,
            frontmatter,
            title,
            content,
        })
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
        Ok(())
    }

    pub fn create_folder(&mut self, parent: Option<&Path>, name: &str) -> Result<PathBuf, String> {
        let safe = sanitize_filename(name);
        let parent_abs = parent
            .map(|p| self.root.join(p))
            .unwrap_or_else(|| self.root.clone());
        let path = parent_abs.join(safe);
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
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
        let mut out = Vec::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(|e| e.ok()) {
            if !entry.path().is_dir() || entry.path() == self.root {
                continue;
            }
            let rel = entry.path().strip_prefix(&self.root).unwrap();
            let s = rel.to_string_lossy();
            if s.starts_with(".omninote") || s.starts_with("_attachments") {
                continue;
            }
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

    // CAD-12: sanitize_filename adversarial coverage.

    #[test]
    fn sanitize_strips_path_separators_unix() {
        let s = sanitize_filename("../../etc/passwd");
        assert!(!s.contains('/'), "got {s:?}");
        assert_eq!(s, ".._.._etc_passwd");
    }

    #[test]
    fn sanitize_strips_path_separators_windows() {
        let s = sanitize_filename("..\\..\\windows\\system32");
        assert!(!s.contains('\\'), "got {s:?}");
    }

    #[test]
    fn sanitize_strips_dangerous_punctuation() {
        let s = sanitize_filename(r#"a/b\c:d*e?f"g<h>i|j"#);
        assert_eq!(s, "a_b_c_d_e_f_g_h_i_j");
    }

    #[test]
    fn sanitize_only_whitespace_becomes_empty() {
        assert_eq!(sanitize_filename("   "), "");
        assert_eq!(sanitize_filename("\t\n  "), "");
    }

    #[test]
    fn sanitize_preserves_unicode_letters_and_emoji() {
        assert_eq!(sanitize_filename("日本語"), "日本語");
        assert_eq!(sanitize_filename("português"), "português");
        assert_eq!(sanitize_filename("título 🔥"), "título 🔥");
    }

    #[test]
    fn sanitize_preserves_zero_width_chars() {
        // Existing behaviour — zero-width chars survive (gap noted in HUMAN.md Q-05)
        let zwsp = "\u{200B}";
        let bom = "\u{FEFF}";
        let s = sanitize_filename(&format!("foo{zwsp}bar{bom}"));
        assert!(s.contains(zwsp));
        assert!(s.contains(bom));
    }

    #[test]
    fn sanitize_handles_long_names() {
        let long = "a".repeat(1024);
        let s = sanitize_filename(&long);
        assert_eq!(s.len(), 1024);
    }

    #[test]
    fn sanitize_empty_stays_empty() {
        assert_eq!(sanitize_filename(""), "");
    }

    // CAD-12: Vault::open edge cases.

    #[test]
    fn open_creates_missing_root() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("does_not_exist_yet");
        let v = Vault::open(nested.clone()).unwrap();
        assert!(nested.exists());
        assert!(v.notes.is_empty());
    }

    #[test]
    fn open_with_file_root_does_not_panic() {
        // Documents current behaviour: passing a regular file as root does not error
        // because internal mkdir attempts use `let _ =`. Vault is constructed with
        // an empty notes list. Tracked as Q-08 in HUMAN.md.
        let dir = tempfile::tempdir().unwrap();
        let file_root = dir.path().join("not_a_dir.txt");
        fs::write(&file_root, b"x").unwrap();
        let v = Vault::open(file_root).unwrap();
        assert!(v.notes.is_empty());
    }

    // CAD-12: create_note path-traversal containment.

    #[test]
    fn create_note_with_traversal_title_stays_inside_vault() {
        let (mut v, _d) = temp_vault();
        let note = v
            .create_note(None, "../escape", NoteType::Resumo)
            .unwrap();
        let canonical_root = v.root.canonicalize().unwrap();
        let canonical_note = note.path.canonicalize().unwrap();
        assert!(
            canonical_note.starts_with(&canonical_root),
            "note escaped vault: {} not under {}",
            canonical_note.display(),
            canonical_root.display()
        );
        assert!(note
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("escape"));
    }

    #[test]
    fn create_note_with_empty_title_uses_default_label() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "", NoteType::Resumo).unwrap();
        assert_eq!(note.title, "Nova nota");
    }

    #[test]
    fn create_note_collision_appends_counter() {
        let (mut v, _d) = temp_vault();
        for _ in 0..5 {
            v.create_note(None, "Foo", NoteType::Resumo).unwrap();
        }
        let stems: Vec<String> = v
            .notes
            .iter()
            .map(|n| n.path.file_stem().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(stems.contains(&"Foo".to_string()));
        assert!(stems.contains(&"Foo (1)".to_string()));
        assert!(stems.contains(&"Foo (4)".to_string()));
    }

    // CAD-12: rename_note + move_note edges.

    #[test]
    fn rename_note_to_same_name_is_noop() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Mesmo", NoteType::Resumo).unwrap();
        let before = note.path.clone();
        v.rename_note(0, "Mesmo").unwrap();
        assert_eq!(v.notes[0].path, before);
    }

    #[test]
    fn move_note_by_id_unknown_errors() {
        let (mut v, _d) = temp_vault();
        let res = v.move_note_by_id("n_doesnotexist", None);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not found"));
    }

    #[test]
    fn move_note_by_id_to_same_location_is_noop() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "Stay", NoteType::Resumo).unwrap();
        v.move_note_by_id(&note.frontmatter.id, None).unwrap();
        assert!(v
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap()
            .path
            .exists());
    }

    #[test]
    fn move_note_creates_target_dir_on_demand() {
        let (mut v, _d) = temp_vault();
        let note = v.create_note(None, "X", NoteType::Resumo).unwrap();
        v.move_note_by_id(&note.frontmatter.id, Some(Path::new("Novo")))
            .unwrap();
        assert!(v.root.join("Novo").exists());
    }

    #[test]
    fn rename_note_by_id_unknown_errors() {
        let (mut v, _d) = temp_vault();
        let res = v.rename_note_by_id("n_nope", "Whatever");
        assert!(res.is_err());
    }

    // CAD-12: delete_folder.

    #[test]
    fn delete_folder_removes_recursively() {
        let (mut v, _d) = temp_vault();
        let abs = v.create_folder(None, "Deep").unwrap();
        let rel = abs.strip_prefix(&v.root).unwrap().to_path_buf();
        v.create_note(Some(&rel), "Inner", NoteType::Resumo).unwrap();
        v.delete_folder(&rel).unwrap();
        assert!(!v.root.join("Deep").exists());
        assert!(!v.notes.iter().any(|n| n.title == "Inner"));
    }

    #[test]
    fn delete_folder_unknown_errors() {
        let (mut v, _d) = temp_vault();
        let res = v.delete_folder(Path::new("does_not_exist"));
        assert!(res.is_err());
    }

    // CAD-12: import_attachment security surface.

    #[test]
    fn import_attachment_collision_uses_counter() {
        let (v, dir) = temp_vault();
        let src = dir.path().join("src_file.png");
        fs::write(&src, b"img-bytes").unwrap();
        let names: Vec<String> = (0..3).map(|_| v.import_attachment(&src).unwrap()).collect();
        assert_eq!(names[0], "src_file.png");
        assert!(names[1].starts_with("src_file_") && names[1].ends_with(".png"));
        assert!(names[2].starts_with("src_file_") && names[2].ends_with(".png"));
        for n in &names {
            assert!(v.root.join("_attachments").join(n).exists());
        }
    }

    #[test]
    fn import_attachment_writes_into_attachments_dir() {
        let (v, dir) = temp_vault();
        let src = dir.path().join("legit.png");
        fs::write(&src, b"x").unwrap();
        let dest_name = v.import_attachment(&src).unwrap();
        assert_eq!(dest_name, "legit.png");
        assert!(v.root.join("_attachments/legit.png").exists());
    }

    #[test]
    fn import_attachment_when_src_missing_errors() {
        let (v, dir) = temp_vault();
        let src = dir.path().join("ghost.bin");
        let res = v.import_attachment(&src);
        assert!(res.is_err());
    }

    #[test]
    fn import_attachment_arbitrary_extension_allowed() {
        // Documents the gap (Q-07): import_attachment has no extension allow-list.
        let (v, dir) = temp_vault();
        let src = dir.path().join("payload.exe");
        fs::write(&src, b"MZ").unwrap();
        let name = v.import_attachment(&src).unwrap();
        assert_eq!(name, "payload.exe");
    }

    // CAD-12: parse_frontmatter robustness (panic safety).

    #[test]
    fn parse_frontmatter_no_delimiter_returns_default_and_full_body() {
        let raw = "no frontmatter here\nat all";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, Frontmatter::default().id);
        assert_eq!(body, raw);
    }

    #[test]
    fn parse_frontmatter_unterminated_returns_default() {
        let raw = "---\nid: abc\ntitle: foo\n\nbody never delimited";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "");
        assert_eq!(body, raw);
    }

    #[test]
    fn parse_frontmatter_garbage_yaml_falls_back_without_panic() {
        let raw = "---\n@@@ }: : :\n---\n\nbody";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "");
        assert_eq!(body.trim(), "body");
    }

    #[test]
    fn parse_frontmatter_extra_unknown_field_is_ignored() {
        let raw = "---\nid: ok\naliases: [other-name]\ncustom_field: 42\n---\n\nbody";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "ok");
        assert_eq!(body.trim(), "body");
    }

    #[test]
    fn parse_frontmatter_deep_nesting_does_not_panic() {
        let mut yaml = String::from("---\nid: deep\nnested: ");
        let depth = 50;
        yaml.push_str(&"[".repeat(depth));
        yaml.push_str("\"x\"");
        yaml.push_str(&"]".repeat(depth));
        yaml.push_str("\n---\n\nbody");
        let (fm, _body) = parse_frontmatter(&yaml);
        assert!(fm.id == "deep" || fm.id.is_empty());
    }

    #[test]
    fn parse_frontmatter_at_end_of_file_yields_empty_body() {
        let raw = "---\nid: tail\n---\n";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "tail");
        assert!(body.is_empty());
    }

    #[test]
    fn parse_frontmatter_obsidian_compat_inline_tags_array() {
        let raw = "---\nid: obs\ntags: [\"rust\", \"egui\"]\n---\n\nbody";
        let (fm, _body) = parse_frontmatter(raw);
        assert_eq!(fm.id, "obs");
        assert_eq!(fm.tags, vec!["rust", "egui"]);
    }

    // CAD-12: reload_notes filters .omninote/ and _attachments/.

    #[test]
    fn reload_notes_skips_internal_dirs() {
        let (mut v, _d) = temp_vault();
        fs::write(v.root.join(".omninote/note_inside.md"), "---\nid: x\n---\nbody").unwrap();
        fs::write(v.root.join("_attachments/note_inside.md"), "---\nid: y\n---\nbody").unwrap();
        v.create_note(None, "Real", NoteType::Resumo).unwrap();
        v.reload_notes();
        assert_eq!(v.notes.len(), 1);
        assert_eq!(v.notes[0].title, "Real");
    }

    #[test]
    fn reload_notes_ignores_non_md_files() {
        let (mut v, _d) = temp_vault();
        fs::write(v.root.join("data.json"), "{}").unwrap();
        fs::write(v.root.join("readme.txt"), "ignore me").unwrap();
        v.reload_notes();
        assert_eq!(v.notes.len(), 0);
    }

    // CAD-12: save_config + AppConfig serde roundtrip via vault.

    #[test]
    fn save_config_round_trip() {
        let (mut v, _d) = temp_vault();
        v.config.dark_mode = true;
        v.config.font_size = 17.5;
        v.config.last_active = Some("note-id".to_string());
        v.save_config().unwrap();
        let reopened = Vault::open(v.root.clone()).unwrap();
        assert!(reopened.config.dark_mode);
        assert!((reopened.config.font_size - 17.5).abs() < f32::EPSILON);
        assert_eq!(reopened.config.last_active.as_deref(), Some("note-id"));
    }

    // CAD-12: list_folders excludes internal dirs.

    #[test]
    fn list_folders_excludes_omninote_and_attachments() {
        let (mut v, _d) = temp_vault();
        v.create_folder(None, "Visible").unwrap();
        let folders: Vec<String> = v
            .list_folders()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(folders.iter().any(|f| f.contains("Visible")));
        assert!(!folders.iter().any(|f| f.starts_with(".omninote")));
        assert!(!folders.iter().any(|f| f.starts_with("_attachments")));
    }

    // CAD-12: sanitize_filename_pub matches private impl.

    #[test]
    fn sanitize_filename_pub_matches_private() {
        for input in ["foo/bar", "x*y?z", "  trim  ", "ok_name"] {
            assert_eq!(sanitize_filename_pub(input), sanitize_filename(input));
        }
    }
}
