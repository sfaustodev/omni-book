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
    /// Refresh the cached snapshot when the vault root or window token changed
    /// (or nothing is cached). The whole outcome — `Ok` *or* `Err` — is cached,
    /// so a failing `git` (corrupt repo, missing binary, permission error) is
    /// remembered instead of re-spawned every frame at 60fps. The key includes
    /// the vault root, so switching vaults invalidates a previous vault's rows.
    /// (triad-agy/codex Slice 5 — the busy-loop lesson.)
    fn refresh_timeline_cache(&mut self) {
        let token = self.timeline_since.clone();
        if let Some(v) = &self.vault {
            let fresh = self
                .timeline_cache
                .as_ref()
                .map(|c| c.root != v.root || c.token != token)
                .unwrap_or(true);
            if !fresh {
                return;
            }
            let result = snapshot::diff_since(&v.root, &token);
            self.timeline_cache = Some(crate::app::TimelineCache {
                root: v.root.clone(),
                token,
                result,
            });
        }
    }

    pub fn show_timeline_view(&mut self, ui: &mut egui::Ui) {
        let theme = self
            .vault
            .as_ref()
            .map(|v| crate::app::theme_for_config(&v.config))
            .unwrap_or_else(crate::theme::Theme::obsidian_dark);

        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "◷", 18.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, "Timeline", 18.0).strong());
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

        // The cache holds the whole outcome; surface Err so a git failure is shown
        // once (not re-spawned every frame). Clone out to drop the &self borrow.
        let outcome = self.timeline_cache.as_ref().map(|c| c.result.clone());
        let report = match outcome {
            Some(Ok(r)) => r,
            Some(Err(e)) => {
                ui.label(crate::ui_a11y::scaled_text(ui, "Timeline indisponível.", 13.0).strong());
                ui.label(crate::ui_a11y::scaled_text(ui, e, 11.0).color(theme.dim));
                return;
            }
            None => {
                ui.label(RichText::new("Sem dados.").weak());
                return;
            }
        };

        if !report.is_git {
            ui.label(
                crate::ui_a11y::scaled_text(
                    ui,
                    "Inicialize git no vault para habilitar a timeline.",
                    13.0,
                )
                .color(theme.dim),
            );
            return;
        }

        ui.label(
            crate::ui_a11y::scaled_text(
                ui,
                format!(
                    "{} · {} commit(s)",
                    window_label(&self.timeline_since),
                    report.commits
                ),
                11.0,
            )
            .weak(),
        );
        ui.add_space(4.0);

        if report.changed.is_empty() {
            ui.label(
                crate::ui_a11y::scaled_text(ui, "Sem mudanças na janela — tente expandir.", 13.0)
                    .color(theme.dim),
            );
            return;
        }

        let mut pending: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("timeline_scroll")
            .show(ui, |ui| {
                for change in &report.changed {
                    let label = format!("{} {}", change_glyph(&change.status), change.path);
                    let (_, activated) = crate::ui_a11y::clickable_row(ui, &theme, &label, |ui| {
                        ui.label(
                            crate::ui_a11y::scaled_text(ui, change_glyph(&change.status), 13.0)
                                .color(theme.accent),
                        );
                        ui.label(
                            crate::ui_a11y::scaled_text(ui, &change.path, 12.0).color(theme.text),
                        );
                    });
                    if activated {
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
