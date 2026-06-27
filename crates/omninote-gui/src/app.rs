use crate::theme;
use crate::watcher::VaultWatcher;
use eframe::egui;
use egui::RichText;
use omninote_core::types::{ConfirmAction, Note, NoteType};
use omninote_core::vault::Vault;

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

        let (new, toggle_edit, settings, close, palette, capture, toggle_dark) =
            ctx.input_mut(|i| {
                (
                    i.consume_shortcut(&new_sc),
                    i.consume_shortcut(&edit_sc),
                    i.consume_shortcut(&settings_sc),
                    i.consume_shortcut(&close_sc),
                    i.consume_shortcut(&palette_sc),
                    i.consume_shortcut(&capture_sc),
                    i.consume_shortcut(&dark_sc),
                )
            });
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
        self.show_editor(ctx);
        self.show_modals(ctx);
        // Overlays (Slice 4) render on top of the panels.
        self.show_command_palette(ctx);
        self.show_quick_capture(ctx);
        self.show_calendar(ctx);
        self.show_onboarding(ctx);
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
        };
        (app, dir)
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
