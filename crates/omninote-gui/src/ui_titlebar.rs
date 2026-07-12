//! Top chrome: vault badge and global actions. Unavailable actions stay visible
//! only when their disabled state and reason are explicit.

use crate::app::OmniNoteApp;
use crate::ui_a11y::{command_shortcut, icon_button, IconButtonSpec};
use egui::RichText;

const VAULT_BUTTON_MIN_WIDTH: f32 = 120.0;
const VAULT_BUTTON_MAX_WIDTH: f32 = 360.0;
const TITLEBAR_ACTIONS_RESERVED_WIDTH: f32 = 520.0;

fn vault_button_width(available_width: f32) -> f32 {
    (available_width - TITLEBAR_ACTIONS_RESERVED_WIDTH)
        .clamp(VAULT_BUTTON_MIN_WIDTH, VAULT_BUTTON_MAX_WIDTH)
}

impl OmniNoteApp {
    pub fn show_titlebar(&mut self, ctx: &egui::Context) {
        let settings_shortcut = command_shortcut(ctx, egui::Key::Comma, false);
        let theme_shortcut = command_shortcut(ctx, egui::Key::D, true);
        let palette_shortcut = command_shortcut(ctx, egui::Key::P, false);
        let tickets_shortcut = command_shortcut(ctx, egui::Key::J, true);
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
                    let vault_width = vault_button_width(ui.available_width());
                    let vault_button = ui.add_sized(
                        [vault_width, crate::ui_a11y::ICON_BUTTON_MIN_SIDE],
                        egui::Button::new(RichText::new(format!("📓 {vault_name}")).size(12.0))
                            .truncate(),
                    );
                    if vault_button
                        .on_hover_text(format!("Trocar vault — {vault_name}"))
                        .clicked()
                    {
                        self.pick_vault_with_ctx(ctx);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        if icon_button(
                            ui,
                            IconButtonSpec::new("⚙", "Configurações")
                                .shortcut(&settings_shortcut)
                                .selected(self.show_settings),
                        )
                        .clicked()
                        {
                            self.show_settings = true;
                        }
                        if icon_button(
                            ui,
                            IconButtonSpec::new("◐", "Alternar tema").shortcut(&theme_shortcut),
                        )
                        .clicked()
                        {
                            self.toggle_current_theme(ctx);
                        }
                        if icon_button(
                            ui,
                            IconButtonSpec::new("📅", "Calendário").selected(self.calendar_open),
                        )
                        .clicked()
                        {
                            self.calendar_open = !self.calendar_open;
                        }
                        if icon_button(
                            ui,
                            IconButtonSpec::new(&palette_shortcut, "Paleta de comandos")
                                .shortcut(&palette_shortcut)
                                .selected(self.palette_open),
                        )
                        .clicked()
                        {
                            self.palette_open = !self.palette_open;
                        }
                        if ui
                            .button("◧ Tickets")
                            .on_hover_text(format!("Tickets ({tickets_shortcut})"))
                            .clicked()
                        {
                            self.toggle_central_overlay(crate::app::CentralOverlay::Tickets);
                        }
                        icon_button(
                            ui,
                            IconButtonSpec::new("🎙", "Ditado").enabled(false, "Em breve"),
                        );
                    });
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_button_is_bounded_and_leaves_room_for_actions() {
        assert_eq!(vault_button_width(900.0), VAULT_BUTTON_MAX_WIDTH);
        assert!(
            vault_button_width(crate::MIN_WINDOW_WIDTH) + TITLEBAR_ACTIONS_RESERVED_WIDTH
                <= crate::MIN_WINDOW_WIDTH
        );
    }
}
