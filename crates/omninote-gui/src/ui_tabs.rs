//! Tab strip above the breadcrumb — CAD-25 Fase B §2.4/§5.5. Single-tab for now:
//! renders the one active note as a tab (consistent chrome with the multi-tab
//! future in v1.3). Right cluster: undo/redo (disabled stubs until an edit-history
//! backend exists) + right-rail toggle.

use crate::app::OmniNoteApp;
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
                if ui
                    .small_button("×")
                    .on_hover_text("Fechar (Cmd+W)")
                    .clicked()
                {
                    self.flush_active();
                    self.active_note = None;
                }
            } else {
                ui.label(RichText::new("sem nota").weak().size(12.0));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let rail_open = self.vault.as_ref().map(|v| v.config.right_rail_open);
                if let Some(open) = rail_open {
                    if ui
                        .selectable_label(open, "⊞")
                        .on_hover_text("Painel direito")
                        .clicked()
                    {
                        if let Some(v) = &mut self.vault {
                            v.config.right_rail_open = !v.config.right_rail_open;
                        }
                    }
                }
                // Undo/redo: no edit-history backend yet — visible but disabled.
                ui.add_enabled(false, egui::Button::new("↷").small())
                    .on_disabled_hover_text("Refazer (em breve)");
                ui.add_enabled(false, egui::Button::new("↶").small())
                    .on_disabled_hover_text("Desfazer (em breve)");
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
