use crate::theme;
use crate::watcher::VaultWatcher;
use eframe::egui;
use egui::RichText;
use omninote_core::types::{ConfirmAction, Note, NoteType};
use omninote_core::vault::Vault;
use std::path::PathBuf;

/// OpenDyslexic, bundled in the binary (OFL). Selectable via the a11y font setting.
const OPEN_DYSLEXIC_OTF: &[u8] = include_bytes!("../assets/fonts/OpenDyslexic-Regular.otf");

/// Register bundled custom fonts. Call once before applying theme/styles.
fn register_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        theme::OPEN_DYSLEXIC_NAME.to_owned(),
        egui::FontData::from_static(OPEN_DYSLEXIC_OTF),
    );
    fonts
        .families
        .entry(egui::FontFamily::Name(theme::OPEN_DYSLEXIC_NAME.into()))
        .or_default()
        .insert(0, theme::OPEN_DYSLEXIC_NAME.to_owned());
    ctx.set_fonts(fonts);
}

pub struct OmniNoteApp {
    pub vault: Option<Vault>,
    pub active_note: Option<Note>,
    pub editing: bool,
    pub query: String,
    pub type_filter: Option<NoteType>,
    pub show_settings: bool,
    pub show_new: bool,
    pub show_import: bool,
    pub confirm_action: Option<ConfirmAction>,
    pub dirty: bool,
    pub last_save: std::time::Instant,
    pub error_msg: Option<String>,
    pub md_cache: egui_commonmark::CommonMarkCache,
    pub watcher: Option<VaultWatcher>,
    /// Self-write window: events arriving before this instant are ignored
    /// (they came from our own save).
    pub self_write_until: std::time::Instant,
    /// Set when external change detected on the active note while dirty=true.
    /// Triggers a conflict modal asking user to keep edits or reload.
    pub external_change_pending: bool,
    /// v0.8 — index of the `/` that opened the slash menu, in note.content.
    /// None when menu is closed. Set when `/` is typed at start of line.
    pub slash_menu_pos: Option<usize>,
}

