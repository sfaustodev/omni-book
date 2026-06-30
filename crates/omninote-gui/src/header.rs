// Top command bar (discoverable entry points) + bottom "References" dock. The dock
// is this edition's home for wikilinks + backlinks (navigable chips) and also
// carries vault status, the unsaved marker, and the external-change reload button.

use std::path::PathBuf;

use egui::{Context, RichText};

use crate::state::OmniNoteApp;

impl OmniNoteApp {
    pub fn show_command_bar(&mut self, ctx: &Context) {
        let mut a_new = false;
        let mut a_find = false;
        let mut a_import = false;
        let mut a_settings = false;
        let mut a_theme = false;

        egui::TopBottomPanel::top("bp_cmd").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("▤ OMNINOTE").strong());
                let vault_name = self
                    .vault
                    .as_ref()
                    .and_then(|v| v.root.file_name())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "—".to_string());
                ui.label(RichText::new(format!("· {vault_name}")).weak().small());
                ui.separator();
                if ui
                    .button("new")
                    .on_hover_text("Nova nota · Cmd/Ctrl+N")
                    .clicked()
                {
                    a_new = true;
                }
                if ui
                    .button("find")
                    .on_hover_text("Busca de texto · Cmd/Ctrl+K")
                    .clicked()
                {
                    a_find = true;
                }
                if ui
                    .button("import")
                    .on_hover_text("PDF · chat Claude · artefato")
                    .clicked()
                {
                    a_import = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("cfg")
                        .on_hover_text("Configurações · Cmd/Ctrl+,")
                        .clicked()
                    {
                        a_settings = true;
                    }
                    let t = if self.cfg.dark_mode {
                        "draft"
                    } else {
                        "blueprint"
                    };
                    if ui
                        .button(t)
                        .on_hover_text("Tema · Cmd/Ctrl+Shift+D")
                        .clicked()
                    {
                        a_theme = true;
                    }
                });
            });
            ui.add_space(3.0);
        });

        if a_new {
            self.show_new = true;
            self.new_title.clear();
        }
        if a_find {
            self.search_open = true;
        }
        if a_import {
            self.show_import = true;
        }
        if a_settings {
            self.show_settings = true;
        }
        if a_theme {
            self.toggle_dark(ctx);
        }
    }

    pub fn show_dock(&mut self, ctx: &Context) {
        let mut nav: Option<PathBuf> = None;
        let mut reload = false;

        egui::TopBottomPanel::bottom("bp_dock")
            .min_height(28.0)
            .show(ctx, |ui| {
                if self.active_note.is_some() {
                    let (out_links, backlinks) = self.compute_connections();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("→ links:").small().weak());
                        if out_links.is_empty() {
                            ui.label(RichText::new("—").small().weak());
                        }
                        for (label, rel) in &out_links {
                            let chip = match rel {
                                Some(_) => RichText::new(format!("[[{label}]]")).small(),
                                None => RichText::new(format!("[[{label}]]?")).small().weak(),
                            };
                            if ui.link(chip).clicked() {
                                if let Some(r) = rel {
                                    nav = Some(r.clone());
                                }
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("← backlinks:").small().weak());
                        if backlinks.is_empty() {
                            ui.label(RichText::new("—").small().weak());
                        }
                        for (label, rel) in &backlinks {
                            if ui.link(RichText::new(label).small()).clicked() {
                                nav = Some(rel.clone());
                            }
                        }
                    });
                    ui.separator();
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).small().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.external_changed
                            && ui
                                .button(RichText::new("⟳ reload (disco)").small())
                                .clicked()
                        {
                            reload = true;
                        }
                        if self.dirty {
                            ui.label(RichText::new("● unsaved").small());
                        }
                    });
                });
            });

        if let Some(rel) = nav {
            self.select_note_by_rel(&rel);
        }
        if reload {
            self.reload_vault();
        }
    }
}
