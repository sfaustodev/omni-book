//! Modal windows: new note, settings (theme + accessibility), import (PDF / Claude
//! chat / artifact), new folder, delete confirmation, the external-change conflict
//! prompt, and the single first-run onboarding card.

use omninote_core::resolver::VaultIndex;
use omninote_core::types::{FontFamily, NoteType};

use crate::app::{OmniNoteApp, Pending};
use crate::theme::Theme;

impl OmniNoteApp {
    pub fn show_modals(&mut self, ctx: &egui::Context) {
        self.modal_new_note(ctx);
        self.modal_new_folder(ctx);
        self.modal_settings(ctx);
        self.modal_import(ctx);
        self.modal_confirm(ctx);
        self.modal_conflict(ctx);
    }

    fn modal_new_note(&mut self, ctx: &egui::Context) {
        if !self.show_new {
            return;
        }
        let theme = self.theme;
        let mut open = true;
        let mut create = false;
        egui::Window::new("Nova nota")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_title)
                        .hint_text("título")
                        .desired_width(280.0),
                );
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Tipo").small().color(theme.dim));
                ui.horizontal_wrapped(|ui| {
                    for ty in NoteType::all() {
                        let on = self.new_type == ty;
                        let txt = egui::RichText::new(format!("{} {}", ty.icon(), ty.label()))
                            .color(if on {
                                theme.note_type_color(ty)
                            } else {
                                theme.dim
                            });
                        if ui.selectable_label(on, txt).clicked() {
                            self.new_type = ty;
                        }
                    }
                });
                ui.add_space(8.0);
                if ui.button("Criar nota").clicked() {
                    create = true;
                }
            });

        if create {
            let title = self.new_title.clone();
            let ty = self.new_type;
            let folder = self.new_in_folder.clone();
            self.create_note_now(folder, &title, ty);
            self.new_title.clear();
            self.show_new = false;
        } else {
            self.show_new = open;
        }
    }

    fn modal_new_folder(&mut self, ctx: &egui::Context) {
        if !self.show_new_folder {
            return;
        }
        let mut open = true;
        let mut create = false;
        egui::Window::new("Nova pasta")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_folder_name)
                        .hint_text("nome da pasta")
                        .desired_width(260.0),
                );
                if ui.button("Criar pasta").clicked() {
                    create = true;
                }
            });

        if create {
            let name = self.new_folder_name.trim().to_string();
            if !name.is_empty() {
                let r = self.vault.as_mut().map(|v| v.create_folder(None, &name));
                match r {
                    Some(Ok(_)) => self.toast_info("Pasta criada"),
                    Some(Err(e)) => self.toast_error(e),
                    None => {}
                }
            }
            self.show_new_folder = false;
        } else {
            self.show_new_folder = open;
        }
    }

    fn modal_settings(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let theme = self.theme;
        let mut open = true;
        egui::Window::new("Configurações")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .open(&mut open)
            .show(ctx, |ui| {
                let Some(vault) = self.vault.as_mut() else {
                    ui.label("Abra um vault primeiro.");
                    return;
                };
                let cfg = &mut vault.config;

                ui.label(
                    egui::RichText::new("APARÊNCIA")
                        .small()
                        .strong()
                        .color(theme.dim),
                );
                ui.checkbox(&mut cfg.dark_mode, "Tema escuro (Cmd/Ctrl+Shift+D)");
                ui.separator();

                ui.label(
                    egui::RichText::new("ACESSIBILIDADE")
                        .small()
                        .strong()
                        .color(theme.dim),
                );
                egui::ComboBox::from_label("Fonte")
                    .selected_text(cfg.font_family.label())
                    .show_ui(ui, |ui| {
                        for f in FontFamily::all() {
                            ui.selectable_value(&mut cfg.font_family, f, f.label());
                        }
                    });
                ui.add(egui::Slider::new(&mut cfg.font_size, 11.0..=28.0).text("Tamanho"));
                ui.add(egui::Slider::new(&mut cfg.line_height, 1.0..=2.2).text("Altura da linha"));
                ui.add(egui::Slider::new(&mut cfg.letter_spacing, 0.0..=4.0).text("Espaçamento"));
            });

        // Keep the live theme in sync with the dark-mode toggle.
        if let Some(vault) = self.vault.as_ref() {
            self.theme = Theme::from_dark(vault.config.dark_mode);
        }
        // Persist config when the window closes.
        if !open {
            if let Some(vault) = self.vault.as_ref() {
                let _ = vault.save_config();
            }
        }
        self.show_settings = open;
    }

    fn modal_import(&mut self, ctx: &egui::Context) {
        if !self.show_import {
            return;
        }
        let theme = self.theme;
        let mut open = true;
        let mut action: Option<u8> = None;
        egui::Window::new("Importar")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Importar como nova nota:").color(theme.dim));
                ui.add_space(6.0);
                if ui.button("📄  PDF — extrair texto").clicked() {
                    action = Some(0);
                }
                if ui.button("💬  Chat do Claude (.json)").clicked() {
                    action = Some(1);
                }
                if ui.button("🧩  Artefato de código").clicked() {
                    action = Some(2);
                }
            });
        self.show_import = open;

        match action {
            Some(0) => self.import_pdf(),
            Some(1) => self.import_chat(),
            Some(2) => self.import_artifact(),
            _ => {}
        }
    }

    fn import_pdf(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_file()
        else {
            return;
        };
        let title = stem(&path);
        match omninote_core::pdf::extract_text(&path) {
            Ok(text) => self.create_with_content(&title, NoteType::Resumo, text),
            Err(e) => self.toast_error(format!("PDF: {e}")),
        }
    }

    fn import_chat(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON/Markdown", &["json", "md"])
            .pick_file()
        else {
            return;
        };
        let title = stem(&path);
        match omninote_core::import::import_claude_chat(&path) {
            Ok(md) => self.create_with_content(&title, NoteType::Resumo, md),
            Err(e) => self.toast_error(format!("Chat: {e}")),
        }
    }

    fn import_artifact(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            return;
        };
        let title = stem(&path);
        match omninote_core::import::import_claude_artifact(&path) {
            Ok(md) => self.create_with_content(&title, NoteType::Codigo, md),
            Err(e) => self.toast_error(format!("Artefato: {e}")),
        }
    }

    /// Create a note pre-filled with imported content and open it.
    fn create_with_content(&mut self, title: &str, ty: NoteType, content: String) {
        self.flush();
        let created = match self.vault.as_mut() {
            Some(v) => v.create_note(None, title, ty),
            None => return,
        };
        let mut note = match created {
            Ok(n) => n,
            Err(e) => {
                self.toast_error(format!("Erro: {e}"));
                return;
            }
        };
        note.content = content;
        if let Some(v) = self.vault.as_mut() {
            if let Err(e) = v.save_note(&note) {
                self.toast_error(format!("Erro: {e}"));
                return;
            }
            if let Some(slot) = v
                .notes
                .iter_mut()
                .find(|n| n.frontmatter.id == note.frontmatter.id)
            {
                *slot = note.clone();
            }
            v.index = VaultIndex::build(&v.notes);
        }
        self.active_id = Some(note.frontmatter.id.clone());
        self.active_note = Some(note);
        self.editing = false;
        self.show_import = false;
        self.onboarding_done = true;
        self.toast_info("Importado");
    }

    fn modal_confirm(&mut self, ctx: &egui::Context) {
        let Some(Pending::DeleteNote(id)) = self.pending.as_ref() else {
            return;
        };
        let id = id.clone();
        let theme = self.theme;
        let mut decision: Option<bool> = None;
        egui::Window::new("Apagar nota?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Esta ação não pode ser desfeita.").color(theme.dim));
                ui.horizontal(|ui| {
                    if ui.button("Cancelar").clicked() {
                        decision = Some(false);
                    }
                    if ui
                        .button(egui::RichText::new("Apagar").color(theme.accent))
                        .clicked()
                    {
                        decision = Some(true);
                    }
                });
            });
        match decision {
            Some(true) => {
                self.delete_note_now(&id);
                self.pending = None;
            }
            Some(false) => self.pending = None,
            None => {}
        }
    }

    fn modal_conflict(&mut self, ctx: &egui::Context) {
        if !self.external_change {
            return;
        }
        let theme = self.theme;
        let mut decision: Option<bool> = None;
        egui::Window::new("Arquivo alterado externamente")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("A nota mudou no disco enquanto você editava.")
                        .color(theme.dim),
                );
                ui.horizontal(|ui| {
                    if ui.button("Manter minha edição").clicked() {
                        decision = Some(false);
                    }
                    if ui.button("Recarregar do disco").clicked() {
                        decision = Some(true);
                    }
                });
            });
        match decision {
            Some(true) => {
                self.dirty = false;
                self.reload_and_reselect();
                self.external_change = false;
            }
            Some(false) => self.external_change = false,
            None => {}
        }
    }

    pub fn show_onboarding(&mut self, ctx: &egui::Context) {
        if self.onboarding_done || self.vault.is_none() {
            return;
        }
        let theme = self.theme;
        let mut done = false;
        egui::Window::new("Bem-vindo ao OmniNote")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Um caderno de notas em arquivos .md — compatível com Obsidian.",
                    )
                    .color(theme.dim),
                );
                ui.add_space(8.0);
                for (k, v) in [
                    ("Cmd/Ctrl+N", "nova nota"),
                    ("Cmd/Ctrl+K", "buscar"),
                    ("Cmd/Ctrl+E", "alternar edição/leitura"),
                    ("Cmd/Ctrl+,", "configurações"),
                    ("Cmd/Ctrl+Shift+D", "alternar tema"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(k).monospace().color(theme.accent));
                        ui.label(egui::RichText::new(v).color(theme.text));
                    });
                }
                ui.add_space(10.0);
                if ui.button("Começar").clicked() {
                    done = true;
                }
            });
        if done {
            self.onboarding_done = true;
        }
    }
}

fn stem(path: &std::path::Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Importado".to_string())
}
