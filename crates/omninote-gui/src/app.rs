use crate::native_menu;
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
/// JetBrains Mono (OFL) — the Terminal body + monospace face, bundled in the binary.
const JETBRAINS_MONO_TTF: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
/// Space Grotesk SemiBold (OFL) — the display face used for headings, bundled in.
const SPACE_GROTESK_TTF: &[u8] = include_bytes!("../assets/fonts/SpaceGrotesk-SemiBold.ttf");

/// Resolve the compatibility state used by legacy configs that predate presets.
pub(crate) fn effective_theme_preset(
    cfg: &omninote_core::types::AppConfig,
) -> omninote_core::types::ThemePreset {
    use omninote_core::types::ThemePreset;
    if cfg.theme_preset == ThemePreset::ObsidianDark && !cfg.dark_mode {
        ThemePreset::ObsidianLight
    } else {
        cfg.theme_preset
    }
}

pub(crate) fn theme_for_config(cfg: &omninote_core::types::AppConfig) -> theme::Theme {
    theme::Theme::from_preset(effective_theme_preset(cfg), cfg.accent_color)
}

pub(crate) fn select_theme_preset(
    cfg: &mut omninote_core::types::AppConfig,
    preset: omninote_core::types::ThemePreset,
) {
    cfg.theme_preset = preset;
    cfg.dark_mode = theme::Theme::from_preset(preset, cfg.accent_color).dark;
}

/// Flip light↔dark in-place, preserving an accessibility/custom preset. Keeps
/// `dark_mode` and `theme_preset` in sync so neither source of truth drifts.
/// Each theme family with a light/dark sibling pair flips within its own pair
/// (Obsidian, Almanac, Blueprint); families with no sibling (`HighContrast`,
/// `Custom`, `Swiss`) are deliberate choices left untouched.
pub(crate) fn toggle_light_dark(cfg: &mut omninote_core::types::AppConfig) {
    use omninote_core::types::ThemePreset;
    cfg.dark_mode = !cfg.dark_mode;
    cfg.theme_preset = match cfg.theme_preset {
        ThemePreset::ObsidianDark | ThemePreset::ObsidianLight if cfg.dark_mode => {
            ThemePreset::ObsidianDark
        }
        ThemePreset::ObsidianDark | ThemePreset::ObsidianLight => ThemePreset::ObsidianLight,
        ThemePreset::AlmanacDark | ThemePreset::AlmanacLight if cfg.dark_mode => {
            ThemePreset::AlmanacDark
        }
        ThemePreset::AlmanacDark | ThemePreset::AlmanacLight => ThemePreset::AlmanacLight,
        ThemePreset::Blueprint | ThemePreset::BlueprintLight if cfg.dark_mode => {
            ThemePreset::Blueprint
        }
        ThemePreset::Blueprint | ThemePreset::BlueprintLight => ThemePreset::BlueprintLight,
        other => other,
    };
}

fn register_custom_fonts(ctx: &egui::Context) {
    // Start from egui's defaults: they carry the emoji/symbol fallback fonts that
    // render the UI glyphs (◧ ◷ ▤ …) and emoji. We register our faces as the
    // PRIMARY of each family and APPEND egui's original chain as fallback, so a
    // glyph our face lacks still resolves instead of rendering as tofu.
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        theme::OPEN_DYSLEXIC_NAME.to_owned(),
        egui::FontData::from_static(OPEN_DYSLEXIC_OTF),
    );
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        egui::FontData::from_static(JETBRAINS_MONO_TTF),
    );
    fonts.font_data.insert(
        theme::SPACE_GROTESK_NAME.to_owned(),
        egui::FontData::from_static(SPACE_GROTESK_TTF),
    );

    // Snapshot egui's default chains before we mutate them, to reuse as the
    // emoji/symbol fallback tail for every family below.
    let default_prop = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let default_mono = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Terminal = mono body: JetBrains Mono leads BOTH the proportional (primary
    // body) and monospace families. Append egui's original tails for fallback.
    let prop_chain = {
        let mut c = vec!["JetBrainsMono".to_owned()];
        c.extend(default_prop.iter().cloned());
        c
    };
    let mono_chain = {
        let mut c = vec!["JetBrainsMono".to_owned()];
        c.extend(default_mono.iter().cloned());
        c
    };
    fonts
        .families
        .insert(egui::FontFamily::Proportional, prop_chain.clone());
    fonts
        .families
        .insert(egui::FontFamily::Monospace, mono_chain);

    // Space Grotesk display family (headings) — falls back through the same
    // proportional tail so non-Latin/emoji headings still render.
    let grotesk_chain = {
        let mut c = vec![theme::SPACE_GROTESK_NAME.to_owned()];
        c.extend(default_prop.iter().cloned());
        c
    };
    fonts.families.insert(
        egui::FontFamily::Name(theme::SPACE_GROTESK_NAME.into()),
        grotesk_chain,
    );

    // OpenDyslexic (a11y opt-in) ships no emoji/symbol glyphs — same tofu-proof
    // fallback tail as the body.
    let dyslexic_chain = {
        let mut c = vec![theme::OPEN_DYSLEXIC_NAME.to_owned()];
        c.extend(default_prop.iter().cloned());
        c
    };
    fonts.families.insert(
        egui::FontFamily::Name(theme::OPEN_DYSLEXIC_NAME.into()),
        dyslexic_chain,
    );

    ctx.set_fonts(fonts);
}

