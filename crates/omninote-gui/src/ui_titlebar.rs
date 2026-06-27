//! Top chrome (title bar) — CAD-25 Fase B §2.3. Vault badge + global actions.
//! A `TopBottomPanel::top`, shown first in `update()` so it claims height before
//! the side/central panels. Actions with a backend today are live; palette and
//! dictation are visible-but-disabled stubs until their slices land (Slice 4 /
//! CAD-23.3) — per the "no half-baked feature in front of the user" rule, a
//! disabled control with an "em breve" tooltip beats a fake working one.

use crate::app::OmniNoteApp;
use egui::RichText;

impl OmniNoteApp {
    pub fn show_titlebar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(34.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(6.0);
                    let vault_name = self
                        .vault
                        .as_ref()
                        .and_then(|v| v.root.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "sem vault".to_string());
                    if ui
                        .button(RichText::new(format!("📓 {vault_name}")).size(12.0))
                        .on_hover_text("Trocar vault")
                        .clicked()
                    {
                        self.pick_vault_with_ctx(ctx);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        if ui
                            .button("⚙")
                            .on_hover_text("Configurações (Cmd+,)")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                        if ui.button("◐").on_hover_text("Tema (Cmd+Shift+D)").clicked() {
                            if let Some(v) = &mut self.vault {
                                crate::app::toggle_light_dark(&mut v.config);
                                crate::app::theme_for_config(&v.config).apply(ctx);
                            }
                        }
                        if ui.button("📅").on_hover_text("Calendário").clicked() {
                            self.calendar_open = !self.calendar_open;
                        }
                        if ui.button("⌘P").on_hover_text("Paleta (Cmd+P)").clicked() {
                            self.palette_open = !self.palette_open;
                        }
                        // Dictation stub — backend is CAD-23.3 (Slice 6 wires UI).
                        ui.add_enabled(false, egui::Button::new("🎙"))
                            .on_disabled_hover_text("Ditado (em breve)");
                    });
                });
            });
    }
}
