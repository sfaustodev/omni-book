//! `OmniNoteApp` — the eframe application: state, the per-frame `update()` loop,
//! font/style setup, vault lifecycle, and the save/select helpers that the sidebar,
//! editor, modals and palette modules build on (each lives in its own file as an
//! `impl OmniNoteApp` block).

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{FontFamily as EguiFamily, FontId, TextStyle};
use omninote_core::types::{AppConfig, FontFamily, Note, NoteType};
use omninote_core::vault::Vault;
use omninote_core::vaults;

use crate::theme::Theme;
use crate::watcher::VaultWatcher;

/// Snippets offered by the editor slash menu (`/` at start of a word).
pub const SLASH_ITEMS: &[(&str, &str)] = &[
    ("Título H1", "# "),
    ("Título H2", "## "),
    ("Título H3", "### "),
    ("Lista", "- "),
    ("Tarefa", "- [ ] "),
    ("Citação", "> "),
    ("Código", "```\n\n```"),
    ("Divisória", "\n---\n"),
    ("Wikilink", "[[]]"),
    ("Negrito", "**texto**"),
    ("Itálico", "*texto*"),
];

/// egui reports cursor offsets as CHARACTER indices; convert to a byte index so it
/// is safe to slice a `String` (note bodies contain accented text and ×/÷/− glyphs).
pub fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

/// A transient corner notification.
pub struct Toast {
    pub msg: String,
    pub until: Instant,
    pub error: bool,
}

/// Confirmable destructive actions.
pub enum Pending {
    DeleteNote(String),
}

pub struct OmniNoteApp {
    pub vault: Option<Vault>,
    pub theme: Theme,

    // Active note (owned clone being edited; synced back on save).
    pub active_id: Option<String>,
    pub active_note: Option<Note>,
    pub editing: bool,
    pub dirty: bool,
    pub last_edit: Option<Instant>,
    pub self_write_until: Option<Instant>,

    // Editor cursor (byte offset into active note content), refreshed each frame.
    pub cursor_byte: usize,

    // Slash menu.
    pub slash_at: Option<usize>,
    pub slash_sel: usize,

    // Sidebar.
    pub filter: String,
    pub focus_filter: bool,
    pub type_filter: Option<NoteType>,
    pub collapsed: HashSet<String>,

    // Modals.
    pub show_new: bool,
    pub new_title: String,
    pub new_type: NoteType,
    pub new_in_folder: Option<PathBuf>,
    pub show_settings: bool,
    pub show_import: bool,
    pub show_new_folder: bool,
    pub new_folder_name: String,
    pub pending: Option<Pending>,
    pub external_change: bool,

    // Search palette (Cmd/Ctrl+K).
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_sel: usize,
    pub focus_palette: bool,

    // Deferred navigation requested by a clicked wikilink.
    pub nav_target: Option<String>,

    pub watcher: Option<VaultWatcher>,
    pub onboarding_done: bool,
    pub toast: Option<Toast>,
}

