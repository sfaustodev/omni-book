use crate::theme;
use crate::watcher::VaultWatcher;
use eframe::egui;
use egui::RichText;
use omninote_core::snapshot::SnapshotReport;
use omninote_core::types::{ConfirmAction, Note, NoteType};
use omninote_core::vault::Vault;
use std::path::PathBuf;

/// Cached Timeline outcome, keyed by the vault root and window token it was
/// computed for. `result` holds the whole outcome — `Ok(report)` (git or non-git)
/// or `Err(message)` — so a failing `git` is remembered instead of re-spawned
/// every frame, and a vault switch is detected by the `root` mismatch.
pub struct TimelineCache {
    pub root: PathBuf,
    pub token: String,
    pub result: Result<SnapshotReport, String>,
}

/// Which full-panel overlay replaces the editor in the central area (Slice 5).
/// Transient (not persisted), mirroring `palette_open`. `None` = the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CentralOverlay {
    #[default]
    None,
    Tickets,
    Timeline,
}

/// Status filter for the Tickets panel chip row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TicketFilter {
    #[default]
    All,
    Doing,
    Todo,
    Done,
}

/// OpenDyslexic, bundled in the binary (OFL). Selectable via the a11y font setting.
const OPEN_DYSLEXIC_OTF: &[u8] = include_bytes!("../assets/fonts/OpenDyslexic-Regular.otf");

/// Register bundled custom fonts. Call once before applying theme/styles.
/// Resolve a config to its concrete theme. While `theme_preset` is still at its
/// default (no settings panel writes it yet — that's Slice 4), honour the legacy
/// `dark_mode` flag so a v1.0 user in light mode isn't silently flipped to dark.
/// An explicitly-chosen preset (light/high-contrast/custom) always wins.
pub(crate) fn theme_for_config(cfg: &omninote_core::types::AppConfig) -> theme::Theme {
    use omninote_core::types::ThemePreset;
    if cfg.theme_preset == ThemePreset::ObsidianDark && !cfg.dark_mode {
        theme::Theme::obsidian_light()
    } else {
        theme::Theme::from_preset(cfg.theme_preset, cfg.accent_color)
    }
}

/// Flip light↔dark in-place, preserving an accessibility/custom preset. Keeps
/// `dark_mode` and `theme_preset` in sync so neither source of truth drifts.
pub(crate) fn toggle_light_dark(cfg: &mut omninote_core::types::AppConfig) {
    use omninote_core::types::ThemePreset;
    cfg.dark_mode = !cfg.dark_mode;
    // Only the plain Obsidian presets track the light/dark boolean; high-contrast
    // and custom are deliberate choices left untouched.
    cfg.theme_preset = match cfg.theme_preset {
        ThemePreset::ObsidianDark | ThemePreset::ObsidianLight if cfg.dark_mode => {
            ThemePreset::ObsidianDark
        }
        ThemePreset::ObsidianDark | ThemePreset::ObsidianLight => ThemePreset::ObsidianLight,
        other => other,
    };
}

fn register_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        theme::OPEN_DYSLEXIC_NAME.to_owned(),
        egui::FontData::from_static(OPEN_DYSLEXIC_OTF),
    );
    // OpenDyslexic ships no emoji/symbol glyphs. Register it as the primary face
    // but append egui's default proportional chain (which carries the emoji
    // fonts) as fallback — otherwise every icon renders as tofu when the
    // dyslexic family is active.
    let mut chain = vec![theme::OPEN_DYSLEXIC_NAME.to_owned()];
    if let Some(default_prop) = fonts.families.get(&egui::FontFamily::Proportional) {
        chain.extend(default_prop.iter().cloned());
    }
    fonts.families.insert(
        egui::FontFamily::Name(theme::OPEN_DYSLEXIC_NAME.into()),
        chain,
    );
    ctx.set_fonts(fonts);
}

/// Consume a command-only `\` key press (the right-rail toggle shortcut),
/// returning whether one fired this frame. Matched command-ONLY instead of via
/// `InputState::consume_shortcut`, whose logical match ignores extra Alt/Shift:
/// on layouts where `\` is typed with AltGr (delivered as Ctrl+Alt) that would
/// trip the toggle and swallow the character in a focused editor.
fn consume_rail_shortcut(i: &mut egui::InputState) -> bool {
    let mut hit = false;
    i.events.retain(|e| {
        let is_rail = matches!(
            e,
            egui::Event::Key {
                key: egui::Key::Backslash,
                pressed: true,
                modifiers,
                ..
            } if modifiers.command_only()
        );
        hit |= is_rail;
        !is_rail
    });
    hit
}

