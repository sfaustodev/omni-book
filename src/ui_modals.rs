use crate::app::OmniNoteApp;
use crate::types::{ConfirmAction, NoteType};

impl OmniNoteApp {
    pub fn show_modals(&mut self, ctx: &egui::Context) {
        self.show_modal_new(ctx);
        self.show_modal_settings(ctx);
        self.show_modal_confirm(ctx);
        self.show_modal_import(ctx);
        self.show_modal_external_change(ctx);
    }

    /// v0.6 CAD-9 — conflict modal when external edit hits a dirty active note.
    fn show_modal_external_change(&mut self, ctx: &egui::Context) {
        if !self.external_change_pending {
            return;
        }
        egui::Window::new("Mudança externa detectada")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("O arquivo da nota ativa foi modificado fora do OmniNote (Obsidian, MCP do Claude, etc.) e você tem edits não salvas.");
                ui.label(egui::RichText::new("O que fazer?").strong());
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("📥 Recarregar (perde meus edits)").clicked() {
                        if let Some(v) = &mut self.vault {
                            crate::actions::external_change_reload(
                                v,
                                &mut self.active_note,
                                &mut self.dirty,
                                &mut self.external_change_pending,
                            );
                        } else {
                            self.dirty = false;
                            self.external_change_pending = false;
                        }
                    }
                    if ui.button("💾 Manter edits (sobrescreve no próximo save)").clicked() {
                        // Push self-write window forward so next save isn't seen as conflict
                        self.self_write_until = std::time::Instant::now()
                            + std::time::Duration::from_millis(400);
                        crate::actions::external_change_keep(
                            &mut self.dirty,
                            &mut self.external_change_pending,
                        );
                    }
                });
            });
    }

    fn show_modal_new(&mut self, ctx: &egui::Context) {
        if !self.show_new {
            return;
        }
        let mut open = self.show_new;
        egui::Window::new("Nova Nota")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Escolha o tipo:");
                ui.separator();
                egui::Grid::new("note_type_grid")
                    .num_columns(3)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (i, t) in NoteType::all().iter().enumerate() {
                            if ui
                                .add_sized(
                                    [70.0, 60.0],
                                    egui::Button::new(format!("{}\n{}", t.icon(), t.label())),
                                )
                                .clicked()
                            {
                                self.flush_active();
                                if let Some(v) = &mut self.vault {
                                    match v.create_note(None, "", *t) {
                                        Ok(note) => {
                                            self.active_note = Some(note);
                                            self.editing = true;
                                            self.dirty = false;
                                        }
                                        Err(e) => self.error_msg = Some(e),
                                    }
                                }
                                self.show_new = false;
                            }
                            if (i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        self.show_new = open;
    }

    fn show_modal_settings(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let mut open = self.show_settings;
        let mut style_dirty = false;

        egui::Window::new("Configurações")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if let Some(v) = &mut self.vault {
                    // Tema
                    let mut dark = v.config.dark_mode;
                    if ui.checkbox(&mut dark, "🌙 Modo escuro").changed() {
                        v.config.dark_mode = dark;
                        ctx.set_visuals(if dark {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        });
                    }
                    ui.separator();

                    // Acessibilidade (v0.5)
                    ui.label(egui::RichText::new("Acessibilidade").strong());

                    // Font family
                    ui.horizontal(|ui| {
                        ui.label("Fonte:");
                        let current = v.config.font_family;
                        egui::ComboBox::from_id_salt("font_family_combo")
                            .selected_text(current.label())
                            .show_ui(ui, |ui| {
                                for f in crate::types::FontFamily::all() {
                                    if ui
                                        .selectable_label(v.config.font_family == f, f.label())
                                        .clicked()
                                        && v.config.font_family != f
                                    {
                                        v.config.font_family = f;
                                        style_dirty = true;
                                    }
                                }
                            });
                    });

                    // Font size
                    ui.horizontal(|ui| {
                        ui.label("Tamanho:");
                        let prev = v.config.font_size;
                        if ui
                            .add(
                                egui::Slider::new(&mut v.config.font_size, 11.0..=24.0)
                                    .suffix("pt"),
                            )
                            .changed()
                            && (v.config.font_size - prev).abs() > 0.01
                        {
                            style_dirty = true;
                        }
                    });

                    // Line height
                    ui.horizontal(|ui| {
                        ui.label("Espaço entre linhas:");
                        let prev = v.config.line_height;
                        if ui
                            .add(
                                egui::Slider::new(&mut v.config.line_height, 1.0..=2.2)
                                    .fixed_decimals(2),
                            )
                            .changed()
                            && (v.config.line_height - prev).abs() > 0.001
                        {
                            style_dirty = true;
                        }
                    });

                    // Reset button
                    if ui.small_button("↩ Restaurar padrões").clicked() {
                        v.config.font_family = crate::types::FontFamily::default();
                        v.config.font_size = 14.0;
                        v.config.line_height = 1.4;
                        style_dirty = true;
                    }

                    ui.separator();
                    ui.label("Vault atual:");
                    ui.label(
                        egui::RichText::new(v.root.to_string_lossy().as_ref())
                            .size(11.0)
                            .weak(),
                    );
                }
                ui.separator();
                if ui.button("📂 Trocar vault").clicked() {
                    self.pick_vault_with_ctx(ctx);
                    self.show_settings = false;
                }
            });

        if style_dirty {
            self.apply_style(ctx);
        }

        self.show_settings = open;
    }

    fn show_modal_confirm(&mut self, ctx: &egui::Context) {
        let action = match self.confirm_action.clone() {
            Some(a) => a,
            None => return,
        };
        let msg = match &action {
            ConfirmAction::DeleteNote(id) => {
                let title = self
                    .vault
                    .as_ref()
                    .and_then(|v| v.notes.iter().find(|n| &n.frontmatter.id == id))
                    .map(|n| n.title.clone())
                    .unwrap_or_default();
                format!(
                    "Deletar nota \"{}\"?\nEssa ação não pode ser desfeita.",
                    title
                )
            }
            ConfirmAction::DeleteFolder(p) => format!(
                "Deletar pasta \"{}\" e todo seu conteúdo?\nEssa ação não pode ser desfeita.",
                p.to_string_lossy()
            ),
        };

        egui::Window::new("Confirmar")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(&msg);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("🗑 Sim, deletar").clicked() {
                        if let Some(v) = &mut self.vault {
                            match &action {
                                ConfirmAction::DeleteNote(id) => {
                                    let _ = crate::actions::confirm_delete_note(
                                        v,
                                        &mut self.active_note,
                                        id,
                                    );
                                }
                                ConfirmAction::DeleteFolder(p) => {
                                    let _ = crate::actions::confirm_delete_folder(
                                        v,
                                        &mut self.active_note,
                                        p,
                                    );
                                }
                            }
                        }
                        crate::actions::cancel_confirm(&mut self.confirm_action);
                    }
                    if ui.button("Cancelar").clicked() {
                        crate::actions::cancel_confirm(&mut self.confirm_action);
                    }
                });
            });
    }

    fn show_modal_import(&mut self, ctx: &egui::Context) {
        if !self.show_import {
            return;
        }
        let mut open = self.show_import;
        egui::Window::new("Importar")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if ui
                        .add_sized(
                            [220.0, 36.0],
                            egui::Button::new("📄 PDF — extrai texto e cria nota"),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PDF", &["pdf"])
                            .pick_file()
                        {
                            self.import_pdf(&path);
                        }
                        self.show_import = false;
                    }
                    if ui
                        .add_sized([220.0, 36.0], egui::Button::new("🤖 Chat Claude (JSON)"))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            self.import_chat(&path);
                        }
                        self.show_import = false;
                    }
                    if ui
                        .add_sized(
                            [220.0, 36.0],
                            egui::Button::new("📦 Artefato Claude (código/html)"),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.import_artifact(&path);
                        }
                        self.show_import = false;
                    }
                });
            });
        self.show_import = open;
    }

    // Import helpers — thin wrappers around crate::actions for the rfd dialog flow.

    fn import_pdf(&mut self, path: &std::path::Path) {
        if let Some(v) = &mut self.vault {
            crate::actions::import_pdf(
                v,
                &mut self.active_note,
                &mut self.editing,
                &mut self.error_msg,
                path,
            );
        }
    }

    fn import_chat(&mut self, path: &std::path::Path) {
        if let Some(v) = &mut self.vault {
            crate::actions::import_chat(
                v,
                &mut self.active_note,
                &mut self.editing,
                &mut self.error_msg,
                path,
            );
        }
    }

    fn import_artifact(&mut self, path: &std::path::Path) {
        if let Some(v) = &mut self.vault {
            crate::actions::import_artifact(
                v,
                &mut self.active_note,
                &mut self.editing,
                &mut self.error_msg,
                path,
            );
        }
    }
}
