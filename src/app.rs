use eframe::egui;
use egui::RichText;
use std::path::PathBuf;
use crate::vault::Vault;
use crate::types::{ConfirmAction, Note, NoteType};

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
}

impl OmniNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let last_vault = dirs::config_dir()
            .map(|d| d.join("omninote").join("last_vault"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(PathBuf::from)
            .filter(|p| p.exists());

        let vault = last_vault.and_then(|p| Vault::open(p).ok());
        if let Some(v) = &vault {
            cc.egui_ctx.set_visuals(if v.config.dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
        }

        Self {
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
        }
    }

    pub fn save_last_vault(&self) {
        if let Some(v) = &self.vault {
            if let Some(d) = dirs::config_dir() {
                let dir = d.join("omninote");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(
                    dir.join("last_vault"),
                    v.root.to_string_lossy().as_bytes(),
                );
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
                    self.vault = Some(v);
                    self.active_note = None;
                    self.save_last_vault();
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

        if let Some(v) = &mut self.vault {
            let desired = format!(
                "{}.md",
                crate::vault::sanitize_filename_pub(&note.title)
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
            false
        }
    }
}

impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use std::time::Duration;

        if self.dirty && self.last_save.elapsed() > Duration::from_millis(600) {
            self.flush_active();
        }

        let (new, toggle_edit, settings, toggle_dark) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::N) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::E) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::Comma) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::D) && i.modifiers.ctrl && i.modifiers.shift,
            )
        });
        if new { self.show_new = true; }
        if toggle_edit && self.active_note.is_some() { self.editing = !self.editing; }
        if settings { self.show_settings = true; }
        if toggle_dark {
            if let Some(v) = &mut self.vault {
                v.config.dark_mode = !v.config.dark_mode;
                ctx.set_visuals(if v.config.dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
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
            egui::Window::new("Erro").collapsible(false).show(ctx, |ui| {
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
