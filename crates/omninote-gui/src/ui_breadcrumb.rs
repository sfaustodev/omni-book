//! Editor breadcrumb row: vault-relative path, word count, mode control and
//! destructive action for the active note.

use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::ConfirmAction;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadEditMode {
    Read,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReadEditState {
    editing: bool,
    discipline_typed: bool,
}

fn transitioned_read_edit(current: ReadEditState, target: ReadEditMode) -> ReadEditState {
    match target {
        ReadEditMode::Read => ReadEditState {
            editing: false,
            ..current
        },
        ReadEditMode::Edit => ReadEditState {
            editing: true,
            ..current
        },
    }
}

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

/// Word count over whitespace-separated tokens (a status hint, not a metric).
/// Known limitation: CJK text without spaces counts as one "word"; a grapheme/
/// script-aware count is deferred until there's a CJK user to justify it.
fn word_count(content: &str) -> usize {
    content.split_whitespace().count()
}

impl OmniNoteApp {
    pub(crate) fn set_read_edit_mode(&mut self, target: ReadEditMode) {
        let already_selected = matches!(
            (target, self.editing),
            (ReadEditMode::Read, false) | (ReadEditMode::Edit, true)
        );
        if already_selected {
            return;
        }
        let next = transitioned_read_edit(
            ReadEditState {
                editing: self.editing,
                discipline_typed: self.discipline_typed,
            },
            target,
        );
        self.editing = next.editing;
        self.discipline_typed = next.discipline_typed;
        self.clear_editor_transients();
    }

    pub(crate) fn toggle_read_edit_mode(&mut self) {
        let target = if self.editing {
            ReadEditMode::Read
        } else {
            ReadEditMode::Edit
        };
        self.set_read_edit_mode(target);
    }

    pub fn show_breadcrumb(&mut self, ui: &mut egui::Ui) {
        let (crumb, words, note_id, has_typed_view) = {
            let Some(note) = &self.active_note else {
                return;
            };
            (
                breadcrumb_path(&note.rel_path),
                word_count(&note.content),
                note.frontmatter.id.clone(),
                crate::ui_discipline::has_typed_discipline_view(&note.rel_path),
            )
        };
        let mut requested_mode = None;
        let edit_shortcut = crate::ui_a11y::command_shortcut(ui.ctx(), egui::Key::E, false);
        let mode_tooltip = format!("Alternar modo ({edit_shortcut})");

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{words} palavras")).weak().size(11.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::ui_a11y::icon_button(
                    ui,
                    crate::ui_a11y::IconButtonSpec::new("🗑", "Deletar nota"),
                )
                .clicked()
                {
                    self.confirm_action = Some(ConfirmAction::DeleteNote(note_id.clone()));
                }

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    if crate::ui_a11y::mode_segment_button(ui, "Ler", !self.editing, &mode_tooltip)
                        .clicked()
                    {
                        requested_mode = Some(ReadEditMode::Read);
                    }
                    if crate::ui_a11y::mode_segment_button(
                        ui,
                        "Editar",
                        self.editing,
                        &mode_tooltip,
                    )
                    .clicked()
                    {
                        requested_mode = Some(ReadEditMode::Edit);
                    }
                });

                if has_typed_view && !self.editing {
                    let label = if self.discipline_typed {
                        "≣ Raw"
                    } else {
                        "◈ Typed"
                    };
                    if ui
                        .button(label)
                        .on_hover_text("Alternar vista tipada / markdown cru")
                        .clicked()
                    {
                        self.discipline_typed = !self.discipline_typed;
                    }
                }
            });
        });
        if !crumb.is_empty() {
            ui.add(
                egui::Label::new(RichText::new(format!("{crumb} ›")).weak().size(11.0)).truncate(),
            );
        }
        if let Some(mode) = requested_mode {
            self.set_read_edit_mode(mode);
        }
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

    #[test]
    fn discipline_edit_mode_preserves_the_typed_read_preference() {
        assert_eq!(
            transitioned_read_edit(
                ReadEditState {
                    editing: false,
                    discipline_typed: true,
                },
                ReadEditMode::Edit,
            ),
            ReadEditState {
                editing: true,
                discipline_typed: true,
            }
        );
    }

    #[test]
    fn discipline_read_mode_restores_the_previous_raw_preference() {
        assert_eq!(
            transitioned_read_edit(
                ReadEditState {
                    editing: true,
                    discipline_typed: false,
                },
                ReadEditMode::Read,
            ),
            ReadEditState {
                editing: false,
                discipline_typed: false,
            }
        );
    }

    #[test]
    fn regular_note_preserves_the_discipline_preference() {
        let state = ReadEditState {
            editing: false,
            discipline_typed: true,
        };
        assert_eq!(
            transitioned_read_edit(state, ReadEditMode::Edit),
            ReadEditState {
                editing: true,
                discipline_typed: true,
            }
        );
        assert_eq!(transitioned_read_edit(state, ReadEditMode::Read), state);
    }
}