impl OmniNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut app = Self {
            vault: None,
            theme: Theme::parchment(),
            active_id: None,
            active_note: None,
            editing: false,
            dirty: false,
            last_edit: None,
            self_write_until: None,
            cursor_byte: 0,
            slash_at: None,
            slash_sel: 0,
            filter: String::new(),
            focus_filter: false,
            type_filter: None,
            collapsed: HashSet::new(),
            show_new: false,
            new_title: String::new(),
            new_type: NoteType::Resumo,
            new_in_folder: None,
            show_settings: false,
            show_import: false,
            show_new_folder: false,
            new_folder_name: String::new(),
            pending: None,
            external_change: false,
            palette_open: false,
            palette_query: String::new(),
            palette_sel: 0,
            focus_palette: false,
            nav_target: None,
            watcher: None,
            onboarding_done: false,
            toast: None,
        };

        if let Ok(Some(path)) = vaults::resolve_active(None) {
            app.open_vault(path, &cc.egui_ctx);
        }
        app
    }

    pub fn config(&self) -> AppConfig {
        self.vault
            .as_ref()
            .map(|v| v.config.clone())
            .unwrap_or_default()
    }

    /// Open (or switch to) a vault and start watching it.
    pub fn open_vault(&mut self, path: PathBuf, ctx: &egui::Context) {
        match Vault::open(path) {
            Ok(vault) => {
                self.theme = Theme::from_dark(vault.config.dark_mode);
                self.watcher = VaultWatcher::new(&vault.root, ctx.clone());
                self.active_id = None;
                self.active_note = None;
                self.editing = false;
                self.dirty = false;
                self.onboarding_done = !vault.notes.is_empty();
                self.vault = Some(vault);
            }
            Err(e) => self.toast_error(format!("Falha ao abrir vault: {e}")),
        }
    }

    /// Persist the active note to disk and sync the in-memory copy + wikilink index.
    pub fn save_active(&mut self) {
        let Some(vault) = self.vault.as_mut() else {
            return;
        };
        let Some(note) = self.active_note.clone() else {
            return;
        };
        if let Err(e) = vault.save_note(&note) {
            self.toast_error(format!("Erro ao salvar: {e}"));
            return;
        }
        // Reflect the edit in the notes list and rebuild the resolution index so
        // backlinks/wikilinks stay accurate without a full disk rescan.
        if let Some(slot) = vault
            .notes
            .iter_mut()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
        {
            *slot = note;
        }
        vault.index = omninote_core::resolver::VaultIndex::build(&vault.notes);
        self.dirty = false;
        self.last_edit = None;
        self.self_write_until = Some(Instant::now() + Duration::from_millis(400));
    }

    /// Save any pending edits before changing what's on screen.
    pub fn flush(&mut self) {
        if self.dirty {
            self.save_active();
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
    }

    /// Select a note by frontmatter id (flushes current edits first).
    pub fn select_note(&mut self, id: &str) {
        self.flush();
        let found = self
            .vault
            .as_ref()
            .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == id).cloned());
        if let Some(note) = found {
            self.active_id = Some(note.frontmatter.id.clone());
            self.active_note = Some(note);
            self.editing = false;
            self.dirty = false;
            self.cursor_byte = 0;
            self.slash_at = None;
        }
    }

    /// Reload notes from disk and re-bind the active note by id.
    pub fn reload_and_reselect(&mut self) {
        let keep = self.active_id.clone();
        if let Some(vault) = self.vault.as_mut() {
            vault.reload_notes();
        }
        if let Some(id) = keep {
            let still = self
                .vault
                .as_ref()
                .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == id).cloned());
            match still {
                Some(note) => self.active_note = Some(note),
                None => {
                    self.active_id = None;
                    self.active_note = None;
                }
            }
        }
        self.dirty = false;
    }

    /// Create a note, persist it, and open it in edit mode.
    pub fn create_note_now(&mut self, folder: Option<PathBuf>, title: &str, ty: NoteType) {
        self.flush();
        let title = if title.trim().is_empty() {
            "Nova nota"
        } else {
            title.trim()
        };
        let res = self
            .vault
            .as_mut()
            .map(|v| v.create_note(folder.as_deref(), title, ty));
        match res {
            Some(Ok(note)) => {
                self.active_id = Some(note.frontmatter.id.clone());
                self.active_note = Some(note);
                self.editing = true;
                self.dirty = false;
                self.cursor_byte = 0;
                self.onboarding_done = true;
                self.toast_info("Nota criada");
            }
            Some(Err(e)) => self.toast_error(format!("Erro: {e}")),
            None => {}
        }
    }

    /// Move a note into a folder (None = vault root). Used by drag-and-drop.
    pub fn move_note(&mut self, id: &str, folder: Option<PathBuf>) {
        self.flush();
        let res = self
            .vault
            .as_mut()
            .map(|v| v.move_note_by_id(id, folder.as_deref()));
        match res {
            Some(Ok(())) => {
                self.reload_and_reselect();
                self.toast_info("Nota movida");
            }
            Some(Err(e)) => self.toast_error(format!("Não movida: {e}")),
            None => {}
        }
    }

    pub fn delete_note_now(&mut self, id: &str) {
        let idx = self
            .vault
            .as_ref()
            .and_then(|v| v.notes.iter().position(|n| n.frontmatter.id == id));
        if let (Some(vault), Some(idx)) = (self.vault.as_mut(), idx) {
            match vault.delete_note(idx) {
                Ok(()) => {
                    if self.active_id.as_deref() == Some(id) {
                        self.active_id = None;
                        self.active_note = None;
                    }
                    self.toast_info("Nota apagada");
                }
                Err(e) => self.toast_error(format!("Erro: {e}")),
            }
        }
    }

    pub fn toggle_theme(&mut self) {
        if let Some(vault) = self.vault.as_mut() {
            vault.config.dark_mode = !vault.config.dark_mode;
            self.theme = Theme::from_dark(vault.config.dark_mode);
            let _ = vault.save_config();
        }
    }

    pub fn toast_info(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast {
            msg: msg.into(),
            until: Instant::now() + Duration::from_secs(3),
            error: false,
        });
    }

    pub fn toast_error(&mut self, msg: impl Into<String>) {
        self.toast = Some(Toast {
            msg: msg.into(),
            until: Instant::now() + Duration::from_secs(5),
            error: true,
        });
    }

    fn in_self_write_window(&self) -> bool {
        self.self_write_until.is_some_and(|t| Instant::now() < t)
    }

    /// Drain watcher events; reload on external change (or flag a conflict if dirty).
    fn poll_watcher(&mut self) {
        let changed = self.watcher.as_ref().is_some_and(|w| w.drain_relevant());
        if changed && !self.in_self_write_window() {
            if self.dirty {
                self.external_change = true;
            } else {
                self.reload_and_reselect();
            }
        }
    }

    /// Central keyboard shortcuts (editor-local ones live in the editor module).
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers {
            shift: true,
            ..Modifiers::COMMAND
        };
        let hit = |m: Modifiers, k: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&egui::KeyboardShortcut::new(m, k)))
        };

        if hit(cmd, Key::N) {
            self.new_in_folder = None;
            self.show_new = true;
        }
        if hit(cmd, Key::E) && self.active_note.is_some() {
            self.editing = !self.editing;
        }
        if hit(cmd, Key::K) {
            self.palette_open = true;
            self.focus_palette = true;
            self.palette_sel = 0;
        }
        if hit(cmd, Key::Comma) {
            self.show_settings = true;
        }
        if hit(cmd_shift, Key::D) {
            self.toggle_theme();
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.palette_open = false;
            self.show_new = false;
            self.show_settings = false;
            self.show_import = false;
            self.slash_at = None;
        }
    }

    fn show_toast(&mut self, ctx: &egui::Context) {
        let Some(toast) = self.toast.as_ref() else {
            return;
        };
        if Instant::now() > toast.until {
            self.toast = None;
            return;
        }
        let (msg, error) = (toast.msg.clone(), toast.error);
        let color = if error {
            self.theme.accent
        } else {
            self.theme.dim
        };
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(self.theme.panel)
                    .stroke(egui::Stroke::new(1.0, color))
                    .show(ui, |ui| {
                        ui.colored_label(color, msg);
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_watcher();
        self.handle_shortcuts(ctx);

        // Debounced autosave.
        if self.dirty {
            if let Some(t) = self.last_edit {
                if t.elapsed() > Duration::from_millis(600) {
                    self.save_active();
                }
            }
            ctx.request_repaint_after(Duration::from_millis(300));
        }

        // Apply theme + accessibility styling for this frame.
        apply_style(ctx, &self.config(), &self.theme);

        // Deferred wikilink navigation (set during the previous view render).
        if let Some(target) = self.nav_target.take() {
            self.navigate_wikilink(&target);
        }

        self.show_sidebar(ctx);
        self.show_editor(ctx);
        self.show_modals(ctx);
        self.show_palette(ctx);
        self.show_onboarding(ctx);
        self.show_toast(ctx);
    }
}

/// Register the bundled fonts and define the "display"/"dyslexic" families.
fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jbmono".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
    );
    fonts.font_data.insert(
        "grotesk".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/SpaceGrotesk-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        "dyslexic".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/OpenDyslexic-Regular.otf")),
    );

    // Prefer JetBrains Mono for monospace.
    fonts
        .families
        .entry(EguiFamily::Monospace)
        .or_default()
        .insert(0, "jbmono".to_owned());

    let proportional = fonts
        .families
        .get(&EguiFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let mut display = vec!["grotesk".to_owned()];
    display.extend(proportional.iter().cloned());
    fonts
        .families
        .insert(EguiFamily::Name("display".into()), display);

    let mut dyslexic = vec!["dyslexic".to_owned()];
    dyslexic.extend(proportional.iter().cloned());
    fonts
        .families
        .insert(EguiFamily::Name("dyslexic".into()), dyslexic);

    ctx.set_fonts(fonts);
}

