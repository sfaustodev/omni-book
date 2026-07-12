//! Calendar popover (daily-note picker) + first-run onboarding — CAD-25 Fase B
//! Slice 4 §3.10/§3.15. Month grid in pt-BR (Q-16); clicking a day opens/creates
//! that daily note. Onboarding fires once on an empty vault (Q-15).

use crate::app::OmniNoteApp;
use crate::ui_a11y::{icon_button, IconButtonSpec};
use chrono::{Datelike, Local, NaiveDate};
use egui::RichText;

/// pt-BR weekday headers, Sunday-first to match the grid.
const DOW_PT: [&str; 7] = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];
const MONTHS_PT: [&str; 12] = [
    "Janeiro",
    "Fevereiro",
    "Março",
    "Abril",
    "Maio",
    "Junho",
    "Julho",
    "Agosto",
    "Setembro",
    "Outubro",
    "Novembro",
    "Dezembro",
];

/// Days in `year`/`month` (1-based month). Pure → unit-testable.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_next = NaiveDate::from_ymd_opt(ny, nm, 1).unwrap();
    first_next.pred_opt().unwrap().day()
}

/// Weekday of the 1st (0 = Sunday … 6 = Saturday). Pure.
pub fn first_weekday(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap()
        .weekday()
        .num_days_from_sunday()
}

impl OmniNoteApp {
    /// Calendar popover — month grid (pt-BR); clicking a day opens/creates that
    /// daily note via the core `ensure_daily`.
    pub fn show_calendar(&mut self, ctx: &egui::Context) {
        if !self.calendar_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.calendar_open = false;
            return;
        }
        let today = Local::now().date_naive();
        let (year, month) = self.calendar_ym.unwrap_or((today.year(), today.month()));

        let mut nav: Option<(i32, u32)> = None;
        let mut pick: Option<NaiveDate> = None;
        egui::Window::new("calendar")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 48.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if icon_button(ui, IconButtonSpec::new("‹", "Mês anterior")).clicked() {
                        nav = Some(if month == 1 {
                            (year - 1, 12)
                        } else {
                            (year, month - 1)
                        });
                    }
                    ui.label(
                        RichText::new(format!("{} {year}", MONTHS_PT[(month - 1) as usize]))
                            .strong(),
                    );
                    if icon_button(ui, IconButtonSpec::new("›", "Próximo mês")).clicked() {
                        nav = Some(if month == 12 {
                            (year + 1, 1)
                        } else {
                            (year, month + 1)
                        });
                    }
                });
                ui.separator();
                egui::Grid::new("cal_grid").num_columns(7).show(ui, |ui| {
                    for d in DOW_PT {
                        ui.label(RichText::new(d).weak().size(11.0));
                    }
                    ui.end_row();
                    let lead = first_weekday(year, month);
                    for _ in 0..lead {
                        ui.label("");
                    }
                    let mut col = lead;
                    for day in 1..=days_in_month(year, month) {
                        let is_today =
                            year == today.year() && month == today.month() && day == today.day();
                        let lbl = if is_today {
                            RichText::new(format!("{day}")).strong()
                        } else {
                            RichText::new(format!("{day}"))
                        };
                        if ui.button(lbl).clicked() {
                            pick = NaiveDate::from_ymd_opt(year, month, day);
                        }
                        col += 1;
                        if col.is_multiple_of(7) {
                            ui.end_row();
                        }
                    }
                });
            });

        if let Some(ym) = nav {
            self.calendar_ym = Some(ym);
        }
        if let Some(date) = pick {
            self.open_daily(date);
            self.calendar_open = false;
        }
    }

    /// Open (or create) the daily note for `date` via the core, then select it.
    fn open_daily(&mut self, date: NaiveDate) {
        let Some(v) = self.vault.as_mut() else { return };
        let opts = omninote_core::daily::DailyOpts {
            date: Some(date),
            ..Default::default()
        };
        match omninote_core::daily::ensure_daily(&v.root, opts) {
            Ok(res) => {
                v.reload_notes();
                if let Some(n) = v.notes.iter().find(|n| n.path == res.path) {
                    let id = n.frontmatter.id.clone();
                    self.select_note(&id);
                }
                self.toast_success(format!("Daily {date}"));
            }
            Err(e) => self.toast_error(format!("Falha no daily: {e}")),
        }
    }

    /// First-run onboarding: a one-shot welcome on an empty vault.
    pub fn show_onboarding(&mut self, ctx: &egui::Context) {
        if self.onboarding_done {
            return;
        }
        let empty = self
            .vault
            .as_ref()
            .map(|v| v.notes.is_empty())
            .unwrap_or(false);
        if !empty {
            self.onboarding_done = true;
            return;
        }
        egui::Window::new("Bem-vindo ao OmniNote")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let new_shortcut = crate::ui_a11y::command_shortcut(ctx, egui::Key::N, false);
                let palette_shortcut = crate::ui_a11y::command_shortcut(ctx, egui::Key::P, false);
                ui.label("Seu vault está vazio. Pra começar:");
                ui.add_space(6.0);
                ui.label(RichText::new(format!("• {new_shortcut} — criar a primeira nota")).weak());
                ui.label(
                    RichText::new(format!("• {palette_shortcut} — paleta de comandos")).weak(),
                );
                ui.label(RichText::new("• Compatível com Obsidian e Claude Desktop").weak());
                ui.add_space(8.0);
                if ui.button("Começar").clicked() {
                    self.onboarding_done = true;
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_in_month_handles_leap_and_short() {
        assert_eq!(days_in_month(2024, 2), 29); // leap
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn first_weekday_is_sunday_indexed() {
        // 2026-06-01 is a Monday → 1.
        assert_eq!(first_weekday(2026, 6), 1);
    }

    #[test]
    fn pt_br_labels_present() {
        assert_eq!(DOW_PT[0], "Dom");
        assert_eq!(MONTHS_PT[2], "Março");
    }
}
