// Modal dialogs: new note, find (full-text), import, settings, onboarding.
// Each uses the same open/collect-action/apply-after pattern to stay borrow-clean.

use std::path::PathBuf;

use egui::{Context, RichText};
use omninote_core::search::{search, SearchOpts};
use omninote_core::types::{FontFamily, NoteType, ThemePreset};

use crate::state::OmniNoteApp;

#[derive(Clone, Copy)]
enum ImportKind {
    Pdf,
    Chat,
    Artifact,
}

fn preset_label(p: ThemePreset) -> &'static str {
    match p {
        ThemePreset::ObsidianDark => "blueprint (noite)",
        ThemePreset::ObsidianLight => "draft (claro)",
        ThemePreset::HighContrast => "alto contraste",
        ThemePreset::Custom => "personalizado",
    }
}

impl OmniNoteApp {
    pub fn show_dialogs(&mut self, ctx: &Context) {
        self.dlg_new(ctx);
        self.dlg_find(ctx);
        self.dlg_import(ctx);
        self.dlg_settings(ctx);
        self.dlg_onboarding(ctx);
    }

    fn dlg_new(&mut self, ctx: &Context) {
        if !self.show_new {
            return;
        }
        let mut keep_open = true;
        let mut create = false;
        let mut cancel = false;
        egui::Window::new("nova nota")
            .collapsible(false)
            .resizable(false)
            .open(&mut keep_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -40.0))
            .show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_title)
                        .hint_text("título")
                        .desired_width(280.0),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for t in NoteType::all() {
                        let label = format!("{} {}", t.icon(), t.label());
                        if ui.selectable_label(self.new_type == t, label).clicked() {
                            self.new_type = t;
                        }
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("criar").clicked() {
                        create = true;
                    }
                    if ui.button("cancelar").clicked() {
                        cancel = true;
                    }
                });
            });
        if create {
            self.create_from_dialog();
            self.show_new = false;
        } else if cancel || !keep_open {
            self.show_new = false;
        }
    }

    fn create_from_dialog(&mut self) {
        let title = self.new_title.trim().to_string();
        let nt = self.new_type;
        let result = match self.vault.as_mut() {
            Some(v) => v.create_note(None, &title, nt),
            None => return,
        };
        match result {
            Ok(n) => {
                let id = n.frontmatter.id.clone();
                self.new_title.clear();
                self.select_note(&id);
                self.editing = true;
            }
            Err(e) => self.status = format!("erro ao criar: {e}"),
        }
    }

    fn dlg_find(&mut self, ctx: &Context) {
        if !self.search_open {
            return;
        }
        let mut keep_open = true;
        let mut jump: Option<PathBuf> = None;
        egui::Window::new("buscar")
            .collapsible(false)
            .resizable(true)
            .open(&mut keep_open)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 70.0))
            .show(ctx, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.find_query)
                        .hint_text("texto a buscar no conteúdo…")
                        .desired_width(f32::INFINITY),
                );
                resp.request_focus();
                ui.separator();

                let query = self.find_query.trim().to_string();
                let hits = match (&self.vault, query.is_empty()) {
                    (Some(v), false) => search(
                        &v.notes,
                        &query,
                        SearchOpts {
                            case_sensitive: false,
                            limit: Some(60),
                        },
                    ),
                    _ => Vec::new(),
                };
                if query.is_empty() {
                    ui.label(
                        RichText::new("digite para buscar no texto das notas.")
                            .weak()
                            .small(),
                    );
                } else {
                    ui.label(
                        RichText::new(format!("{} resultado(s)", hits.len()))
                            .weak()
                            .small(),
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(380.0)
                    .show(ui, |ui| {
                        for h in &hits {
                            let head = ui.add(
                                egui::Label::new(
                                    RichText::new(format!("{}  · linha {}", h.title, h.line_no))
                                        .strong(),
                                )
                                .sense(egui::Sense::click()),
                            );
                            ui.label(RichText::new(&h.snippet).small().weak());
                            ui.add_space(3.0);
                            if head.clicked() {
                                jump = Some(h.rel_path.clone());
                            }
                        }
                    });
            });
        if let Some(rel) = jump {
            self.select_note_by_rel(&rel);
            self.search_open = false;
            self.find_query.clear();
        }
        if !keep_open {
            self.search_open = false;
        }
    }

    fn dlg_import(&mut self, ctx: &Context) {
        if !self.show_import {
            return;
        }
        let mut keep_open = true;
        let mut action: Option<ImportKind> = None;
        egui::Window::new("importar")
            .collapsible(false)
            .resizable(false)
            .open(&mut keep_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("importar como nova nota:");
                ui.add_space(8.0);
                if ui.button("📄  PDF — texto extraído…").clicked() {
                    action = Some(ImportKind::Pdf);
                }
                if ui.button("💬  chat Claude (.json)…").clicked() {
                    action = Some(ImportKind::Chat);
                }
                if ui.button("🧩  artefato Claude (.json)…").clicked() {
                    action = Some(ImportKind::Artifact);
                }
            });
        if let Some(kind) = action {
            self.run_import(kind);
            self.show_import = false;
        }
        if !keep_open {
            self.show_import = false;
        }
    }

    fn run_import(&mut self, kind: ImportKind) {
        let (filter_name, exts): (&str, &[&str]) = match kind {
            ImportKind::Pdf => ("PDF", &["pdf"]),
            _ => ("JSON", &["json"]),
        };
        let path = match rfd::FileDialog::new()
            .add_filter(filter_name, exts)
            .pick_file()
        {
            Some(p) => p,
            None => return,
        };
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "importado".to_string());
        let res = match kind {
            ImportKind::Pdf => omninote_core::pdf::extract_text(&path),
            ImportKind::Chat => omninote_core::import::import_claude_chat(&path),
            ImportKind::Artifact => omninote_core::import::import_claude_artifact(&path),
        };
        match res {
            Ok(body) => self.import_as_note(&title, body),
            Err(e) => self.status = format!("falha ao importar: {e}"),
        }
    }

    fn dlg_settings(&mut self, ctx: &Context) {
        if !self.show_settings {
            return;
        }
        let mut keep_open = true;
        let mut changed = false;
        let mut switch_vault = false;
        egui::Window::new("configurações")
            .collapsible(false)
            .resizable(false)
            .open(&mut keep_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_min_width(330.0);
                ui.label(RichText::new("tema").strong());
                egui::ComboBox::from_label("preset")
                    .selected_text(preset_label(self.cfg.theme_preset))
                    .show_ui(ui, |ui| {
                        for p in [
                            ThemePreset::ObsidianLight,
                            ThemePreset::ObsidianDark,
                            ThemePreset::HighContrast,
                            ThemePreset::Custom,
                        ] {
                            if ui
                                .selectable_value(&mut self.cfg.theme_preset, p, preset_label(p))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                if ui
                    .checkbox(&mut self.cfg.dark_mode, "modo escuro")
                    .changed()
                {
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("cor de destaque");
                    if ui
                        .color_edit_button_srgb(&mut self.cfg.accent_color)
                        .changed()
                    {
                        self.cfg.theme_preset = ThemePreset::Custom;
                        changed = true;
                    }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.label(RichText::new("acessibilidade").strong());
                egui::ComboBox::from_label("fonte")
                    .selected_text(self.cfg.font_family.label())
                    .show_ui(ui, |ui| {
                        for f in FontFamily::all() {
                            if ui
                                .selectable_value(&mut self.cfg.font_family, f, f.label())
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                if ui
                    .add(egui::Slider::new(&mut self.cfg.font_size, 10.0..=26.0).text("tamanho"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.line_height, 1.0..=2.2)
                            .text("altura linha"),
                    )
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(
                        egui::Slider::new(&mut self.cfg.letter_spacing, 0.0..=4.0)
                            .text("espaçamento"),
                    )
                    .changed()
                {
                    changed = true;
                }

                ui.add_space(6.0);
                ui.separator();
                ui.label(RichText::new("vault").strong());
                if let Some(v) = &self.vault {
                    ui.label(RichText::new(v.root.display().to_string()).small().weak());
                }
                if ui.button("trocar vault…").clicked() {
                    switch_vault = true;
                }
            });
        if changed {
            self.apply_theme(ctx);
            self.persist_config();
        }
        if switch_vault {
            self.show_settings = false;
            self.pick_vault(ctx);
        }
        if !keep_open {
            self.show_settings = false;
        }
    }

    fn dlg_onboarding(&mut self, ctx: &Context) {
        if self.tutorial_step == 0 {
            return;
        }
        let mut keep_open = true;
        egui::Window::new("omninote · blueprint")
            .collapsible(false)
            .resizable(false)
            .open(&mut keep_open)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("um vault de notas em Markdown, compatível com Obsidian.");
                ui.add_space(8.0);
                if ui.button("escolher pasta-vault…").clicked() {
                    self.pick_vault(ctx);
                }
            });
        if !keep_open {
            self.tutorial_step = 0;
        }
    }
}
