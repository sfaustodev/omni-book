// App chrome: top toolbar (discoverable entry points for every primary action)
// and bottom status bar (vault status, unsaved dot, and the external-change
// reload affordance that the filesystem watcher drives).

use egui::{Context, RichText};

use crate::app::OmniNoteApp;

impl OmniNoteApp {
    pub fn show_toolbar(&mut self, ctx: &Context) {
        let mut act_new = false;
        let mut act_search = false;
        let mut act_import = false;
        let mut act_settings = false;
        let mut act_theme = false;

        egui::TopBottomPanel::top("almanac_toolbar").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("OmniNote").strong());
                ui.separator();
                if ui
                    .button("＋ Nova")
                    .on_hover_text("Nova nota · Cmd/Ctrl+N")
                    .clicked()
                {
                    act_new = true;
                }
                if ui
                    .button("Buscar")
                    .on_hover_text("Busca de texto · Cmd/Ctrl+K")
                    .clicked()
                {
                    act_search = true;
                }
                if ui
                    .button("Importar")
                    .on_hover_text("PDF · chat Claude · artefato")
                    .clicked()
                {
                    act_import = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Config")
                        .on_hover_text("Configurações · Cmd/Ctrl+,")
                        .clicked()
                    {
                        act_settings = true;
                    }
                    let theme_label = if self.cfg.dark_mode {
                        "Claro"
                    } else {
                        "Escuro"
                    };
                    if ui
                        .button(theme_label)
                        .on_hover_text("Alternar tema · Cmd/Ctrl+Shift+D")
                        .clicked()
                    {
                        act_theme = true;
                    }
                });
            });
            ui.add_space(3.0);
        });

        if act_new {
            self.show_new = true;
            self.new_title.clear();
        }
        if act_search {
            self.search_open = true;
        }
        if act_import {
            self.show_import = true;
        }
        if act_settings {
            self.show_settings = true;
        }
        if act_theme {
            self.toggle_dark(ctx);
        }
    }

    pub fn show_statusbar(&mut self, ctx: &Context) {
        let mut reload = false;
        egui::TopBottomPanel::bottom("almanac_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status).small().weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.external_changed
                        && ui
                            .button(RichText::new("⟳ recarregar — mudou no disco").small())
                            .clicked()
                    {
                        reload = true;
                    }
                    if self.dirty {
                        ui.label(RichText::new("● não salvo").small());
                    }
                });
            });
        });
        if reload {
            self.reload_vault();
        }
    }
}