pub struct OmniNoteApp {
    pub vault: Option<Vault>,
    pub active_note: Option<Note>,
    pub editing: bool,
    pub query: String,
    pub type_filter: Option<NoteType>,
    pub show_settings: bool,
    pub show_new: bool,
    pub show_import: bool,
    pub confirm_action: Option<ConfirmAction>,
    pub dirty: bool,
    pub last_save: std::time::Instant,
    pub error_msg: Option<String>,
    pub md_cache: egui_commonmark::CommonMarkCache,
    pub watcher: Option<VaultWatcher>,
    /// Self-write window: events arriving before this instant are ignored
    /// (they came from our own save).
    pub self_write_until: std::time::Instant,
    /// Set when external change detected on the active note while dirty=true.
    /// Triggers a conflict modal asking user to keep edits or reload.
    pub external_change_pending: bool,
    /// v0.8 — index of the `/` that opened the slash menu, in note.content.
    /// None when menu is closed. Set when `/` is typed at start of line.
    pub slash_menu_pos: Option<usize>,
    /// CAD-25 Slice 4 — command palette (Ctrl+P) open state + query + selection.
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_sel: usize,
    /// In-app quick-capture popup (Ctrl+Shift+Space) — appends to Inbox.md.
    pub capture_open: bool,
    pub capture_text: String,
    /// Bottom-right toast queue (action feedback).
    pub toasts: Vec<crate::ui_toasts::Toast>,
    /// One-shot onboarding shown on first run with an empty vault.
    pub onboarding_done: bool,
    /// Calendar popover (daily-note picker) open state + viewed (year, month).
    pub calendar_open: bool,
    pub calendar_ym: Option<(i32, u32)>,
    /// Last known selection/cursor byte range in the content editor, captured
    /// while the editor has it so the right-click format menu can act on it even
    /// after the menu steals focus.
    pub editor_sel: Option<(usize, usize)>,
    /// CAD-25 Slice 5 — full-panel overlay (Tickets / Timeline) over the editor.
    pub central_overlay: CentralOverlay,
    /// Tickets panel: status filter + free-text query (transient).
    pub tickets_filter: TicketFilter,
    pub tickets_query: String,
    /// Timeline panel: window token (`1d`/`7d`/`30d`) + a git-diff result cached
    /// by `(vault root, token)`. The whole outcome is stored — including `Err`
    /// and the non-git case — so a failing `git` never re-spawns every frame
    /// (the busy-loop lesson) and a vault switch can't serve the old vault's
    /// rows. The cache is keyed (not just token-checked) to invalidate on either
    /// axis. (triad-agy/codex Slice 5.)
    pub timeline_since: String,
    pub timeline_cache: Option<TimelineCache>,
    /// Typed↔Raw toggle for discipline files: `true` renders the structured view,
    /// `false` falls back to the generic editor. Defaults to typed.
    pub discipline_typed: bool,
    /// DIARY `+ Append entry` dialog state (the one mutating affordance — reuses
    /// `discipline::diary_quick`).
    pub diary_append_open: bool,
    pub diary_append_text: String,
}

