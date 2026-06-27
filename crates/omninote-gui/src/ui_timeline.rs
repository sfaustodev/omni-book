//! Timeline view — CAD-25 Slice 5 §3.17. A full-panel list of what changed in
//! the vault within a time window, built on [`omninote_core::snapshot::diff_since`]
//! (which shells out to `git` and degrades gracefully for non-git vaults — no
//! `git2` dependency). The git result is cached on the app keyed by the window
//! token so the panel never spawns `git` per frame (the busy-loop lesson).
//!
//! Per-day grouping (§3.17 date headers) needs commit timestamps that
//! `snapshot::ChangeEntry` does not expose yet; v1.2 lists changed files flat
//! under the window header. The diff modal on row-click is likewise deferred —
//! a click selects the note if it lives in the vault.

use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::snapshot;

// ──────────────────────── pure helpers (free fns) ────────────────────────

/// Map a git porcelain status letter to a glyph (§3.17). Unknown → ✎.
pub fn change_glyph(status: &str) -> &'static str {
    match status.chars().next() {
        Some('A') => "⊕",
        Some('M') => "✎",
        Some('D') => "⊝",
        Some('R') => "⤴",
        Some('C') => "⊕",
        _ => "✎",
    }
}

/// Human chip label for a window token, reusing [`snapshot::parse_since`]
/// (`"7d"` → `"7 days ago"`). Falls back to the raw token for empty input.
pub fn window_label(token: &str) -> String {
    snapshot::parse_since(token).unwrap_or_else(|| token.to_string())
}

// ──────────────────────── render ────────────────────────

impl OmniNoteApp {
    /// Refresh the cached snapshot if the window token changed (or nothing is
    /// cached). Called on panel open and on filter change — never per frame.
    fn refresh_timeline_cache(&mut self) {
        let token = self.timeline_since.clone();
        let stale = self
            .timeline_cache
            .as_ref()
            .map(|(since, _)| since != &token)
            .unwrap_or(true);
        if !stale {
            return;
        }
        if let Some(v) = &self.vault {
            if let Ok(report) = snapshot::diff_since(&v.root, &token) {
                self.timeline_cache = Some((token, report));
            }
        }
    }

    pub fn show_timeline_view(&mut self, ui: &mut egui::Ui) {
        let theme = self
            .vault
            .as_ref()
            .map(|v| crate::app::theme_for_config(&v.config))
            .unwrap_or_else(crate::theme::Theme::obsidian_dark);

        ui.horizontal(|ui| {
            ui.label(RichText::new("◷").color(theme.accent).size(18.0));
            ui.label(RichText::new("Timeline").strong().size(18.0));
        });

        // Window filter chips — selecting one mutates the token; the cache picks
        // it up on the next call to refresh below.
        let mut token = self.timeline_since.clone();
        ui.horizontal(|ui| {
            for (label, tok) in [("Today", "1d"), ("This week", "7d"), ("This month", "30d")] {
                let selected = token == tok;
                if ui.selectable_label(selected, label).clicked() {
                    token = tok.to_string();
                }
            }
        });
        if token != self.timeline_since {
            self.timeline_since = token;
        }
        ui.separator();

        self.refresh_timeline_cache();

        let report = self.timeline_cache.as_ref().map(|(_, r)| r.clone());
        let Some(report) = report else {
            ui.label(RichText::new("Sem dados.").weak());
            return;
        };

        if !report.is_git {
            ui.label(
                RichText::new("Inicialize git no vault para habilitar a timeline.")
                    .color(theme.dim),
            );
            return;
        }

        ui.label(
            RichText::new(format!(
                "{} · {} commit(s)",
                window_label(&self.timeline_since),
                report.commits
            ))
            .size(11.0)
            .weak(),
        );
        ui.add_space(4.0);

        if report.changed.is_empty() {
            ui.label(RichText::new("Sem mudanças na janela — tente expandir.").color(theme.dim));
            return;
        }

        let mut pending: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("timeline_scroll")
            .show(ui, |ui| {
                for change in &report.changed {
                    let resp = ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(change_glyph(&change.status))
                                .color(theme.accent)
                                .size(13.0),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(&change.path).color(theme.text).size(12.0),
                            )
                            .sense(egui::Sense::click()),
                        )
                    });
                    if resp
                        .inner
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        pending = Some(change.path.clone());
                    }
                }
            });

        // Row-click v1.2 stub: select the note if the changed path is in the vault.
        if let Some(path) = pending {
            self.select_note_by_target(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_glyph_maps_statuses() {
        assert_eq!(change_glyph("A"), "⊕");
        assert_eq!(change_glyph("M"), "✎");
        assert_eq!(change_glyph("D"), "⊝");
        assert_eq!(change_glyph("R"), "⤴");
        assert_eq!(change_glyph("C"), "⊕");
        assert_eq!(change_glyph("R100"), "⤴");
        // Unknown → ✎.
        assert_eq!(change_glyph("Z"), "✎");
        assert_eq!(change_glyph(""), "✎");
    }

    #[test]
    fn window_label_delegates_to_parse_since() {
        assert_eq!(window_label("7d"), "7 days ago");
        assert_eq!(window_label("1d"), "1 days ago");
        assert_eq!(window_label("30d"), "30 days ago");
    }
}
