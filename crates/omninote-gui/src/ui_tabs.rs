//! Single-note tab strip with close, edit-history status and right-rail toggle.

use crate::app::OmniNoteApp;
use crate::ui_a11y::{command_shortcut, icon_button, IconButtonSpec};
use egui::RichText;

/// Truncate a tab title to fit, appending `…` when cut. `max_chars` is a rough
/// proxy for the ~140px width budget in the spec.
fn truncate_title(title: &str, max_chars: usize) -> String {
    let count = title.chars().count();
    if count <= max_chars {
        return title.to_string();
    }
    let head: String = title.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{head}…")
}

impl OmniNoteApp {
    pub fn show_tab_strip(&mut self, ui: &mut egui::Ui) {
        let close_shortcut = command_shortcut(ui.ctx(), egui::Key::W, false);
        let rail_shortcut = command_shortcut(ui.ctx(), egui::Key::Backslash, false);
        ui.horizontal(|ui| {
            if let Some(note) = &self.active_note {
                let glyph = note.frontmatter.note_type.icon();
                let title = if note.title.is_empty() {
                    "Untitled".to_string()
                } else {
                    truncate_title(&note.title, 24)
                };
                let dot = if self.dirty { " ●" } else { "" };
                ui.label(RichText::new(format!("{glyph} {title}{dot}")).size(12.0));
                if icon_button(
                    ui,
                    IconButtonSpec::new("×", "Fechar nota").shortcut(&close_shortcut),
                )
                .clicked()
                {
                    // Closing the tab drops the active note: flush-first so a
                    // pending external-change conflict keeps the note open
                    // instead of discarding the unsaved buffer.
                    let _ = self.switch_active(None);
                }
            } else {
                ui.label(RichText::new("sem nota").weak().size(12.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let rail_open = self.vault.as_ref().map(|v| v.config.right_rail_open);
                if let Some(open) = rail_open {
                    let rail_enabled = self.central_overlay == crate::app::CentralOverlay::None;
                    if icon_button(
                        ui,
                        IconButtonSpec::new("⊞", "Painel direito")
                            .shortcut(&rail_shortcut)
                            .selected(open)
                            .enabled(rail_enabled, "Indisponível sobre outro painel"),
                    )
                    .clicked()
                    {
                        self.toggle_right_rail();
                    }
                }
                icon_button(
                    ui,
                    IconButtonSpec::new("↷", "Refazer").enabled(false, "Em breve"),
                );
                icon_button(
                    ui,
                    IconButtonSpec::new("↶", "Desfazer").enabled(false, "Em breve"),
                );
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_title_is_unchanged() {
        assert_eq!(truncate_title("curto", 24), "curto");
    }

    #[test]
    fn long_title_is_truncated_with_ellipsis() {
        let long = "um título bem longo que ultrapassa o limite";
        let t = truncate_title(long, 10);
        assert_eq!(t.chars().count(), 10);
        assert!(t.ends_with('…'));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        // Multibyte chars must not be sliced mid-codepoint.
        let t = truncate_title("áéíóúçãõ-extra-long-tail", 5);
        assert_eq!(t.chars().count(), 5);
        assert!(t.starts_with('á'));
    }
}