/// Apply per-frame visuals + text styles driven by the accessibility config.
fn apply_style(ctx: &egui::Context, cfg: &AppConfig, theme: &Theme) {
    let body_family = match cfg.font_family {
        FontFamily::Monospace => EguiFamily::Monospace,
        FontFamily::Dyslexic => EguiFamily::Name("dyslexic".into()),
        _ => EguiFamily::Proportional,
    };
    let sz = cfg.font_size.clamp(11.0, 28.0);

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(sz * 0.82, body_family.clone()),
        ),
        (TextStyle::Body, FontId::new(sz, body_family.clone())),
        (TextStyle::Button, FontId::new(sz, body_family.clone())),
        (
            TextStyle::Monospace,
            FontId::new(sz * 0.96, EguiFamily::Monospace),
        ),
        (
            TextStyle::Heading,
            FontId::new(sz * 1.55, EguiFamily::Name("display".into())),
        ),
    ]
    .into();
    let lh = cfg.line_height.clamp(1.0, 2.2);
    style.spacing.item_spacing.y = 3.0 + (lh - 1.0) * 8.0;
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.visuals = theme.visuals();
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slash_items_are_well_formed() {
        assert!(!SLASH_ITEMS.is_empty());
        for (label, snippet) in SLASH_ITEMS {
            assert!(!label.is_empty());
            assert!(!snippet.is_empty());
        }
    }
}
