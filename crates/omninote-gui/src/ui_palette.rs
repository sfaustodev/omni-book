//! Command palette (Ctrl+P) — CAD-25 Fase B Slice 4 §2.6. Fuzzy-search over
//! commands + notes; Enter runs the top hit. Also hosts the in-app quick-capture
//! popup (Ctrl+Shift+Space → append to Inbox.md). Anchored center-top overlay.

use crate::app::OmniNoteApp;
use egui::RichText;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// A thing the palette can run. Notes carry their id; commands a static action.
#[derive(Clone)]
pub enum PaletteItem {
    Command { label: &'static str, action: Cmd },
    Note { id: String, title: String },
}

#[derive(Clone, Copy)]
pub enum Cmd {
    NewNote,
    Settings,
    Import,
    ToggleTheme,
    ToggleRail,
    SwitchVault,
    QuickCapture,
    Tickets,
    Timeline,
}

impl PaletteItem {
    fn haystack(&self) -> &str {
        match self {
            PaletteItem::Command { label, .. } => label,
            PaletteItem::Note { title, .. } => title,
        }
    }
}

/// The fixed command set, always available.
fn commands() -> Vec<PaletteItem> {
    use Cmd::*;
    [
        ("➕ Nova nota", NewNote),
        ("⚙ Configurações", Settings),
        ("📥 Importar", Import),
        ("◐ Alternar tema", ToggleTheme),
        ("⊞ Alternar painel direito", ToggleRail),
        ("📂 Trocar vault", SwitchVault),
        ("✎ Captura rápida", QuickCapture),
        ("◧ Tickets", Tickets),
        ("◷ Timeline", Timeline),
    ]
    .into_iter()
    .map(|(label, action)| PaletteItem::Command { label, action })
    .collect()
}

/// Rank items against `query` with a fuzzy matcher; empty query keeps order.
/// Pure → unit-testable. Returns indices into `items`, best first.
pub fn rank(items: &[PaletteItem], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..items.len()).take(50).collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, it)| matcher.fuzzy_match(it.haystack(), query).map(|s| (s, i)))
        .collect();
    // Higher score first (descending).
    scored.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    scored.into_iter().take(50).map(|(_, i)| i).collect()
}

impl OmniNoteApp {
    /// Build the full candidate list: commands + every note title.
    fn palette_items(&self) -> Vec<PaletteItem> {
        let mut items = commands();
        if let Some(v) = &self.vault {
            for n in &v.notes {
                items.push(PaletteItem::Note {
                    id: n.frontmatter.id.clone(),
                    title: n.title.clone(),
                });
            }
        }
        items
    }

    pub fn show_command_palette(&mut self, ctx: &egui::Context) {
        if !self.palette_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.palette_open = false;
            return;
        }

        let items = self.palette_items();
        let ranked = rank(&items, &self.palette_query);
        self.palette_sel = self.palette_sel.min(ranked.len().saturating_sub(1));

