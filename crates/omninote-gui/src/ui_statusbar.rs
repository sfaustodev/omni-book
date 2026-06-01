//! Bottom chrome (status bar) — CAD-25 Fase B §1.4/§2. Save state, note count,
//! active note type, app version. A `TopBottomPanel::bottom`, so it must be
//! shown before the `CentralPanel` in `update()`.

use crate::app::OmniNoteApp;
use egui::RichText;

impl OmniNoteApp {
    pub fn show_statusbar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(8.0);

                    let (dot, label) = if self.dirty {
                        ("●", "editando")
                    } else {
                        ("✓", "salvo")
                    };
                    ui.label(RichText::new(format!("{dot} {label}")).size(11.0).weak());

                    if let Some(v) = &self.vault {
                        ui.separator();
                        ui.label(
                            RichText::new(format!("{} notas", v.notes.len()))
                                .size(11.0)
                                .weak(),
                        );
                    }

                    if let Some(n) = &self.active_note {
                        ui.separator();
                        let t = n.frontmatter.note_type;
                        ui.label(
                            RichText::new(format!("{} {}", t.icon(), t.label()))
                                .size(11.0)
                                .weak(),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(concat!("v", env!("CARGO_PKG_VERSION")))
                                .size(11.0)
                                .weak(),
                        );
                    });
                });
            });
    }
}
