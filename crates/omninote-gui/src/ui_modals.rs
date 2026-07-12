use crate::app::OmniNoteApp;
use omninote_core::types::{ConfirmAction, NoteType};

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
                        self.dirty = false;
                        self.clear_editor_transients();
                        if let Some(v) = &mut self.vault {
                            v.reload_notes();
                        }
                        if let Some(active) = &self.active_note {
                            let path = active.path.clone();
                            if let Some(v) = &self.vault {
                                if let Some(fresh) =
                                    v.notes.iter().find(|n| n.path == path).cloned()
                                {
                                    self.active_note = Some(fresh);
                                }
                            }
                        }
                        self.external_change_pending = false;
                    }
                    if ui.button("💾 Manter edits (sobrescreve no próximo save)").clicked() {
                        // Push self-write window forward so next save isn't seen as conflict
                        self.self_write_until = std::time::Instant::now()
                            + std::time::Duration::from_millis(400);
                        self.external_change_pending = false;
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
                                // Creating a note replaces active_note, so it must
                                // first flush the current buffer. If a pending
                                // external-change conflict blocks the flush, keep the
                                // modal open and don't switch away — otherwise the
                                // unsaved edits the conflict protects are lost.
                                if self.flush_active() {
                                    self.clear_editor_transients();
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
        let mut picked_preset = None;

        egui::Window::new("Configurações")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                if let Some(v) = &mut self.vault {
                    // Tema
                    ui.horizontal(|ui| {
                        ui.label("Tema:");
                        let current = v.config.theme_preset;
                        egui::ComboBox::from_id_salt("theme_preset_combo")
                            .selected_text(current.label())
                            .show_ui(ui, |ui| {
                                for preset in omninote_core::types::ThemePreset::all() {
                                    if ui
                                        .selectable_label(
                                            v.config.theme_preset == preset,
                                            preset.label(),
                                        )
                                        .clicked()
                                        && v.config.theme_preset != preset
                                    {
                                        v.config.theme_preset = preset;
                                        v.config.dark_mode =
                                            crate::app::theme_for_config(&v.config).dark;
                                        crate::app::theme_for_config(&v.config).apply(ctx);
                                        picked_preset = Some(preset);
                                    }
                                }
                            });
                    });
                    if v.config.theme_preset == omninote_core::types::ThemePreset::Custom {
                        ui.horizontal(|ui| {
                            ui.label("Cor de destaque:");
                            let mut rgb = v.config.accent_color;
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                v.config.accent_color = rgb;
                                crate::app::theme_for_config(&v.config).apply(ctx);
                            }
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
                                for f in omninote_core::types::FontFamily::all() {
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
                        v.config.font_family = omninote_core::types::FontFamily::default();
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
        // Keep the native "Tema" menu's checkmarks in lockstep when the pick
        // came from this Settings ComboBox instead of the native menu itself.
        if let Some(preset) = picked_preset {
            if let Some(nm) = &self.native_menu {
                nm.sync_theme_check(preset);
            }
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
                        let mut removed_active = false;
                        match &action {
                            ConfirmAction::DeleteNote(id) => {
                                if let Some(v) = &mut self.vault {
                                    if let Some(idx) =
                                        v.notes.iter().position(|n| &n.frontmatter.id == id)
                                    {
                                        let _ = v.delete_note(idx);
                                        if self
                                            .active_note
                                            .as_ref()
                                            .is_some_and(|n| &n.frontmatter.id == id)
                                        {
                                            // Deleting the active note discards its
                                            // buffer by design (no flush). Clear the
                                            // dirty/conflict state too so a pending
                                            // external-change modal isn't orphaned
                                            // pointing at a note that no longer exists.
                                            self.active_note = None;
                                            self.dirty = false;
                                            self.external_change_pending = false;
                                            removed_active = true;
                                        }
                                    }
                                }
                            }
                            ConfirmAction::DeleteFolder(p) => {
                                if let Some(v) = &mut self.vault {
                                    let _ = v.delete_folder(p);
                                    if self
                                        .active_note
                                        .as_ref()
                                        .is_some_and(|n| n.rel_path.starts_with(p))
                                    {
                                        // Same as DeleteNote: the active note is gone,
                                        // so drop its dirty/conflict state to avoid an
                                        // orphaned conflict modal.
                                        self.active_note = None;
                                        self.dirty = false;
                                        self.external_change_pending = false;
                                        removed_active = true;
                                    }
                                }
                            }
                        }
                        if removed_active {
                            self.clear_editor_transients();
                        }
                        self.confirm_action = None;
                    }
                    if ui.button("Cancelar").clicked() {
                        self.confirm_action = None;
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

    // Import helpers

    fn import_pdf(&mut self, path: &std::path::Path) {
        // Importing replaces active_note, so flush the current buffer first. A
        // pending external-change conflict blocks the flush — bail so the unsaved
        // edits (and the modal pointing at the current note) aren't silently
        // dropped. A clean-but-dirty buffer is saved here before we switch away.
        if !self.flush_active() {
            return;
        }
        self.clear_editor_transients();
        let content = match omninote_core::pdf::extract_text(path) {
            Ok(t) => t,
            Err(e) => {
                self.error_msg = Some(e);
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("PDF")
            .to_string();
        if let Some(v) = &mut self.vault {
            match v.create_note(None, &title, NoteType::Resumo) {
                Ok(mut note) => {
                    note.content = content;
                    if let Ok(name) = v.import_attachment(path) {
                        note.frontmatter.attachments.push(name);
                    }
                    let _ = v.save_note(&note);
                    if let Some(e) = v
                        .notes
                        .iter_mut()
                        .find(|n| n.frontmatter.id == note.frontmatter.id)
                    {
                        *e = note.clone();
                    }
                    self.active_note = Some(note);
                    self.editing = false;
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }

    fn import_chat(&mut self, path: &std::path::Path) {
        // Flush-first: see import_pdf — bail if a pending conflict blocks the flush.
        if !self.flush_active() {
            return;
        }
        self.clear_editor_transients();
        let content = match omninote_core::import::import_claude_chat(path) {
            Ok(c) => c,
            Err(e) => {
                self.error_msg = Some(e);
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Chat")
            .to_string();
        if let Some(v) = &mut self.vault {
            match v.create_note(None, &title, NoteType::Resumo) {
                Ok(mut note) => {
                    note.content = content;
                    let _ = v.save_note(&note);
                    if let Some(e) = v
                        .notes
                        .iter_mut()
                        .find(|n| n.frontmatter.id == note.frontmatter.id)
                    {
                        *e = note.clone();
                    }
                    self.active_note = Some(note);
                    self.editing = false;
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }

    fn import_artifact(&mut self, path: &std::path::Path) {
        // Flush-first: see import_pdf — bail if a pending conflict blocks the flush.
        if !self.flush_active() {
            return;
        }
        self.clear_editor_transients();
        let content = match omninote_core::import::import_claude_artifact(path) {
            Ok(c) => c,
            Err(e) => {
                self.error_msg = Some(e);
                return;
            }
        };
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Artefato")
            .to_string();
        if let Some(v) = &mut self.vault {
            match v.create_note(None, &title, NoteType::Codigo) {
                Ok(mut note) => {
                    note.content = content;
                    let _ = v.save_note(&note);
                    if let Some(e) = v
                        .notes
                        .iter_mut()
                        .find(|n| n.frontmatter.id == note.frontmatter.id)
                    {
                        *e = note.clone();
                    }
                    self.active_note = Some(note);
                    self.editing = false;
                }
                Err(e) => self.error_msg = Some(e),
            }
        }
    }
}