        // Arrow keys move the selection; Enter runs it.
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowDown) {
                self.palette_sel = (self.palette_sel + 1).min(ranked.len().saturating_sub(1));
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                self.palette_sel = self.palette_sel.saturating_sub(1);
            }
        });
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        let mut chosen: Option<usize> = None;
        egui::Window::new("palette")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
            .default_width(520.0)
            .show(ctx, |ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.palette_query)
                        .hint_text("Comando ou nota…")
                        .desired_width(f32::INFINITY),
                );
                edit.request_focus();
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (row, &idx) in ranked.iter().enumerate() {
                            let selected = row == self.palette_sel;
                            let label = match &items[idx] {
                                PaletteItem::Command { label, .. } => label.to_string(),
                                PaletteItem::Note { title, .. } => format!("📄 {title}"),
                            };
                            let resp = ui.selectable_label(selected, RichText::new(label));
                            if resp.clicked() {
                                chosen = Some(idx);
                            }
                        }
                    });
            });

        if enter {
            if let Some(&idx) = ranked.get(self.palette_sel) {
                chosen = Some(idx);
            }
        }
        if let Some(idx) = chosen {
            self.run_palette_item(items[idx].clone(), ctx);
            self.palette_open = false;
            self.palette_query.clear();
            self.palette_sel = 0;
        }
    }

    fn run_palette_item(&mut self, item: PaletteItem, ctx: &egui::Context) {
        match item {
            PaletteItem::Note { id, .. } => self.select_note(&id),
            PaletteItem::Command { action, .. } => match action {
                Cmd::NewNote => self.show_new = true,
                Cmd::Settings => self.show_settings = true,
                Cmd::Import => self.show_import = true,
                Cmd::ToggleRail => {
                    if let Some(v) = &mut self.vault {
                        v.config.right_rail_open = !v.config.right_rail_open;
                    }
                }
                Cmd::ToggleTheme => {
                    if let Some(v) = &mut self.vault {
                        crate::app::toggle_light_dark(&mut v.config);
                        crate::app::theme_for_config(&v.config).apply(ctx);
                    }
                }
                Cmd::SwitchVault => self.pick_vault_with_ctx(ctx),
                Cmd::QuickCapture => {
                    self.capture_open = true;
                    self.capture_text.clear();
                }
                Cmd::Tickets => self.toggle_central_overlay(crate::app::CentralOverlay::Tickets),
                Cmd::Timeline => self.toggle_central_overlay(crate::app::CentralOverlay::Timeline),
            },
        }
    }

    /// In-app quick-capture popup — one line appended to `Inbox.md` (Q-06 plain
    /// bullets at top of file). The global-hotkey daemon is CAD-24, deferred.
    pub fn show_quick_capture(&mut self, ctx: &egui::Context) {
        if !self.capture_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.capture_open = false;
            return;
        }
        let submit = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        egui::Window::new("capture")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 120.0])
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Captura rápida → Inbox.md").weak().size(11.0));
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.capture_text)
                        .hint_text("uma linha…")
                        .desired_width(f32::INFINITY),
                );
                edit.request_focus();
            });
        if submit && !self.capture_text.trim().is_empty() {
            let line = self.capture_text.trim().to_string();
            match self.append_to_inbox(&line) {
                Ok(()) => self.toast_success("Capturado em Inbox.md"),
                Err(e) => self.toast_error(format!("Falha na captura: {e}")),
            }
            self.capture_open = false;
            self.capture_text.clear();
        }
    }

    pub(crate) fn append_to_inbox(&mut self, line: &str) -> Result<(), String> {
        // Flush-first: if the active note IS Inbox.md and dirty, an un-flushed
        // buffer would be auto-saved over the line we're about to capture (and
        // the post-append reload + self_write_until would mask that loss as our
        // own write). Persist the buffer before appending so disk reflects the
        // user's edits, then resync `active_note` from the reloaded vault so the
        // captured bullet is in the live buffer. A pending external-change
        // conflict blocks the flush — bail rather than clobber the disk file.
        if !self.flush_active() {
            return Err("conflito externo pendente — resolva o modal antes de capturar".into());
        }
        let v = self.vault.as_mut().ok_or("sem vault")?;
        v.append_inbox_line(line)?;
        v.reload_notes();
        // Suppress the watcher's external-change modal for our own write — the
        // same self-write window the GUI's save path opens.
        self.self_write_until = std::time::Instant::now() + std::time::Duration::from_millis(400);
        // Resync the active buffer from disk if it was Inbox.md (or any note the
        // append touched), so the just-captured bullet shows immediately instead
        // of being overwritten by the next autosave of the pre-capture buffer.
        if let Some(active_path) = self.active_note.as_ref().map(|n| n.path.clone()) {
            if let Some(v) = &self.vault {
                if let Some(fresh) = v.notes.iter().find(|n| n.path == active_path).cloned() {
                    self.active_note = Some(fresh);
                    self.dirty = false;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<PaletteItem> {
        vec![
            PaletteItem::Command {
                label: "Nova nota",
                action: Cmd::NewNote,
            },
            PaletteItem::Command {
                label: "Configurações",
                action: Cmd::Settings,
            },
            PaletteItem::Note {
                id: "1".into(),
                title: "Sprint review".into(),
            },
        ]
    }

    #[test]
    fn rank_empty_query_keeps_all_in_order() {
        let it = items();
        assert_eq!(rank(&it, ""), vec![0, 1, 2]);
        assert_eq!(rank(&it, "   "), vec![0, 1, 2]);
    }

    #[test]
    fn rank_fuzzy_matches_subsequence() {
        let it = items();
        // "nn" should match "Nova nota" (n..n).
        let r = rank(&it, "nn");
        assert_eq!(r.first().copied(), Some(0));
        // "spr" matches the note title, not the commands.
        let r2 = rank(&it, "spr");
        assert_eq!(r2.first().copied(), Some(2));
    }

    #[test]
    fn rank_no_match_returns_empty() {
        assert!(rank(&items(), "zzzzxxq").is_empty());
    }
}
