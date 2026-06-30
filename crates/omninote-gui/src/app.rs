// `OmniNoteApp` — all application state + the per-frame `update()` loop.
//
// State model (inherited from the engine's design): `active_note` is an OWNED
// clone of the note being edited. `flush_active()` saves it to disk and syncs it
// back into `vault.notes`, sidestepping the &mut vault / &mut note borrow clash.
// Never hold an index into `vault.notes` across a mutation — ids are stable, indices are not.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui::{Context, Key, Modifiers};
use egui_commonmark::CommonMarkCache;
use omninote_core::types::{AppConfig, ConfirmAction, Note, NoteType, ThemePreset};
use omninote_core::vault::Vault;
use omninote_core::vaults;

use crate::theme::Theme;
use crate::watcher::VaultWatcher;

pub struct OmniNoteApp {
    pub vault: Option<Vault>,
    pub active_id: Option<String>,
    pub active_note: Option<Note>,
    pub editing: bool,
    pub dirty: bool,
    pub last_edit: f64,
    pub md_cache: CommonMarkCache,

    // sidebar
    pub search_query: String,
    pub type_filter: Option<NoteType>,
    pub collapsed: HashSet<PathBuf>,

    // overlays / modals
    pub search_open: bool,
    pub palette_query: String,
    pub show_new: bool,
    pub new_title: String,
    pub new_type: NoteType,
    pub new_folder: Option<PathBuf>,
    pub show_settings: bool,
    pub show_import: bool,
    pub confirm: Option<ConfirmAction>,

    // editable config mirror (settings edits this, then flush -> vault.config)
    pub cfg: AppConfig,

    // fs watcher
    pub watcher: Option<VaultWatcher>,
    pub external_changed: bool,

    // chrome
    pub status: String,
    pub toast: Option<(String, f64)>,
    pub tutorial_step: u8,
}

