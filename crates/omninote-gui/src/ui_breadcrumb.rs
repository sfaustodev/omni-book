//! Editor breadcrumb row — CAD-25 Fase B §2.4. Path from vault root to the note,
//! plus a right cluster: word count, view/edit toggle, delete. Replaces the old
//! ad-hoc sticky header that lived inline in `ui_editor::show_editor`.

use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::ConfirmAction;
use std::path::Path;

/// `Daily/2026/2026-05-20.md` → `Daily / 2026`. Returns an empty string for a
/// note at the vault root (no parent segments to show).
fn breadcrumb_path(rel_path: &Path) -> String {
    rel_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .unwrap_or_default()
}

/// Word count over whitespace-separated tokens (good enough for a status hint).
fn word_count(content: &str) -> usize {
    content.split_whitespace().count()
}

impl OmniNoteApp {
    pub fn show_breadcrumb(&mut self, ui: &mut egui::Ui) {
        let Some(note) = &self.active_note else {
            return;
        };
        let crumb = breadcrumb_path(&note.rel_path);
        let words = word_count(&note.content);
        let note_id = note.frontmatter.id.clone();

        ui.horizontal(|ui| {
            if !crumb.is_empty() {
                ui.label(RichText::new(crumb).weak().size(11.0));
                ui.label(RichText::new("›").weak().size(11.0));
            }
            ui.label(RichText::new(format!("{words} palavras")).weak().size(11.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑").on_hover_text("Deletar nota").clicked() {
                    self.confirm_action = Some(ConfirmAction::DeleteNote(note_id));
                }
                let (glyph, hint) = if self.editing {
                    ("👁", "Ver (Cmd+E)")
                } else {
                    ("✎", "Editar (Cmd+E)")
                };
                if ui.button(glyph).on_hover_text(hint).clicked() {
                    self.editing = !self.editing;
                }
            });
        });
        ui.separator();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn breadcrumb_joins_parent_segments() {
        let p = PathBuf::from("Daily/2026/2026-05-20.md");
        assert_eq!(breadcrumb_path(&p), "Daily / 2026");
    }

    #[test]
    fn breadcrumb_empty_for_root_note() {
        assert_eq!(breadcrumb_path(&PathBuf::from("note.md")), "");
    }

    #[test]
    fn word_count_counts_whitespace_tokens() {
        assert_eq!(word_count("um dois  três\nquatro"), 4);
        assert_eq!(word_count("   "), 0);
        assert_eq!(word_count(""), 0);
    }
}
