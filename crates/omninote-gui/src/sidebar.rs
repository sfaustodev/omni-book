// Left panel: header, search filter, type chips, and a folder tree of notes.
// Notes are drag sources; folder headers (and the root) are drop zones that move
// the note via `vault.move_note_by_id`. Folder collapse state lives in `collapsed`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use egui::{Context, RichText, Sense};
use omninote_core::types::NoteType;

use crate::app::OmniNoteApp;

type Row = (String, String, bool); // (id, label, selected)

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &Context) {
        let mut to_select: Option<String> = None;
        let mut do_new = false;
        let mut pick = false;
        let mut toggle_folder: Option<PathBuf> = None;
        let mut move_op: Option<(String, Option<PathBuf>)> = None;

        egui::SidePanel::left("almanac_sidebar")
            .resizable(true)
            .default_width(266.0)
            .width_range(200.0..=420.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("Almanac");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("＋")
                            .on_hover_text("Nova nota (Cmd/Ctrl+N)")
                            .clicked()
                        {
                            do_new = true;
                        }
                    });
                });
                ui.separator();

                if self.vault.is_none() {
                    ui.add_space(8.0);
                    if ui.button("Abrir vault…").clicked() {
                        pick = true;
                    }
                    return;
                }

                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Filtrar títulos…")
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(self.type_filter.is_none(), "Todos")
                        .clicked()
                    {
                        self.type_filter = None;
                    }
                    for t in NoteType::all() {
                        let on = self.type_filter == Some(t);
                        if ui.selectable_label(on, t.label()).clicked() {
                            self.type_filter = if on { None } else { Some(t) };
                        }
                    }
                });
                ui.separator();

                // Group filtered notes by parent folder; collect the folder list.
                let q = self.search_query.to_lowercase();
                let tf = self.type_filter;
                let active = self.active_id.clone();
                let (folders, groups): (Vec<PathBuf>, BTreeMap<PathBuf, Vec<Row>>) = match &self
                    .vault
                {
                    Some(v) => {
                        let mut groups: BTreeMap<PathBuf, Vec<Row>> = BTreeMap::new();
                        for n in &v.notes {
                            if let Some(t) = tf {
                                if n.frontmatter.note_type != t {
                                    continue;
                                }
                            }
                            if !q.is_empty() && !n.title.to_lowercase().contains(&q) {
                                continue;
                            }
                            let parent = n
                                .rel_path
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_default();
                            let sel = active.as_deref() == Some(n.frontmatter.id.as_str());
                            let label = format!("{}  {}", n.frontmatter.note_type.icon(), n.title);
                            groups.entry(parent).or_default().push((
                                n.frontmatter.id.clone(),
                                label,
                                sel,
                            ));
                        }
                        (v.list_folders(), groups)
                    }
                    None => (Vec::new(), BTreeMap::new()),
                };

                let empty: Vec<Row> = Vec::new();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Root zone.
                    let root_rows = groups.get(&PathBuf::new()).unwrap_or(&empty);
                    let (_z, dropped) = ui.dnd_drop_zone::<String, _>(
                        egui::Frame::none().inner_margin(2.0),
                        |ui| {
                            ui.label(RichText::new("⌂ Raiz").small().weak());
                            note_rows(ui, root_rows, 0.0, &mut to_select);
                        },
                    );
                    if let Some(id) = dropped {
                        move_op = Some(((*id).clone(), None));
                    }

                    // Folders.
                    for folder in &folders {
                        let depth = folder.components().count() as f32;
                        let collapsed = self.collapsed.contains(folder);
                        let arrow = if collapsed { "▸" } else { "▾" };
                        let name = folder
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let (_z, dropped) = ui.dnd_drop_zone::<String, _>(
                            egui::Frame::none().inner_margin(2.0),
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(depth * 10.0);
                                    if ui
                                        .add(
                                            egui::Label::new(format!("{arrow} 📁 {name}"))
                                                .sense(Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        toggle_folder = Some(folder.clone());
                                    }
                                });
                                if !collapsed {
                                    if let Some(rows) = groups.get(folder) {
                                        note_rows(ui, rows, depth * 10.0 + 14.0, &mut to_select);
                                    }
                                }
                            },
                        );
                        if let Some(id) = dropped {
                            move_op = Some(((*id).clone(), Some(folder.clone())));
                        }
                    }
                });
            });

        if let Some(id) = to_select {
            self.select_note(&id);
        }
        if let Some(f) = toggle_folder {
            if !self.collapsed.remove(&f) {
                self.collapsed.insert(f);
            }
        }
        if let Some((id, dest)) = move_op {
            self.move_note(&id, dest);
        }
        if do_new {
            self.show_new = true;
        }
        if pick {
            self.pick_vault(ctx);
        }
    }
}

/// Render note rows as drag sources, indented by `indent` px.
fn note_rows(ui: &mut egui::Ui, rows: &[Row], indent: f32, to_select: &mut Option<String>) {
    for (id, label, sel) in rows {
        ui.horizontal(|ui| {
            if indent > 0.0 {
                ui.add_space(indent);
            }
            ui.dnd_drag_source(egui::Id::new(("note-drag", id)), id.clone(), |ui| {
                if ui.selectable_label(*sel, label).clicked() {
                    *to_select = Some(id.clone());
                }
            });
        });
    }
}