impl OmniNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let last_vault = dirs::config_dir()
            .map(|d| d.join("omninote").join("last_vault"))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(PathBuf::from)
            .filter(|p| p.exists());

        let vault = last_vault.and_then(|p| Vault::open(p).ok());
        register_custom_fonts(&cc.egui_ctx);
        // Required for `egui::Image::new("file://…")` to load attachments from
        // disk in the inline renderer (CAD-25 Slice 3 image embeds).
        egui_extras::install_image_loaders(&cc.egui_ctx);
        vault
            .as_ref()
            .map(|v| theme_for_config(&v.config))
            .unwrap_or_else(theme::Theme::obsidian_dark)
            .apply(&cc.egui_ctx);

        let watcher = vault.as_ref().and_then(|v| VaultWatcher::new(&v.root).ok());

        let app = Self {
            vault,
            active_note: None,
            editing: false,
            query: String::new(),
            type_filter: None,
            show_settings: false,
            show_new: false,
            show_import: false,
            confirm_action: None,
            dirty: false,
            last_save: std::time::Instant::now(),
            error_msg: None,
            md_cache: egui_commonmark::CommonMarkCache::default(),
            watcher,
            self_write_until: std::time::Instant::now(),
            external_change_pending: false,
            slash_menu_pos: None,
            palette_open: false,
            palette_query: String::new(),
            palette_sel: 0,
            capture_open: false,
            capture_text: String::new(),
            toasts: Vec::new(),
            onboarding_done: false,
            calendar_open: false,
            calendar_ym: None,
            editor_sel: None,
            central_overlay: CentralOverlay::None,
            tickets_filter: TicketFilter::All,
            tickets_query: String::new(),
            timeline_since: "7d".to_string(),
            timeline_cache: None,
            discipline_typed: true,
            diary_append_open: false,
            diary_append_text: String::new(),
        };
        app.apply_style(&cc.egui_ctx);
        app
    }

    pub fn save_last_vault(&self) {
        if let Some(v) = &self.vault {
            if let Some(d) = dirs::config_dir() {
                let dir = d.join("omninote");
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(dir.join("last_vault"), v.root.to_string_lossy().as_bytes());
            }
        }
    }

    /// Apply font/spacing settings from vault config to egui style.
    /// Call after vault open and after settings changes.
    pub fn apply_style(&self, ctx: &egui::Context) {
        let v = match &self.vault {
            Some(v) => v,
            None => return,
        };
        let cfg = &v.config;
        let mut style = (*ctx.style()).clone();

        // Font sizes — scale all text styles relative to base font_size
        let base = cfg.font_size;
        let scale = base / 14.0; // 14pt is egui default base
        for (text_style, font_id) in style.text_styles.iter_mut() {
            let default_size = match text_style {
                egui::TextStyle::Heading => 20.0,
                egui::TextStyle::Body => 14.0,
                egui::TextStyle::Monospace => 12.0,
                egui::TextStyle::Button => 14.0,
                egui::TextStyle::Small => 10.0,
                _ => 14.0,
            };
            font_id.size = (default_size * scale).round();
            font_id.family = crate::theme::font_family_to_egui(cfg.font_family);
        }

        // Line spacing via item_spacing
        let extra = (base * (cfg.line_height - 1.0)).max(0.0);
        style.spacing.item_spacing.y = 3.0 + extra;

        ctx.set_style(style);
    }

    pub fn pick_vault_with_ctx(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Escolha (ou crie) uma pasta pra ser seu vault")
            .pick_folder()
        {
            match Vault::open(path) {
                Ok(v) => {
                    let root = v.root.clone();
                    let name = root
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let count = v.notes.len();
                    let theme = theme_for_config(&v.config);
                    self.vault = Some(v);
                    self.active_note = None;
                    // Drop the Timeline cache — it belongs to the old vault root.
                    // (The cache is also root-keyed, but clearing here frees it
                    // promptly and matches the watcher/reload pattern.)
                    self.timeline_cache = None;
                    self.save_last_vault();
                    // Re-apply style + full theme (preset-aware) from the new vault's
                    // config so font and theme take effect immediately, not next toggle.
                    self.apply_style(ctx);
                    theme.apply(ctx);
                    self.watcher = VaultWatcher::new(&root).ok();
                    self.toast_info(format!("Vault “{name}” · {count} notas"));
                }
                Err(e) => {
                    self.toast_error(format!("Falha ao abrir vault: {e}"));
                    self.error_msg = Some(e);
                }
            }
        }
    }

    /// Persist the active note. Returns `true` if it saved (or there was nothing
    /// to save). On a disk failure it returns `false`, keeps `dirty=true`, and
    /// sets `error_msg` — so callers (Cmd+W / tab close) don't drop unsaved work
    /// just because the write failed.
    pub fn flush_active(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let note = match self.active_note.take() {
            Some(n) => n,
            None => {
                self.dirty = false;
                return true;
            }
        };

        // Set self-write window so notify events from our save are ignored
        self.self_write_until = std::time::Instant::now() + std::time::Duration::from_millis(400);

        let mut save_err: Option<String> = None;
        if let Some(v) = &mut self.vault {
            let desired = format!(
                "{}.md",
                omninote_core::vault::sanitize_filename_pub(&note.title)
            );
            let current = note
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let final_note = if desired != current {
                match v.rename_note_by_id(&note.frontmatter.id, &note.title) {
                    Ok(renamed) => {
                        let mut n = renamed;
                        n.frontmatter = note.frontmatter.clone();
                        n.content = note.content.clone();
                        if let Err(e) = v.save_note(&n) {
                            save_err = Some(e);
                        }
                        n
                    }
                    Err(_) => {
                        if let Err(e) = v.save_note(&note) {
                            save_err = Some(e);
                        }
                        note
                    }
                }
            } else {
                if let Err(e) = v.save_note(&note) {
                    save_err = Some(e);
                }
                note
            };

            if let Some(existing) = v
                .notes
                .iter_mut()
                .find(|n| n.frontmatter.id == final_note.frontmatter.id)
            {
                *existing = final_note.clone();
            }
            self.active_note = Some(final_note);
        } else {
            self.active_note = Some(note);
        }

        if let Some(e) = save_err {
            // Keep dirty so the next autosave retries; surface the failure.
            self.error_msg = Some(format!("Falha ao salvar: {e}"));
            return false;
        }
        self.dirty = false;
        self.last_save = std::time::Instant::now();
        true
    }

    /// Toggle a full-panel overlay: a second press of the same target returns to
    /// the editor. Opening Timeline drops the cache so it refreshes on next show.
    pub fn toggle_central_overlay(&mut self, target: CentralOverlay) {
        self.central_overlay = if self.central_overlay == target {
            CentralOverlay::None
        } else {
            target
        };
        if self.central_overlay == CentralOverlay::Timeline {
            self.timeline_cache = None;
        }
    }

    pub fn select_note(&mut self, id: &str) {
        self.flush_active();
        if let Some(v) = &self.vault {
            if let Some(note) = v.notes.iter().find(|n| n.frontmatter.id == id) {
                self.active_note = Some(note.clone());
                self.editing = false;
                self.dirty = false;
                // A successful selection must surface the editor: clear any
                // full-panel overlay (Tickets/Timeline) so the chosen note isn't
                // hidden behind it. Covers every entry point — sidebar, palette,
                // discipline section, timeline row — and select_note_by_target,
                // which delegates here. (triad-agy/codex Slice 5.)
                self.central_overlay = CentralOverlay::None;
                // Drop any selection carried from the previous note — a stale
                // byte range must never act on a different buffer. The consumer
                // also clamps, but resetting at the switch is the real cure.
                // CAD-25b Slice 4. Covers select_note_by_target too (delegates here).
                self.editor_sel = None;
            }
        }
    }

    /// Resolve a wikilink target through the [`omninote_core::resolver::VaultIndex`]
    /// (filename / path / alias / case-insensitive) and select the resulting
    /// note. Returns true if resolved + selected, false if unresolved. CAD-20.
    pub fn select_note_by_target(&mut self, target: &str) -> bool {
        let rel_path = match self.vault.as_ref().and_then(|v| v.index.resolve(target)) {
            Some(p) => p.clone(),
            None => return false,
        };
        let id = self.vault.as_ref().and_then(|v| {
            v.notes
                .iter()
                .find(|n| n.rel_path == rel_path)
                .map(|n| n.frontmatter.id.clone())
        });
        if let Some(id) = id {
            self.select_note(&id);
            true
        } else {
            false
        }
    }

    /// Drain any pending filesystem events from the watcher and reload as needed.
    /// Filters out events arriving during the self-write window.
    pub fn poll_watcher(&mut self) {
        let watcher = match &self.watcher {
            Some(w) => w,
            None => return,
        };
        let changes = watcher.drain_md_changes();
        if changes.is_empty() {
            return;
        }
        // If we just saved, ignore — these are our own writes echoing back
        if std::time::Instant::now() < self.self_write_until {
            return;
        }
        // An external .md change can shift the git diff the Timeline view caches;
        // drop it so the next Timeline render recomputes over fresh state.
        self.timeline_cache = None;
        // Did the active note's file change?
        let active_path = self.active_note.as_ref().map(|n| n.path.clone());
        let active_changed = match &active_path {
            Some(ap) => changes.iter().any(|p| p == ap),
            None => false,
        };

        if active_changed && self.dirty {
            // Conflict: external change + we have unsaved edits. Ask user.
            self.external_change_pending = true;
            return;
        }

        // Safe to reload silently
        if let Some(v) = &mut self.vault {
            v.reload_notes();
            // If active note was renamed/deleted externally, drop it
            if let Some(ap) = active_path {
                let still_exists = v.notes.iter().any(|n| n.path == ap);
                if !still_exists {
                    self.active_note = None;
                } else if active_changed && !self.dirty {
                    // Refresh active_note content from disk
                    if let Some(fresh) = v.notes.iter().find(|n| n.path == ap).cloned() {
                        self.active_note = Some(fresh);
                    }
                }
            }
        }
    }
}

impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use std::time::Duration;

        if self.dirty && self.last_save.elapsed() > Duration::from_millis(600) {
            self.flush_active();
        }

        // Watcher poll (v0.6 — CAD-9)
        self.poll_watcher();
        // Request repaint regularly so watcher events are noticed even when idle
        ctx.request_repaint_after(Duration::from_millis(500));

        // `consume_shortcut` maps COMMAND to Cmd on macOS and Ctrl elsewhere, and
        // consumes the event so a focused TextEdit doesn't also intercept it.
        let new_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::N);
        let edit_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::E);
        let settings_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Comma);
        let close_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::W);
        let palette_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::P);
        let capture_sc = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::Space,
        );
        let dark_sc = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::D,
        );
        let tickets_sc = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::J,
        );
        let timeline_sc = egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
            egui::Key::H,
        );
        let (new, toggle_edit, settings, close, palette, capture, toggle_dark, tickets, timeline) =
            ctx.input_mut(|i| {
                (
                    i.consume_shortcut(&new_sc),
                    i.consume_shortcut(&edit_sc),
                    i.consume_shortcut(&settings_sc),
                    i.consume_shortcut(&close_sc),
                    i.consume_shortcut(&palette_sc),
                    i.consume_shortcut(&capture_sc),
                    i.consume_shortcut(&dark_sc),
                    i.consume_shortcut(&tickets_sc),
                    i.consume_shortcut(&timeline_sc),
                )
            });
        // Cmd/Ctrl+\ toggles the right rail, matched command-ONLY (see
        // `consume_rail_shortcut`) rather than via `consume_shortcut`, whose
        // logical match ignores extra Alt/Shift — that would let AltGr (Ctrl+Alt)
        // typing of `\` on international layouts trip the toggle and eat the char.
        let rail = ctx.input_mut(consume_rail_shortcut);
        if tickets {
            self.toggle_central_overlay(CentralOverlay::Tickets);
        }
        if timeline {
            self.toggle_central_overlay(CentralOverlay::Timeline);
        }
        if rail {
            self.toggle_right_rail();
        }
        if palette {
            self.palette_open = !self.palette_open;
            self.palette_query.clear();
            self.palette_sel = 0;
        }
        if capture {
            self.capture_open = true;
            self.capture_text.clear();
        }
        if close && self.active_note.is_some() {
            // Only drop the note if it actually persisted — otherwise keep it
            // open (flush_active sets error_msg) so edits aren't silently lost.
            if self.flush_active() {
                self.active_note = None;
            }
        }
        if new {
            self.show_new = true;
        }
        if toggle_edit && self.active_note.is_some() {
            self.editing = !self.editing;
        }
        if settings {
            self.show_settings = true;
        }
        if toggle_dark {
            if let Some(v) = &mut self.vault {
                toggle_light_dark(&mut v.config);
                theme_for_config(&v.config).apply(ctx);
            }
        }

        // Error window is rendered before the no-vault early return below, so a
        // failed vault open (which leaves vault=None) still surfaces its message
        // instead of being swallowed by the welcome screen.
        if let Some(err) = self.error_msg.clone() {
            egui::Window::new("Erro")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(&err);
                    if ui.button("OK").clicked() {
                        self.error_msg = None;
                    }
                });
        }

        if self.vault.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.heading("📓 OmniNote");
                    ui.add_space(8.0);
                    ui.label("Escolha uma pasta pra ser seu vault.");
                    ui.label(
                        RichText::new("Compatível com Obsidian e Claude Desktop.")
                            .size(11.0)
                            .weak(),
                    );
                    ui.add_space(20.0);
                    if ui.button("📂 Abrir / Criar Vault").clicked() {
                        self.pick_vault_with_ctx(ctx);
                    }
                });
            });
            return;
        }

        // Panel order: top/bottom chrome and side panels reserve their space
        // before the editor's CentralPanel fills what remains.
        self.show_titlebar(ctx);
        self.show_statusbar(ctx);
        self.show_sidebar(ctx);
        self.show_right_rail(ctx);
        // Central area: an overlay (Tickets/Timeline) replaces the editor when active.
        match self.central_overlay {
            CentralOverlay::None => self.show_editor(ctx),
            CentralOverlay::Tickets => {
                egui::CentralPanel::default().show(ctx, |ui| self.show_tickets_panel(ui));
            }
            CentralOverlay::Timeline => {
                egui::CentralPanel::default().show(ctx, |ui| self.show_timeline_view(ui));
            }
        }
        self.show_modals(ctx);
        // Overlays (Slice 4) render on top of the panels.
        self.show_command_palette(ctx);
        self.show_quick_capture(ctx);
        self.show_calendar(ctx);
        self.show_onboarding(ctx);
        self.show_diary_append(ctx);
        self.show_toasts(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_active();
        if let Some(v) = &self.vault {
            let _ = v.save_config();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an OmniNoteApp headless (no eframe::CreationContext) over a temp vault
    /// with two notes "A" and "B". Mirrors `new()`'s struct init minus the egui-ctx
    /// side effects (fonts/theme/image-loaders), which logic tests don't need.
    fn test_app() -> (OmniNoteApp, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path().to_path_buf()).unwrap();
        vault.create_note(None, "A", NoteType::Resumo).unwrap();
        vault.create_note(None, "B", NoteType::Resumo).unwrap();
        let app = OmniNoteApp {
            vault: Some(vault),
            active_note: None,
            editing: false,
            query: String::new(),
            type_filter: None,
            show_settings: false,
            show_new: false,
            show_import: false,
            confirm_action: None,
            dirty: false,
            last_save: std::time::Instant::now(),
            error_msg: None,
            md_cache: egui_commonmark::CommonMarkCache::default(),
            watcher: None,
            self_write_until: std::time::Instant::now(),
            external_change_pending: false,
            slash_menu_pos: None,
            palette_open: false,
            palette_query: String::new(),
            palette_sel: 0,
            capture_open: false,
            capture_text: String::new(),
            toasts: Vec::new(),
            onboarding_done: false,
            calendar_open: false,
            calendar_ym: None,
            editor_sel: None,
            central_overlay: CentralOverlay::None,
            tickets_filter: TicketFilter::All,
            tickets_query: String::new(),
            timeline_since: "7d".to_string(),
            timeline_cache: None,
            discipline_typed: true,
            diary_append_open: false,
            diary_append_text: String::new(),
        };
        (app, dir)
    }

    #[test]
    fn central_overlay_defaults_to_none() {
        let (app, _dir) = test_app();
        assert_eq!(app.central_overlay, CentralOverlay::None);
        assert!(app.discipline_typed, "typed view on by default");
        assert_eq!(app.timeline_since, "7d");
    }

    #[test]
    fn diary_append_flushes_unsaved_active_edits() {
        // Regression: appending a DIARY entry while the active note has unsaved
        // edits must flush them first — otherwise the reload that re-reads notes
        // from disk replaces the dirty buffer with stale content and the edits are
        // silently lost.
        let (mut app, _dir) = test_app();
        let (id, path) = {
            let n = &app.vault.as_ref().unwrap().notes[0];
            (n.frontmatter.id.clone(), n.path.clone())
        };
        app.select_note(&id);
        app.active_note.as_mut().unwrap().content = "EDITADO EM MEMORIA".into();
        app.dirty = true;

        app.append_diary_entry("nova entrada do diario").unwrap();

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            on_disk.contains("EDITADO EM MEMORIA"),
            "unsaved active-note edit must be flushed before the diary reload"
        );
        assert!(!app.dirty, "active note is clean after a successful append");
    }

    #[test]
    fn toggle_central_overlay_flips_tickets_and_timeline() {
        // Mirrors what Cmd+Shift+J / Cmd+Shift+H call: a press opens, a second
        // press of the same target returns to None (the editor).
        let (mut app, _dir) = test_app();
        app.toggle_central_overlay(CentralOverlay::Tickets);
        assert_eq!(app.central_overlay, CentralOverlay::Tickets);
        app.toggle_central_overlay(CentralOverlay::Tickets);
        assert_eq!(app.central_overlay, CentralOverlay::None);

        app.toggle_central_overlay(CentralOverlay::Timeline);
        assert_eq!(app.central_overlay, CentralOverlay::Timeline);
        // Switching directly to the other overlay (not toggling off) works too.
        app.toggle_central_overlay(CentralOverlay::Tickets);
        assert_eq!(app.central_overlay, CentralOverlay::Tickets);
    }

    #[test]
    fn right_rail_hidden_under_central_overlay() {
        // The rail renders per-active-note metadata, so it must hide while a
        // full-panel overlay (Tickets/Timeline) replaces the editor — otherwise
        // it shows stale context for the last note next to a global view.
        let (mut app, _dir) = test_app();
        app.vault.as_mut().unwrap().config.right_rail_open = true;
        assert!(app.right_rail_visible(), "visible over the editor");

        app.toggle_central_overlay(CentralOverlay::Tickets);
        assert!(
            !app.right_rail_visible(),
            "hidden under the Tickets overlay"
        );

        app.toggle_central_overlay(CentralOverlay::Timeline);
        assert!(
            !app.right_rail_visible(),
            "hidden under the Timeline overlay"
        );

        app.toggle_central_overlay(CentralOverlay::Timeline);
        assert!(app.right_rail_visible(), "returns with the editor");

        // The overlay guard is independent of the user's open/closed toggle.
        app.vault.as_mut().unwrap().config.right_rail_open = false;
        assert!(
            !app.right_rail_visible(),
            "still hidden when toggled closed"
        );
    }

    #[test]
    fn show_right_rail_no_panic_under_overlay_and_editor() {
        // Render smoke: the guard must hold inside a real egui frame, not only in
        // the predicate. Under an overlay no SidePanel is registered (early-return);
        // over the editor with the rail open it renders and exercises the post-guard
        // vault unwrap. A future regression that drops the guard would panic or
        // reserve width here, which the predicate-only test cannot catch.
        let (mut app, _dir) = test_app();
        app.vault.as_mut().unwrap().config.right_rail_open = true;
        let ctx = egui::Context::default();

        app.central_overlay = CentralOverlay::Tickets;
        let _ = ctx.run(Default::default(), |ctx| app.show_right_rail(ctx));

        app.central_overlay = CentralOverlay::None;
        let _ = ctx.run(Default::default(), |ctx| app.show_right_rail(ctx));
    }

    #[test]
    fn toggle_right_rail_no_op_under_overlay() {
        // Every rail-toggle affordance (sidebar/tabs/palette button, Cmd+\) routes
        // through toggle_right_rail. Over the editor it flips the preference; under
        // a full-panel overlay it is inert — the rail has nothing to toggle.
        let (mut app, _dir) = test_app();
        let open0 = app.vault.as_ref().unwrap().config.right_rail_open;

        app.toggle_right_rail();
        assert_eq!(
            app.vault.as_ref().unwrap().config.right_rail_open,
            !open0,
            "toggles over the editor"
        );
        app.toggle_right_rail();
        assert_eq!(app.vault.as_ref().unwrap().config.right_rail_open, open0);

        app.central_overlay = CentralOverlay::Tickets;
        app.toggle_right_rail();
        assert_eq!(
            app.vault.as_ref().unwrap().config.right_rail_open,
            open0,
            "inert while an overlay is active"
        );
    }

    #[test]
    fn backslash_rail_shortcut_is_command_only() {
        // Regression for the AltGr trap (triad round 2, codex + empirical probe):
        // the rail toggle must fire on plain Cmd/Ctrl+\ but NOT when Alt or Shift
        // is also held — otherwise AltGr (delivered as Ctrl+Alt) typing of `\` on
        // international layouts would toggle the rail and swallow the character.
        // Drives the real consume_rail_shortcut path, so a revert to the logical
        // consume_shortcut (which ignores extra modifiers) makes this go red.
        use egui::{Event, Key, Modifiers};
        let fires = |mods: Modifiers| -> bool {
            let ctx = egui::Context::default();
            let mut input = egui::RawInput::default();
            input.events.push(Event::Key {
                key: Key::Backslash,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: mods,
            });
            let mut hit = false;
            let _ = ctx.run(input, |ctx| {
                hit = ctx.input_mut(consume_rail_shortcut);
            });
            hit
        };
        assert!(
            fires(Modifiers::COMMAND),
            "plain Cmd/Ctrl+\\ toggles the rail"
        );
        assert!(
            !fires(Modifiers {
                alt: true,
                ..Modifiers::COMMAND
            }),
            "Cmd+Alt+\\ must not fire"
        );
        assert!(
            !fires(Modifiers {
                shift: true,
                ..Modifiers::COMMAND
            }),
            "Cmd+Shift+\\ must not fire"
        );
        assert!(
            !fires(Modifiers {
                ctrl: true,
                alt: true,
                command: true,
                ..Default::default()
            }),
            "AltGr (Ctrl+Alt) typing of backslash must not fire"
        );
    }

    #[test]
    fn opening_timeline_clears_stale_cache() {
        let (mut app, dir) = test_app();
        // Seed a stale cache, then open Timeline — it must drop so the next show
        // refreshes (avoids serving a window's results under a different token).
        app.timeline_cache = Some(TimelineCache {
            root: dir.path().to_path_buf(),
            token: "1d".to_string(),
            result: Ok(omninote_core::snapshot::SnapshotReport {
                since: "1 days ago".into(),
                is_git: false,
                commits: 0,
                changed: Vec::new(),
            }),
        });
        app.toggle_central_overlay(CentralOverlay::Timeline);
        assert!(app.timeline_cache.is_none(), "cache cleared on open");
    }

    #[test]
    fn timeline_view_over_non_git_vault_populates_cache_once() {
        // A non-git tempdir vault: snapshot degrades to is_git=false; the panel
        // must cache that result once and not re-spawn git every frame.
        let (mut app, _dir) = test_app();
        app.central_overlay = CentralOverlay::Timeline;
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.show_timeline_view(ui);
            });
        });
        let cache = app.timeline_cache.as_ref().expect("cache populated");
        assert_eq!(cache.token, "7d", "keyed on the active window token");
        let report = cache.result.as_ref().expect("non-git vault is Ok, not Err");
        assert!(!report.is_git, "tempdir vault is not a git work tree");

        // A second frame with the same token must not replace the cache instance.
        let before = cache.result.clone();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.show_timeline_view(ui);
            });
        });
        assert_eq!(
            app.timeline_cache.as_ref().unwrap().result,
            before,
            "same-token frame reuses the cached report"
        );
    }

    #[test]
    fn tickets_panel_tolerates_missing_jira() {
        // A Notion-only vault (no JIRA.md) must render the panel without error.
        let (mut app, dir) = test_app();
        std::fs::create_dir_all(dir.path().join("discipline")).unwrap();
        std::fs::write(
            dir.path().join("discipline/NOTION.md"),
            "# NOTION\n\n| ID | Title | Status |\n|--|--|--|\n| CAD-1 | x | 🌱 Backlog |\n",
        )
        .unwrap();
        app.central_overlay = CentralOverlay::Tickets;
        let ctx = egui::Context::default();
        // No panic / no error despite JIRA.md being absent.
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                app.show_tickets_panel(ui);
            });
        });
        assert!(app.error_msg.is_none());
    }

    #[test]
    fn select_note_clears_central_overlay() {
        // Selecting a note while Tickets/Timeline is open must surface the editor
        // (clear the overlay) so the chosen note isn't hidden. (triad Slice 5.)
        let (mut app, _dir) = test_app();
        let id_a = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.title == "A")
            .unwrap()
            .frontmatter
            .id
            .clone();
        app.central_overlay = CentralOverlay::Tickets;
        app.select_note(&id_a);
        assert_eq!(
            app.central_overlay,
            CentralOverlay::None,
            "a successful select must drop the overlay"
        );

        // A no-op select (unknown id) leaves the overlay untouched.
        app.central_overlay = CentralOverlay::Timeline;
        app.select_note("does-not-exist");
        assert_eq!(app.central_overlay, CentralOverlay::Timeline);
    }

    #[test]
    fn timeline_cache_keyed_by_root_and_token_survives_frames() {
        // Regression for the busy-loop: a populated cache for the active
        // (root, token) must be reused — not re-spawned — on subsequent frames,
        // even when the underlying result was a non-git Ok (and, by the same
        // keying, an Err would be remembered too). (triad Slice 5.)
        let (mut app, _dir) = test_app();
        app.central_overlay = CentralOverlay::Timeline;
        let ctx = egui::Context::default();
        let run = |app: &mut OmniNoteApp| {
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| app.show_timeline_view(ui));
            });
        };
        run(&mut app);
        let first = app.timeline_cache.as_ref().expect("cache populated");
        let root = first.root.clone();
        assert_eq!(first.token, "7d");
        // Mutating the cached result in place lets us detect a spurious refresh:
        // if the panel re-ran diff_since it would overwrite this sentinel.
        app.timeline_cache.as_mut().unwrap().result = Ok(SnapshotReport {
            since: "sentinel".into(),
            is_git: false,
            commits: 42,
            changed: Vec::new(),
        });
        run(&mut app);
        let again = app.timeline_cache.as_ref().unwrap();
        assert_eq!(again.root, root);
        assert_eq!(
            again.result.as_ref().unwrap().commits,
            42,
            "same (root, token) frame must not re-spawn git"
        );
    }

    #[test]
    fn select_note_resets_stale_editor_selection() {
        // CAD-25b Slice 4 (review finding): switching notes must clear editor_sel so
        // a stale byte range from the previous note can't act on a different buffer.
        let (mut app, _dir) = test_app();
        let id_b = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.title == "B")
            .unwrap()
            .frontmatter
            .id
            .clone();
        app.editor_sel = Some((5, 10));
        app.select_note(&id_b);
        assert_eq!(
            app.editor_sel, None,
            "select_note deve limpar a seleção stale"
        );
        assert!(app.active_note.is_some(), "nota B deve ficar ativa");
    }
}