impl OmniNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let last_vault = dirs::config_dir()
            .map(|d| d.join("omninote").join("last_vault"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(PathBuf::from)
            .filter(|p| p.exists());

        let vault = last_vault.and_then(|p| Vault::open(p).ok());
        register_custom_fonts(&cc.egui_ctx);
        let dark = vault.as_ref().map(|v| v.config.dark_mode).unwrap_or(true);
        theme::apply_theme(&cc.egui_ctx, dark);

        let watcher = vault.as_ref().and_then(|v| VaultWatcher::new(&v.root).ok());

        let app = Self {
            vault,
            active_note: None,
            editing: false,
            query: String::new(),
            type_filter: None,
            show_settings: false,
            show_new: false,
            show_import: false,
            confirm_action: None,
            dirty: false,
            last_save: std::time::Instant::now(),
            error_msg: None,
            md_cache: egui_commonmark::CommonMarkCache::default(),
            watcher,
            self_write_until: std::time::Instant::now(),
            external_change_pending: false,
            slash_menu_pos: None,
        };
        app.apply_style(&cc.egui_ctx);
        app
    }

    pub fn save_last_vault(&self) {
        if let Some(v) = &self.vault {
            if let Some(d) = dirs::config_dir() {
                let dir = d.join("omninote");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("last_vault"), v.root.to_string_lossy().as_bytes());
            }
        }
    }

    /// Apply font/spacing settings from vault config to egui style.
    /// Call after vault open and after settings changes.
    pub fn apply_style(&self, ctx: &egui::Context) {
        let v = match &self.vault {
            Some(v) => v,
            None => return,
        };
        let cfg = &v.config;
        let mut style = (*ctx.style()).clone();

        // Font sizes — scale all text styles relative to base font_size
        let base = cfg.font_size;
        let scale = base / 14.0; // 14pt is egui default base
        for (text_style, font_id) in style.text_styles.iter_mut() {
            let default_size = match text_style {
                egui::TextStyle::Heading => 20.0,
                egui::TextStyle::Body => 14.0,
                egui::TextStyle::Monospace => 12.0,
                egui::TextStyle::Button => 14.0,
                egui::TextStyle::Small => 10.0,
                _ => 14.0,
            };
            font_id.size = (default_size * scale).round();
            font_id.family = crate::theme::font_family_to_egui(cfg.font_family);
        }

        // Line spacing via item_spacing
        let extra = (base * (cfg.line_height - 1.0)).max(0.0);
        style.spacing.item_spacing.y = 3.0 + extra;

        ctx.set_style(style);
    }

    pub fn pick_vault_with_ctx(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Escolha (ou crie) uma pasta pra ser seu vault")
            .pick_folder()
        {
            match Vault::open(path) {
                Ok(v) => {
                    let root = v.root.clone();
                    self.vault = Some(v);
                    self.active_note = None;
                    self.save_last_vault();
                    self.apply_style(ctx);
                    self.watcher = VaultWatcher::new(&root).ok();
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }

    pub fn pick_vault(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Escolha (ou crie) uma pasta pra ser seu vault")
            .pick_folder()
        {
            match Vault::open(path) {
                Ok(v) => {
                    let root = v.root.clone();
                    self.vault = Some(v);
                    self.active_note = None;
                    self.save_last_vault();
                    self.watcher = VaultWatcher::new(&root).ok();
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }

    pub fn flush_active(&mut self) {
        if !self.dirty {
            return;
        }
        let note = match self.active_note.take() {
            Some(n) => n,
            None => {
                self.dirty = false;
                return;
            }
        };

        // Set self-write window so notify events from our save are ignored
        self.self_write_until = std::time::Instant::now() + std::time::Duration::from_millis(400);

        if let Some(v) = &mut self.vault {
            let desired = format!(
                "{}.md",
                omninote_core::vault::sanitize_filename_pub(&note.title)
            );
            let current = note
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let final_note = if desired != current {
                match v.rename_note_by_id(&note.frontmatter.id, &note.title) {
                    Ok(renamed) => {
                        let mut n = renamed;
                        n.frontmatter = note.frontmatter.clone();
                        n.content = note.content.clone();
                        let _ = v.save_note(&n);
                        n
                    }
                    Err(_) => {
                        let _ = v.save_note(&note);
                        note
                    }
                }
            } else {
                let _ = v.save_note(&note);
                note
            };

            if let Some(existing) = v
                .notes
                .iter_mut()
                .find(|n| n.frontmatter.id == final_note.frontmatter.id)
            {
                *existing = final_note.clone();
            }
            self.active_note = Some(final_note);
        } else {
            self.active_note = Some(note);
        }

        self.dirty = false;
        self.last_save = std::time::Instant::now();
    }

    pub fn select_note(&mut self, id: &str) {
        self.flush_active();
        if let Some(v) = &self.vault {
            if let Some(note) = v.notes.iter().find(|n| n.frontmatter.id == id) {
                self.active_note = Some(note.clone());
                self.editing = false;
                self.dirty = false;
            }
        }
    }

    /// Find first note matching `title` (case-insensitive) and select it.
    /// Returns true if found, false otherwise.
    pub fn select_note_by_title(&mut self, title: &str) -> bool {
        let target_id = self.vault.as_ref().and_then(|v| {
            v.notes
                .iter()
                .find(|n| n.title.eq_ignore_ascii_case(title))
                .map(|n| n.frontmatter.id.clone())
        });
        if let Some(id) = target_id {
            self.select_note(&id);
            true
        } else {
            // CAD-20: fall back to wikilink resolver (handles aliases, paths,
            // case-insensitive matching). `title` here may actually be a full
            // wikilink target like "folder/Note" or a frontmatter alias.
            self.select_note_by_target(title)
        }
    }

    /// Resolve a wikilink target through the [`omninote_core::resolver::VaultIndex`]
    /// (filename / path / alias / case-insensitive) and select the resulting
    /// note. Returns true if resolved + selected, false if unresolved. CAD-20.
    pub fn select_note_by_target(&mut self, target: &str) -> bool {
        let rel_path = match self.vault.as_ref().and_then(|v| v.index.resolve(target)) {
            Some(p) => p.clone(),
            None => return false,
        };
        let id = self.vault.as_ref().and_then(|v| {
            v.notes
                .iter()
                .find(|n| n.rel_path == rel_path)
                .map(|n| n.frontmatter.id.clone())
        });
        if let Some(id) = id {
            self.select_note(&id);
            true
        } else {
            false
        }
    }

    /// Drain any pending filesystem events from the watcher and reload as needed.
    /// Filters out events arriving during the self-write window.
    pub fn poll_watcher(&mut self) {
        let watcher = match &self.watcher {
            Some(w) => w,
            None => return,
        };
        let changes = watcher.drain_md_changes();
        if changes.is_empty() {
            return;
        }
        // If we just saved, ignore — these are our own writes echoing back
        if std::time::Instant::now() < self.self_write_until {
            return;
        }
        // Did the active note's file change?
        let active_path = self.active_note.as_ref().map(|n| n.path.clone());
        let active_changed = match &active_path {
            Some(ap) => changes.iter().any(|p| p == ap),
            None => false,
        };

        if active_changed && self.dirty {
            // Conflict: external change + we have unsaved edits. Ask user.
            self.external_change_pending = true;
            return;
        }

        // Safe to reload silently
        if let Some(v) = &mut self.vault {
            v.reload_notes();
            // If active note was renamed/deleted externally, drop it
            if let Some(ap) = active_path {
                let still_exists = v.notes.iter().any(|n| n.path == ap);
                if !still_exists {
                    self.active_note = None;
                } else if active_changed && !self.dirty {
                    // Refresh active_note content from disk
                    if let Some(fresh) = v.notes.iter().find(|n| n.path == ap).cloned() {
                        self.active_note = Some(fresh);
                    }
                }
            }
        }
    }
}

impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use std::time::Duration;

        if self.dirty && self.last_save.elapsed() > Duration::from_millis(600) {
            self.flush_active();
        }

        // Watcher poll (v0.6 — CAD-9)
        self.poll_watcher();
        // Request repaint regularly so watcher events are noticed even when idle
        ctx.request_repaint_after(Duration::from_millis(500));

        // `consume_shortcut` maps COMMAND to Cmd on macOS and Ctrl elsewhere, and
        // consumes the event so a focused TextEdit doesn't also intercept it.
        let new_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::N);
        let edit_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::E);
        let settings_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Comma);
        let dark_sc = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::D,
        );

        let (new, toggle_edit, settings, toggle_dark) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&new_sc),
                i.consume_shortcut(&edit_sc),
                i.consume_shortcut(&settings_sc),
                i.consume_shortcut(&dark_sc),
            )
        });
        if new {
            self.show_new = true;
        }
        if toggle_edit && self.active_note.is_some() {
            self.editing = !self.editing;
        }
        if settings {
            self.show_settings = true;
        }
        if toggle_dark {
            if let Some(v) = &mut self.vault {
                v.config.dark_mode = !v.config.dark_mode;
                let dark = v.config.dark_mode;
                theme::apply_theme(ctx, dark);
            }
        }

        if self.vault.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("📓 OmniNote");
                    ui.add_space(8.0);
                    ui.label("Escolha uma pasta pra ser seu vault.");
                    ui.label(
                        RichText::new("Compatível com Obsidian e Claude Desktop.")
                            .size(11.0)
                            .weak(),
                    );
                    ui.add_space(20.0);
                    if ui.button("📂 Abrir / Criar Vault").clicked() {
                        self.pick_vault();
                    }
                });
            });
            return;
        }

        self.show_sidebar(ctx);
        self.show_editor(ctx);
        self.show_modals(ctx);

        if let Some(err) = self.error_msg.clone() {
            egui::Window::new("Erro")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&err);
                    if ui.button("OK").clicked() {
                        self.error_msg = None;
                    }
                });
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_active();
        if let Some(v) = &self.vault {
            let _ = v.save_config();
        }
    }
}