impl OmniNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            vault: None,
            active_id: None,
            active_note: None,
            editing: true,
            dirty: false,
            last_edit: 0.0,
            md_cache: CommonMarkCache::default(),
            search_query: String::new(),
            type_filter: None,
            collapsed: HashSet::new(),
            search_open: false,
            palette_query: String::new(),
            show_new: false,
            new_title: String::new(),
            new_type: NoteType::default(),
            new_folder: None,
            show_settings: false,
            show_import: false,
            confirm: None,
            cfg: AppConfig::default(),
            watcher: None,
            external_changed: false,
            status: String::new(),
            toast: None,
            tutorial_step: 0,
        };
        egui_extras::install_image_loaders(&cc.egui_ctx);
        if let Some(p) = vaults::last_vault_path() {
            app.open_vault(p, &cc.egui_ctx);
        } else {
            // First run with no remembered vault: show the welcome popup (≤5 total).
            app.tutorial_step = 1;
        }
        app.apply_theme(&cc.egui_ctx);
        app
    }

    pub fn apply_theme(&self, ctx: &Context) {
        Theme::from_config(&self.cfg).apply(ctx, &self.cfg);
    }

    pub fn open_vault(&mut self, root: PathBuf, ctx: &Context) {
        match Vault::open(root.clone()) {
            Ok(v) => {
                self.cfg = v.config.clone();
                self.active_id = None;
                self.active_note = None;
                self.dirty = false;
                self.watcher = VaultWatcher::spawn(&root);
                self.external_changed = false;
                self.status = format!("Vault · {}", root.display());
                self.vault = Some(v);
                self.apply_theme(ctx);
            }
            Err(e) => self.status = format!("Erro ao abrir vault: {e}"),
        }
    }

    pub fn pick_vault(&mut self, ctx: &Context) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.open_vault(dir, ctx);
            self.tutorial_step = 0;
        }
    }

    /// Switch the active note, flushing any pending edits on the old one first.
    pub fn select_note(&mut self, id: &str) {
        self.flush_active();
        let note = self
            .vault
            .as_ref()
            .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == id).cloned());
        if let Some(n) = note {
            self.active_note = Some(n);
            self.active_id = Some(id.to_string());
            self.dirty = false;
        }
    }

    pub fn select_note_by_rel(&mut self, rel: &Path) {
        let id = self.vault.as_ref().and_then(|v| {
            v.notes
                .iter()
                .find(|n| n.rel_path == rel)
                .map(|n| n.frontmatter.id.clone())
        });
        if let Some(id) = id {
            self.select_note(&id);
        }
    }

    /// Create a new note carrying imported markdown, select it in read mode.
    pub fn import_as_note(&mut self, title: &str, body: String) {
        let created = match self.vault.as_mut() {
            Some(v) => v.create_note(None, title, NoteType::Resumo),
            None => return,
        };
        match created {
            Ok(mut note) => {
                note.content = body;
                let id = note.frontmatter.id.clone();
                if let Some(v) = self.vault.as_mut() {
                    let _ = v.save_note(&note);
                    if let Some(slot) = v
                        .notes
                        .iter_mut()
                        .find(|n| n.frontmatter.id == note.frontmatter.id)
                    {
                        slot.content = note.content.clone();
                    }
                }
                self.select_note(&id);
                self.editing = false;
                self.status = "Importado".to_string();
            }
            Err(e) => self.status = format!("Erro ao importar: {e}"),
        }
    }

    pub fn close_active(&mut self) {
        self.flush_active();
        self.active_note = None;
        self.active_id = None;
    }

    /// Persist the active note to disk (only when dirty) and sync it back into the
    /// vault's in-memory list + link index.
    pub fn flush_active(&mut self) {
        if !self.dirty {
            return;
        }
        if let (Some(note), Some(v)) = (self.active_note.take(), self.vault.as_mut()) {
            if v.save_note(&note).is_ok() {
                v.reload_notes(); // refresh content + rebuild wikilink/backlink index
            }
            self.active_note = Some(note);
            self.dirty = false;
        }
    }

    /// Re-read the vault from disk after an external change, keeping the active
    /// note selected if it still exists.
    pub fn reload_vault(&mut self) {
        let id = self.active_id.clone();
        if let Some(v) = self.vault.as_mut() {
            v.reload_notes();
        }
        self.external_changed = false;
        if let Some(id) = id {
            self.active_note = self
                .vault
                .as_ref()
                .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == id).cloned());
        }
    }

    pub fn persist_config(&mut self) {
        if let Some(v) = self.vault.as_mut() {
            v.config = self.cfg.clone();
            let _ = v.save_config();
        }
    }

    /// Move a note to a different folder (drag-and-drop). `dest = None` is the root.
    pub fn move_note(&mut self, id: &str, dest: Option<PathBuf>) {
        let res = match self.vault.as_mut() {
            Some(v) => v.move_note_by_id(id, dest.as_deref()),
            None => return,
        };
        match res {
            Ok(()) => {
                if let Some(v) = self.vault.as_mut() {
                    v.reload_notes();
                }
                if self.active_id.as_deref() == Some(id) {
                    let want = id.to_string();
                    self.active_note = self
                        .vault
                        .as_ref()
                        .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == want).cloned());
                }
                self.status = "Nota movida".to_string();
            }
            Err(e) => self.status = format!("Erro ao mover: {e}"),
        }
    }

    pub fn toggle_dark(&mut self, ctx: &Context) {
        self.cfg.dark_mode = !self.cfg.dark_mode;
        self.cfg.theme_preset = if self.cfg.dark_mode {
            ThemePreset::ObsidianDark
        } else {
            ThemePreset::ObsidianLight
        };
        self.apply_theme(ctx);
        self.persist_config();
        let msg = if self.cfg.dark_mode {
            "Tema · noite"
        } else {
            "Tema · pergaminho"
        };
        self.toast(ctx, msg);
    }

    pub fn toast(&mut self, ctx: &Context, msg: impl Into<String>) {
        let now = ctx.input(|i| i.time);
        self.toast = Some((msg.into(), now + 2.5));
    }

    fn close_overlays(&mut self) {
        self.search_open = false;
        self.show_new = false;
        self.show_settings = false;
        self.show_import = false;
        self.confirm = None;
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        if ctx.input_mut(|i| i.consume_key(cmd, Key::N)) {
            self.show_new = true;
            self.new_title.clear();
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::E)) {
            self.editing = !self.editing;
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::K)) {
            self.search_open = !self.search_open;
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::Comma)) {
            self.show_settings = true;
        }
        if ctx.input_mut(|i| i.consume_key(cmd_shift, Key::D)) {
            self.toggle_dark(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, Key::W)) {
            self.close_active();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.close_overlays();
        }
    }

    fn maybe_autosave(&mut self, ctx: &Context) {
        if self.dirty {
            let now = ctx.input(|i| i.time);
            if now - self.last_edit > 0.8 {
                self.flush_active();
            }
        }
    }

    fn poll_watcher(&mut self) {
        if let Some(w) = &self.watcher {
            if w.drain() {
                self.external_changed = true;
            }
        }
    }

    fn show_toast(&mut self, ctx: &Context) {
        if let Some((msg, until)) = self.toast.clone() {
            let now = ctx.input(|i| i.time);
            if now > until {
                self.toast = None;
                return;
            }
            egui::Area::new(egui::Id::new("almanac_toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(msg);
                    });
                });
            ctx.request_repaint();
        }
    }
}

impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.maybe_autosave(ctx);
        self.poll_watcher();

        // Panel order matters: top/bottom reserve space, then side, then central.
        self.show_toolbar(ctx);
        self.show_statusbar(ctx);
        self.show_sidebar(ctx);
        self.show_connections(ctx);
        self.show_editor(ctx);
        self.show_modals(ctx);
        self.show_toast(ctx);
    }
}

#[cfg(test)]
impl OmniNoteApp {
    /// Build an app around an already-open vault, no egui context required.
    /// Mirrors `new()` minus the UI bootstrap so state logic can be unit-tested.
    fn test_app(vault: Vault) -> Self {
        Self {
            vault: Some(vault),
            active_id: None,
            active_note: None,
            editing: true,
            dirty: false,
            last_edit: 0.0,
            md_cache: CommonMarkCache::default(),
            search_query: String::new(),
            type_filter: None,
            collapsed: HashSet::new(),
            search_open: false,
            palette_query: String::new(),
            show_new: false,
            new_title: String::new(),
            new_type: NoteType::default(),
            new_folder: None,
            show_settings: false,
            show_import: false,
            confirm: None,
            cfg: AppConfig::default(),
            watcher: None,
            external_changed: false,
            status: String::new(),
            toast: None,
            tutorial_step: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn app_with_vault() -> (OmniNoteApp, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path().to_path_buf()).unwrap();
        (OmniNoteApp::test_app(vault), dir)
    }

    fn new_note(app: &mut OmniNoteApp, title: &str) -> String {
        app.vault
            .as_mut()
            .unwrap()
            .create_note(None, title, NoteType::Resumo)
            .unwrap()
            .frontmatter
            .id
    }

    #[test]
    fn create_and_select_note() {
        let (mut app, _d) = app_with_vault();
        let id = new_note(&mut app, "Olá");
        app.select_note(&id);
        assert_eq!(app.active_id.as_deref(), Some(id.as_str()));
        assert_eq!(app.active_note.as_ref().unwrap().title, "Olá");
    }

    #[test]
    fn edit_then_flush_persists_to_disk() {
        let (mut app, _d) = app_with_vault();
        let id = new_note(&mut app, "Nota");
        app.select_note(&id);
        app.active_note.as_mut().unwrap().content = "corpo novo".to_string();
        app.dirty = true;
        app.flush_active();
        assert!(!app.dirty, "flush should clear the dirty flag");

        let root = app.vault.as_ref().unwrap().root.clone();
        let reopened = Vault::open(root).unwrap();
        let n = reopened
            .notes
            .iter()
            .find(|n| n.frontmatter.id == id)
            .unwrap();
        assert!(n.content.contains("corpo novo"));
    }

    #[test]
    fn move_note_changes_folder() {
        let (mut app, _d) = app_with_vault();
        app.vault
            .as_mut()
            .unwrap()
            .create_folder(None, "Arquivo")
            .unwrap();
        let id = new_note(&mut app, "Movível");
        app.move_note(&id, Some(PathBuf::from("Arquivo")));
        let moved = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.frontmatter.id == id)
            .unwrap();
        assert!(
            moved.rel_path.starts_with("Arquivo"),
            "got {:?}",
            moved.rel_path
        );
    }

    #[test]
    fn import_creates_and_selects_note() {
        let (mut app, _d) = app_with_vault();
        app.import_as_note("Importado", "# Conteúdo".to_string());
        let active = app
            .active_note
            .as_ref()
            .expect("import should select the note");
        assert!(active.content.contains("Conteúdo"));
        assert!(!app.editing, "imports open in read mode");
    }

    #[test]
    fn select_by_rel_path_round_trips() {
        let (mut app, _d) = app_with_vault();
        let id = new_note(&mut app, "PorCaminho");
        let rel = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.frontmatter.id == id)
            .unwrap()
            .rel_path
            .clone();
        app.select_note_by_rel(&rel);
        assert_eq!(app.active_id.as_deref(), Some(id.as_str()));
    }
}