/// Consume an application keyboard shortcut, returning whether it fired this
/// frame. AltGr-safe: rejects any event where Alt is held. No app shortcut uses
/// Alt, and on Windows/Linux AltGr arrives as Ctrl+Alt and maps to `command`, so
/// egui's own `consume_shortcut` (a logical match that ignores extra modifiers)
/// fires every COMMAND chord while the user types an AltGr character — `€`
/// (AltGr+E) would trip Cmd+E. Shift, by contrast, is matched logically: a
/// pattern's Shift must be present, but extra Shift is tolerated, because some
/// layouts need Shift to produce a shortcut's key (`=` is Shift+0 on German
/// QWERTZ, so invoking Cmd+= sends Ctrl+Shift+Equals). Key repeats are consumed
/// too — so a held chord doesn't leak to a focused editor — but only a fresh
/// press fires the action.
pub(crate) fn consume_app_shortcut(i: &mut egui::InputState, sc: &egui::KeyboardShortcut) -> bool {
    let mut hit = false;
    i.events.retain(|e| {
        let matched = matches!(
            e,
            egui::Event::Key { key, pressed: true, modifiers, .. }
            if *key == sc.logical_key
                && !modifiers.alt
                && modifiers.matches_logically(sc.modifiers)
        );
        if matched && matches!(e, egui::Event::Key { repeat: false, .. }) {
            hit = true;
        }
        !matched
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
    /// One-shot formatting command tied to the note and selection that spawned it.
    pub pending_editor_action: Option<crate::ui_editor::PendingEditorAction>,
    /// The native macOS "Tema"/"Editar" menu bar (`native_menu.rs`). A no-op
    /// stub on non-macOS targets — see that module's doc comment. `Option` so
    /// `update()` can `.take()` it before calling `pump(self, ctx)` — the same
    /// take-then-restore idiom `flush_active`/`active_note` use, sidestepping
    /// the double-mutable-borrow a plain field would need (`pump` takes
    /// `&mut OmniNoteApp`, which is `self` itself).
    pub native_menu: Option<native_menu::NativeMenu>,
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
        let current_preset = vault
            .as_ref()
            .map(|v| effective_theme_preset(&v.config))
            .unwrap_or_default();
        let native_menu = match native_menu::NativeMenu::build(&cc.egui_ctx, current_preset) {
            Ok(menu) => Some(menu),
            Err(error) => {
                eprintln!("omninote: native menu unavailable ({error})");
                None
            }
        };

        let app = Self {
            vault,
            native_menu,
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
            pending_editor_action: None,
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

        // Mono-forward Terminal type scale. Sizes are px-before-scale; the whole
        // map scales by the user's font_size relative to egui's 14px base. The
        // BODY face follows the a11y font setting (JetBrains Mono by default, or
        // OpenDyslexic when chosen); the HEADING is always the Space Grotesk
        // display face — a deliberate display/body split — UNLESS the user picked
        // the dyslexic face, where legibility wins and the heading uses it too.
        let scale = cfg.font_size / 14.0;
        let body_family = crate::theme::font_family_to_egui(cfg.font_family);
        let heading_family = if cfg.font_family == omninote_core::types::FontFamily::Dyslexic {
            body_family.clone()
        } else {
            egui::FontFamily::Name(crate::theme::SPACE_GROTESK_NAME.into())
        };
        let px = |size: f32| (size * scale).round();
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(px(26.0), heading_family),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(px(13.5), body_family.clone()),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(px(12.5), egui::FontFamily::Monospace),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(px(12.0), body_family.clone()),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(px(11.0), body_family),
            ),
        ]
        .into();

        // Accessibility line-height: extra vertical gap between stacked widgets
        // (and wrapped lines) proportional to the chosen line_height. Owned here,
        // not in `theme.apply()`, so a theme re-bind never wipes it.
        let extra = (cfg.font_size * (cfg.line_height - 1.0)).max(0.0);
        style.spacing.item_spacing.y = 3.0 + extra;

        // TODO(letter_spacing): egui 0.29 exposes no native per-glyph tracking on
        // TextStyle/FontId, so `cfg.letter_spacing` cannot be applied to the egui
        // text layout here. It is applied in the md_render text paths in a later
        // phase. The settings slider still round-trips the value (persisted in
        // config); it is intentionally not silently dropped.
        let _ = cfg.letter_spacing;

        ctx.set_style(style);
    }

    pub(crate) fn apply_current_theme_and_style(&self, ctx: &egui::Context) {
        let Some(vault) = &self.vault else {
            return;
        };
        theme_for_config(&vault.config).apply(ctx);
        self.apply_style(ctx);
    }

    pub(crate) fn toggle_current_theme(&mut self, ctx: &egui::Context) {
        if let Some(vault) = &mut self.vault {
            toggle_light_dark(&mut vault.config);
            let _ = vault.save_config();
        }
        self.apply_current_theme_and_style(ctx);
        let preset = self
            .vault
            .as_ref()
            .map(|vault| effective_theme_preset(&vault.config));
        if let (Some(native_menu), Some(preset)) = (&self.native_menu, preset) {
            native_menu.sync_theme_check(preset);
        }
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
                    self.vault = Some(v);
                    self.active_note = None;
                    self.clear_editor_transients();
                    // Drop the Timeline cache — it belongs to the old vault root.
                    // (The cache is also root-keyed, but clearing here frees it
                    // promptly and matches the watcher/reload pattern.)
                    self.timeline_cache = None;
                    self.save_last_vault();
                    self.apply_current_theme_and_style(ctx);
                    if let (Some(native_menu), Some(preset)) = (
                        &self.native_menu,
                        self.vault
                            .as_ref()
                            .map(|vault| effective_theme_preset(&vault.config)),
                    ) {
                        native_menu.sync_theme_check(preset);
                    }
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
        self.clear_editor_transients();
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
        let selected = self
            .vault
            .as_ref()
            .and_then(|vault| vault.notes.iter().find(|note| note.frontmatter.id == id))
            .cloned();
        if let Some(note) = selected {
            self.clear_editor_transients();
            self.active_note = Some(note);
            self.editing = false;
            self.dirty = false;
            // A successful selection must surface the editor: clear any
            // full-panel overlay (Tickets/Timeline) so the chosen note isn't
            // hidden behind it. Covers every entry point — sidebar, palette,
            // discipline section, timeline row — and select_note_by_target.
            self.central_overlay = CentralOverlay::None;
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

        // Any cached byte target belongs to the pre-reload buffer.
        self.clear_editor_transients();
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

        // Drain native "Tema"/"Editar" menu clicks first, before anything else
        // reads `pending_editor_action`/`editor_sel` this frame. Take-then-restore
        // (mirrors `flush_active`/`active_note`): `pump` needs `&mut self`, which
        // a plain `&mut self.native_menu` field access can't also hold.
        if let Some(mut nm) = self.native_menu.take() {
            nm.pump(self, ctx);
            self.native_menu = Some(nm);
        }

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

        // All shortcuts go through `consume_app_shortcut` (AltGr-safe — rejects
        // Alt) instead of egui's logical `consume_shortcut`, which fired every
        // COMMAND chord while typing an AltGr char on Windows/Linux (`€` = AltGr+E
        // tripped Cmd+E). COMMAND maps to Cmd on macOS / Ctrl elsewhere; the consume
        // removes the event so a focused TextEdit doesn't also see it.
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
        let rail_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Backslash);
        let (
            new,
            toggle_edit,
            settings,
            close,
            palette,
            capture,
            toggle_dark,
            tickets,
            timeline,
            rail,
        ) = ctx.input_mut(|i| {
            (
                consume_app_shortcut(i, &new_sc),
                consume_app_shortcut(i, &edit_sc),
                consume_app_shortcut(i, &settings_sc),
                consume_app_shortcut(i, &close_sc),
                consume_app_shortcut(i, &palette_sc),
                consume_app_shortcut(i, &capture_sc),
                consume_app_shortcut(i, &dark_sc),
                consume_app_shortcut(i, &tickets_sc),
                consume_app_shortcut(i, &timeline_sc),
                consume_app_shortcut(i, &rail_sc),
            )
        });
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
            // open (flush_active sets error_msg / conflict modal) so edits aren't
            // silently lost.
            let _ = self.switch_active(None);
        }
        if new {
            self.show_new = true;
        }
        if toggle_edit && self.active_note.is_some() {
            self.toggle_read_edit_mode();
        }
        if settings {
            self.show_settings = true;
        }
        if toggle_dark {
            self.toggle_current_theme(ctx);
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
        // CRT / phosphor scanlines — the Matrix-terminal texture. A thin dark line
        // every 3px on a foreground layer (sits above all panels, captures no
        // input). Dark mode only; a light theme has no CRT to fake.
        let crt = self
            .vault
            .as_ref()
            .map(|v| theme_for_config(&v.config).dark)
            .unwrap_or(true);
        if crt {
            let screen = ctx.screen_rect();
            let p = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("crt_scanlines"),
            ));
            let line = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30);
            let mut y = screen.top();
            while y < screen.bottom() {
                p.hline(screen.x_range(), y, egui::Stroke::new(1.0_f32, line));
                y += 3.0;
            }
        }
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
            pending_editor_action: None,
            // None in the headless test helper: `muda::Menu` can only be built on
            // the real AppKit main thread, and `cargo test` runs each test on its
            // own worker thread. `pump`/`sync_theme_check` are no-ops on `None`.
            native_menu: None,
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
    fn applying_a_theme_preserves_the_configured_line_height() {
        let (mut app, _dir) = test_app();
        let config = &mut app.vault.as_mut().unwrap().config;
        config.font_size = 18.0;
        config.line_height = 1.8;
        let ctx = egui::Context::default();

        app.apply_current_theme_and_style(&ctx);

        let expected = 3.0 + 18.0 * 0.8;
        assert!((ctx.style().spacing.item_spacing.y - expected).abs() < 0.0001);
    }

    #[test]
    fn long_nested_folder_name_does_not_expand_fixed_sidebar() {
        let (mut app, _dir) = test_app();
        let vault = app.vault.as_mut().unwrap();
        vault.create_folder(None, "Projects").unwrap();
        vault
            .create_folder(
                Some(std::path::Path::new("Projects")),
                "web3-threat-reports",
            )
            .unwrap();
        vault
            .create_folder(
                Some(std::path::Path::new("Projects/web3-threat-reports")),
                "2026-04-tirios-contagious-interview",
            )
            .unwrap();
        select_theme_preset(
            &mut vault.config,
            omninote_core::types::ThemePreset::AlmanacLight,
        );

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        register_custom_fonts(&ctx);
        app.apply_current_theme_and_style(&ctx);
        let mut central_left = 0.0;
        let mut full_name_is_accessible = false;
        let mut full_name_top = None;

        for frame in 0..3 {
            let mut input = egui::RawInput {
                time: Some(f64::from(frame) * 0.2),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                ..Default::default()
            };
            input.focused = true;

            let output = ctx.run(input, |ctx| {
                app.show_sidebar(ctx);
                central_left = egui::CentralPanel::default()
                    .show(ctx, |_| {})
                    .response
                    .rect
                    .left();
            });
            if let Some(update) = output.platform_output.accesskit_update {
                for (_, node) in &update.nodes {
                    if node.name() == Some("2026-04-tirios-contagious-interview") {
                        full_name_is_accessible = true;
                        full_name_top = node.bounds().map(|bounds| bounds.y0);
                    }
                }
            }
        }

        assert!((central_left - 280.0).abs() < f32::EPSILON);
        assert!(full_name_is_accessible);
        assert!(
            full_name_top.is_some_and(|top| top < 800.0),
            "folder tree must remain inside the viewport, got y={full_name_top:?}"
        );
    }

    #[test]
    fn shortcut_mode_toggle_uses_the_discipline_transition() {
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.active_note.as_mut().unwrap().rel_path = "discipline/SPRINT.md".into();

        app.toggle_read_edit_mode();
        assert!(app.editing);
        assert!(app.discipline_typed);

        app.toggle_read_edit_mode();
        assert!(!app.editing);
        assert!(app.discipline_typed);
    }

    #[test]
    fn shortcut_mode_toggle_restores_the_raw_read_preference() {
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.active_note.as_mut().unwrap().rel_path = "discipline/SPRINT.md".into();
        app.discipline_typed = false;

        app.toggle_read_edit_mode();
        assert!(app.editing);
        assert!(!app.discipline_typed);

        app.toggle_read_edit_mode();
        assert!(!app.editing);
        assert!(!app.discipline_typed);
    }

    #[test]
    fn selecting_the_active_read_mode_is_idempotent() {
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.editing = false;
        app.discipline_typed = false;
        app.editor_sel = Some((1, 2));

        app.set_read_edit_mode(crate::ui_breadcrumb::ReadEditMode::Read);

        assert!(!app.discipline_typed);
        assert_eq!(app.editor_sel, Some((1, 2)));
    }

    #[test]
    fn explicit_dark_selection_escapes_the_legacy_light_fallback() {
        let mut config = omninote_core::types::AppConfig {
            theme_preset: omninote_core::types::ThemePreset::ObsidianDark,
            dark_mode: false,
            ..Default::default()
        };
        assert_eq!(
            effective_theme_preset(&config),
            omninote_core::types::ThemePreset::ObsidianLight
        );

        select_theme_preset(&mut config, omninote_core::types::ThemePreset::ObsidianDark);

        assert!(config.dark_mode);
        assert_eq!(
            effective_theme_preset(&config),
            omninote_core::types::ThemePreset::ObsidianDark
        );
        assert_eq!(
            theme_for_config(&config),
            crate::theme::Theme::obsidian_dark()
        );
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
    fn shortcuts_reject_alt_but_tolerate_layout_shift() {
        // Regression (triad rounds 3-4 + probes). egui's consume_shortcut matches
        // logically and ignores extra Alt/Shift, so on Windows/Linux AltGr
        // (= Ctrl+Alt, mapped to command) tripped every COMMAND chord — typing `€`
        // (AltGr+E) fired Cmd+E. consume_app_shortcut rejects Alt (the fix) but
        // TOLERATES Shift, because some layouts need Shift to produce a key (`=` is
        // Shift+0 on QWERTZ, so invoking Cmd+= sends Ctrl+Shift+Equals — a strict
        // exact match would wrongly break it). Drives the real helper.
        use egui::{Event, Key, Modifiers};
        let fires =
            |sc: &egui::KeyboardShortcut, key: Key, mods: Modifiers, repeat: bool| -> bool {
                let ctx = egui::Context::default();
                let mut input = egui::RawInput::default();
                input.events.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat,
                    modifiers: mods,
                });
                let mut hit = false;
                let _ = ctx.run(input, |ctx| {
                    hit = ctx.input_mut(|i| consume_app_shortcut(i, sc));
                });
                hit
            };
        let altgr = Modifiers {
            ctrl: true,
            alt: true,
            command: true,
            ..Default::default()
        };
        let cmd_e = egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::E);
        let cmd_eq = egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::Equals);
        let cmd_shift_j =
            egui::KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::J);

        // Intended chords fire.
        assert!(
            fires(&cmd_e, Key::E, Modifiers::COMMAND, false),
            "Cmd/Ctrl+E"
        );
        assert!(
            fires(
                &cmd_shift_j,
                Key::J,
                Modifiers::COMMAND.plus(Modifiers::SHIFT),
                false
            ),
            "Cmd/Ctrl+Shift+J"
        );
        // Layout Shift tolerated: on QWERTZ `=` is Shift+0, so Cmd+= arrives as
        // Ctrl+Shift+Equals and MUST still fire (the agy round-4 regression).
        assert!(
            fires(
                &cmd_eq,
                Key::Equals,
                Modifiers::COMMAND.plus(Modifiers::SHIFT),
                false
            ),
            "Cmd+= with layout Shift (QWERTZ) must fire"
        );

        // Alt is rejected — the € / Cmd+E bug.
        assert!(
            !fires(&cmd_e, Key::E, altgr, false),
            "AltGr+E (€) must not fire Cmd+E"
        );
        assert!(
            !fires(
                &cmd_e,
                Key::E,
                Modifiers {
                    alt: true,
                    ..Modifiers::COMMAND
                },
                false
            ),
            "Cmd+Alt+E must not fire"
        );
        // A Shift chord still requires its Shift.
        assert!(
            !fires(&cmd_shift_j, Key::J, Modifiers::COMMAND, false),
            "Cmd+J without Shift must not fire the Shift chord"
        );

        // Held repeats fire once. egui derives the `repeat` flag, so drive two
        // presses without a release on one Context: egui marks the second a repeat.
        let ctx = egui::Context::default();
        let press = || {
            let mut input = egui::RawInput::default();
            input.events.push(Event::Key {
                key: Key::E,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            });
            let mut hit = false;
            let _ = ctx.run(input, |ctx| {
                hit = ctx.input_mut(|i| consume_app_shortcut(i, &cmd_e));
            });
            hit
        };
        assert!(press(), "first press fires");
        assert!(!press(), "held repeat must not refire");
    }

    #[test]
    fn gui_shortcuts_route_through_altgr_safe_helper() {
        // Mechanical gate (triad rounds 3-5): every shortcut must route through the
        // AltGr-safe consume_app_shortcut. egui's logical InputState shortcut method
        // matches logically and reintroduces the AltGr collision (typing `€` fires
        // Cmd+E). Recurse the whole crate src/. The needle is the bare call form, so
        // it catches both the method-call and fully-qualified spellings; it is split
        // (and this fn's name avoids it) so the test's own source isn't a false
        // positive. A space before the paren is normalized away by `cargo fmt`, and
        // the AltGr-safe helper's own name does not contain the needle.
        let needle = ["consume", "_shortcut("].concat();
        fn scan(dir: &std::path::Path, needle: &str) {
            for entry in std::fs::read_dir(dir).expect("read dir") {
                let path = entry.expect("dir entry").path();
                if path.is_dir() {
                    scan(&path, needle);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let text = std::fs::read_to_string(&path).expect("read source");
                    assert!(
                        !text.contains(needle),
                        "{}: route shortcuts through consume_app_shortcut, not egui's logical method",
                        path.display()
                    );
                }
            }
        }
        scan(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &needle,
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
    fn select_note_resets_stale_editor_actions() {
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
        app.select_note(&id_a);
        app.editing = true;
        app.editor_sel = Some((5, 10));
        app.slash_menu_pos = Some(5);
        app.queue_editor_action(crate::ui_editor::MdFormat::Bold);
        assert!(app.pending_editor_action.is_some());
        app.select_note(&id_b);
        assert_eq!(app.editor_sel, None);
        assert_eq!(app.slash_menu_pos, None);
        assert!(app.pending_editor_action.is_none());
        assert!(app.active_note.is_some(), "nota B deve ficar ativa");
    }

    #[test]
    fn format_command_in_read_mode_is_dropped_immediately() {
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.editing = false;
        app.queue_editor_action(crate::ui_editor::MdFormat::Bold);
        assert!(app.pending_editor_action.is_none());
    }

    #[test]
    fn format_command_in_typed_discipline_view_is_dropped_immediately() {
        let (mut app, _dir) = test_app();
        let id = app.vault.as_ref().unwrap().notes[0].frontmatter.id.clone();
        app.select_note(&id);
        app.active_note.as_mut().unwrap().rel_path = "discipline/SPRINT.md".into();
        app.editing = true;
        app.discipline_typed = true;

        app.queue_editor_action(crate::ui_editor::MdFormat::Bold);

        assert!(app.pending_editor_action.is_none());
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
