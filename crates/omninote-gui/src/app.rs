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
        // Resolve the startup vault through the shared core resolver so a CLI
        // `omninote vault switch` is honored here too (the registry is the single
        // source of truth).
        //
        // The resolver fails closed on a corrupt `vaults.toml` (the right call for
        // the CLI, which writes). The GUI only opens a vault for viewing, so on a
        // resolver error it falls back to the legacy `last_vault` pointer instead.
        // Without this, a corrupt registry would trap the user in the welcome
        // screen every startup: the picker writes `last_vault` (never repairs the
        // registry), so the next `resolve_active` errors again.
        let resolved = match omninote_core::vaults::resolve_active(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("omninote: vault registry unreadable ({e}); falling back to last_vault");
                Self::legacy_last_vault()
            }
        };
        let vault = resolved.and_then(|p| Vault::open(p).ok());
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

    /// Read the legacy `~/.config/omninote/last_vault` pointer directly, mirroring
    /// the core resolver's legacy branch (trim + require the path to exist). Used
    /// only as the lenient GUI fallback when the registry is corrupt.
    fn legacy_last_vault() -> Option<std::path::PathBuf> {
        let path = omninote_core::vaults::last_vault_path()?;
        std::fs::read_to_string(path)
            .ok()
            .map(|s| std::path::PathBuf::from(s.trim()))
            .filter(|p| p.exists())
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
            // Switching vaults abandons the active note (set to None below). Flush
            // it into the CURRENT vault first; if a pending external-change
            // conflict blocks the flush, bail so the user resolves the modal
            // rather than silently discarding the unsaved buffer.
            if !self.flush_active() {
                return;
            }
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
    /// Whether the inactivity auto-save should fire this frame. Gated on
    /// `!external_change_pending`: while a conflict modal is pending resolution,
    /// auto-saving our in-memory buffer would silently overwrite the external
    /// edit (e.g. a line captured by the CLI) before the user decides. The
    /// caller must poll the watcher first so the flag reflects this frame.
    fn should_autosave(&self, idle: std::time::Duration) -> bool {
        self.dirty && !self.external_change_pending && self.last_save.elapsed() > idle
    }

    pub fn flush_active(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        // A pending external-change conflict means the file on disk holds content
        // we haven't reconciled (e.g. an `omninote capture` write). Refuse to flush
        // over it from ANY caller — auto-save, note switch, or window close — until
        // the user resolves the conflict modal; otherwise the in-memory buffer
        // silently clobbers the external write.
        if self.external_change_pending {
            return false;
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

    /// Replace the active note after flushing the current one. Returns false (and
    /// does NOT replace) when a pending external-change conflict blocks the flush
    /// — the caller must bail so the user resolves the conflict modal first. Use
    /// for EVERY user-initiated active-note change (open / create / import /
    /// vault-switch / close / move); internal reloads (watcher refresh, post-save
    /// resync) set `active_note` directly, since they don't drop unconfirmed edits.
    #[must_use]
    pub(crate) fn switch_active(&mut self, new: Option<Note>) -> bool {
        if !self.flush_active() {
            return false;
        }
        self.active_note = new;
        self.dirty = false;
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
        // Abort the switch if the active buffer couldn't be flushed: a pending
        // external-change conflict makes `flush_active` return false, and
        // replacing `active_note` anyway would discard the unsaved edits the
        // conflict modal exists to protect. Keep the current note + modal up so
        // the user resolves it first. Every note-switch entry point (sidebar,
        // palette, backlinks, calendar, right rail, select_note_by_target)
        // routes through here, so this one guard covers them all.
        if !self.flush_active() {
            return;
        }
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

        // Order is load-bearing: poll the watcher BEFORE the auto-save decision so
        // an external change detected this frame sets `external_change_pending`
        // before `should_autosave` is consulted. Otherwise the inactivity auto-save
        // could flush our in-memory buffer over a just-captured external line in the
        // same frame the conflict is detected, destroying it before the conflict
        // modal ever renders.
        self.poll_watcher();
        if self.should_autosave(Duration::from_millis(600)) {
            self.flush_active();
        }
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
        if tickets {
            self.toggle_central_overlay(CentralOverlay::Tickets);
        }
        if timeline {
            self.toggle_central_overlay(CentralOverlay::Timeline);
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
            // open (flush_active sets error_msg / conflict modal) so edits aren't
            // silently lost.
            let _ = self.switch_active(None);
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
        // On a clean exit this persists the active buffer. If an external-change
        // conflict is still pending, `flush_active` returns false and writes
        // nothing — we deliberately let the on-disk external version win rather
        // than clobber it on the way out (the in-memory edits were never
        // confirmed against the conflict). Config still saves regardless.
        let _ = self.flush_active();
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

    #[test]
    fn select_note_aborts_switch_while_external_change_pending() {
        // The conflict gate is only useful if callers honor it. With a pending
        // external-change conflict + unsaved edits on note A, switching to note B
        // must keep A active (and dirty), not silently discard A's edits.
        let (mut app, _dir) = test_app();
        let id_a = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
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
        assert_ne!(id_a, id_b);

        app.select_note(&id_a);
        app.active_note.as_mut().unwrap().content = "UNSAVED EDIT".into();
        app.dirty = true;
        app.external_change_pending = true;

        app.select_note(&id_b);

        let active = app.active_note.as_ref().expect("note A stays active");
        assert_eq!(
            active.frontmatter.id, id_a,
            "switch must abort: note A stays active while the conflict is pending"
        );
        assert_eq!(
            active.content, "UNSAVED EDIT",
            "note A's unsaved edits must be preserved, not discarded"
        );
        assert!(app.dirty, "still dirty — nothing was saved or switched");
        assert!(
            app.external_change_pending,
            "conflict stays pending until the user resolves the modal"
        );
    }

    #[test]
    fn switch_active_aborts_and_preserves_buffer_while_external_change_pending() {
        // The shared chokepoint behind every user-initiated active-note change
        // (vault switch, import, close, move, "Nova nota aqui"). A pending
        // external-change conflict must make it return false WITHOUT replacing
        // the active note, so the unsaved buffer + conflict modal survive.
        let (mut app, _dir) = test_app();
        let id_a = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id_a);
        app.active_note.as_mut().unwrap().content = "UNSAVED EDIT".into();
        app.dirty = true;
        app.external_change_pending = true;

        // Simulate a vault-switch / close attempt (new = None).
        let switched = app.switch_active(None);

        assert!(
            !switched,
            "switch must report failure while conflict pending"
        );
        let active = app.active_note.as_ref().expect("note A must stay active");
        assert_eq!(active.frontmatter.id, id_a, "active note unchanged");
        assert_eq!(
            active.content, "UNSAVED EDIT",
            "unsaved edits preserved, not dropped"
        );
        assert!(app.dirty, "still dirty — nothing flushed");
        assert!(
            app.external_change_pending,
            "conflict stays pending for the user to resolve"
        );
    }

    #[test]
    fn switch_active_flushes_and_replaces_when_clean_path() {
        // The success path: no conflict → flush the current buffer, then replace
        // with the new note (here None, i.e. a close). The previous note's edits
        // are persisted to disk by the flush, not lost.
        let (mut app, _dir) = test_app();
        let id_a = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id_a);
        app.active_note.as_mut().unwrap().content = "edited body".into();
        app.dirty = true;

        let switched = app.switch_active(None);

        assert!(switched, "clean switch succeeds");
        assert!(
            app.active_note.is_none(),
            "active replaced with None (closed)"
        );
        assert!(!app.dirty, "dirty cleared after flush");
        // The edit must have reached disk via the flush.
        let on_disk = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.frontmatter.id == id_a)
            .expect("note A still in vault")
            .content
            .clone();
        assert!(
            on_disk.contains("edited body"),
            "flush persisted the buffer before the switch; got: {on_disk}"
        );
    }

    #[test]
    fn flush_active_refuses_while_external_change_pending() {
        // A pending conflict must block flush from EVERY caller (not just
        // auto-save): switching notes or closing the window must not overwrite the
        // external write that raised the conflict modal.
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.active_note.as_mut().unwrap().content = "UNSAVED".into();
        app.dirty = true;
        app.external_change_pending = true;
        assert!(
            !app.flush_active(),
            "flush refused while a conflict is pending"
        );
        assert!(app.dirty, "nothing saved — still dirty");
    }

    #[test]
    fn quick_capture_does_not_lose_line_when_inbox_active_and_dirty() {
        // FIX 2: with Inbox.md active AND the user's editor buffer dirty, a quick
        // capture must lose neither side. Without flush-first+resync, the stale
        // in-memory buffer would be auto-saved over the just-captured bullet.
        // Flush-first persists the user's editor edits to disk, the append
        // prepends the bullet on top, and the resync pulls the merged file back
        // into the live buffer (clean) — so the next autosave can't undo it.
        let (mut app, _dir) = test_app();

        // First capture creates Inbox.md and loads it into the vault.
        app.append_to_inbox("primeira captura").unwrap();
        let inbox_id = app
            .vault
            .as_ref()
            .unwrap()
            .notes
            .iter()
            .find(|n| n.path.file_name().and_then(|s| s.to_str()) == Some("Inbox.md"))
            .expect("Inbox.md created")
            .frontmatter
            .id
            .clone();

        // Make Inbox.md the active note, then simulate the user editing it in the
        // main editor (unsaved buffer carrying a sentinel line).
        app.select_note(&inbox_id);
        let edited = format!(
            "{}\nEDITOR EDIT LINE",
            app.active_note.as_ref().unwrap().content
        );
        app.active_note.as_mut().unwrap().content = edited;
        app.dirty = true;

        // Capture a new line while Inbox is active+dirty.
        app.append_to_inbox("segunda captura").unwrap();

        // Disk must carry BOTH the captured bullet and the user's editor edit —
        // neither is dropped.
        let on_disk =
            std::fs::read_to_string(app.vault.as_ref().unwrap().root.join("Inbox.md")).unwrap();
        assert!(
            on_disk.contains("segunda captura"),
            "captured line must reach disk; got:\n{on_disk}"
        );
        assert!(
            on_disk.contains("EDITOR EDIT LINE"),
            "the user's unsaved editor edit must be flushed, not lost; got:\n{on_disk}"
        );

        // The live active buffer was resynced from disk, so it shows the captured
        // line and is clean — the next autosave can't clobber the bullet.
        let active = app.active_note.as_ref().expect("Inbox stays active");
        assert!(
            active.content.contains("segunda captura"),
            "active buffer resynced with the captured line; got:\n{}",
            active.content
        );
        assert!(!app.dirty, "resynced buffer is clean");
    }

    #[test]
    fn autosave_suppressed_while_external_change_pending() {
        // Data-loss guard: a pending external-change conflict must block the
        // inactivity auto-save, or `flush_active` would overwrite an externally
        // captured line (e.g. from `omninote capture`) before the user resolves
        // the modal.
        let (mut app, _dir) = test_app();
        let idle = std::time::Duration::from_millis(600);
        // Make the inactivity window elapse regardless of wall-clock timing.
        app.last_save = std::time::Instant::now() - std::time::Duration::from_secs(5);

        // Clean buffer never auto-saves.
        app.dirty = false;
        app.external_change_pending = false;
        assert!(!app.should_autosave(idle));

        // Dirty + idle elapsed + no conflict → auto-save fires.
        app.dirty = true;
        assert!(app.should_autosave(idle));

        // Dirty + idle elapsed but a conflict is pending → suppressed.
        app.external_change_pending = true;
        assert!(!app.should_autosave(idle));
    }
}
