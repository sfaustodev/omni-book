//! Native macOS menu bar — **Tema** (theme switcher) + **Editar** (the same
//! markdown formatting commands the right-click/`/` menus expose) + a minimal
//! **Arquivo**. eframe/egui has no built-in system menu bar
//! (`emilk/egui#3411`, still open upstream); `muda` owns the real `NSApp` menu
//! directly (`Menu::init_for_nsapp`), independent of winit's version, so it
//! coexists safely with our pinned eframe/egui 0.29.
//!
//! macOS-only for this pass (`mod macos` below): attaching to a window on
//! Linux/Windows (`init_for_window`) would need extracting a raw window
//! handle from eframe's `Frame`, an unconfirmed API surface for eframe
//! 0.29 — and the ask was specifically the macOS top menu bar. Linux/Windows
//! get the `stub` module instead (same public API, no-op) so `app.rs` never
//! needs a single `#[cfg]` of its own; they keep the Settings-modal theme
//! picker and the existing right-click/slash formatting menus — no
//! regression, just no native menu bar there yet.
//!
//! Deliberately **not** wired here: Cut/Paste/Undo/Redo. They already work
//! today via keyboard (Cmd+X/C/V/Z — egui's `TextEdit` + `egui-winit`'s
//! built-in `arboard`-backed clipboard handle that for free). Making them
//! *also* clickable needs either OS `PredefinedMenuItem`s (which likely don't
//! reach egui's custom-rendered buffer — no NSTextView responder chain) or a
//! bespoke undo/redo content-snapshot stack — real new scope nobody asked
//! for. Selecionar tudo/Copiar are included (safe, cheap) but deliberately
//! carry **no accelerator**: egui's `TextEdit` already owns Cmd+A/Cmd+C when
//! focused, and double-binding those keys risks a double-fire. Formatting
//! shortcuts stay owned by the focused egui editor; native items are click-only.

/// RGBA bytes whose dimensions are safe to hand to a native menu backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuIconRgba {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

/// Rejects zero-area or malformed images before a platform encoder sees them.
pub fn validated_menu_icon_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Option<MenuIconRgba> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected_len = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(4)?;
    if rgba.len() != expected_len {
        return None;
    }
    Some(MenuIconRgba {
        rgba,
        width,
        height,
    })
}

/// The byte range covering the whole note (used by "Selecionar tudo"). Pure
/// and platform-independent so it's unit-tested on every CI runner, not just
/// macOS. Off macOS the only non-test caller (`macos::pump`) is compiled out,
/// so the bin target sees it as dead there — hence the scoped allow.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn select_all_range(len: usize) -> (usize, usize) {
    (0, len)
}

/// Char-boundary-safe substring extraction for "Copiar" — same snapping
/// technique the editor formatter uses, so a selection ending
/// mid-multibyte-char never panics. Pure and platform-independent; same
/// scoped allow as [`select_all_range`] (caller compiled out off macOS).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn copy_slice(content: &str, sel: (usize, usize)) -> String {
    let mut a = sel.0.min(content.len());
    let mut b = sel.1.min(content.len());
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    while a < content.len() && !content.is_char_boundary(a) {
        a += 1;
    }
    while b < content.len() && !content.is_char_boundary(b) {
        b += 1;
    }
    content[a..b].to_string()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{copy_slice, select_all_range, validated_menu_icon_rgba};
    use crate::app::OmniNoteApp;
    use crate::ui_editor::{editor_actions_for, EditorEntryPoint, MdFormat};
    use muda::accelerator::Accelerator;
    use muda::{
        CheckMenuItem, IconMenuItem, IsMenuItem, Menu, MenuEvent, MenuId, MenuItem,
        PredefinedMenuItem, Submenu,
    };
    use omninote_core::types::ThemePreset;
    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver};

    enum Action {
        Theme(ThemePreset),
        Format(MdFormat),
        SelectAll,
        Copy,
        NewNote,
        Settings,
        Close,
    }

    enum FallibleIconMenuItem {
        Plain(MenuItem),
        Icon(IconMenuItem),
    }

    impl FallibleIconMenuItem {
        fn new(
            text: &str,
            accelerator: Option<Accelerator>,
            raw_icon: Option<(Vec<u8>, u32, u32)>,
        ) -> Self {
            let icon = raw_icon
                .and_then(|(rgba, width, height)| validated_menu_icon_rgba(rgba, width, height))
                .and_then(|data| muda::Icon::from_rgba(data.rgba, data.width, data.height).ok());
            match icon {
                Some(icon) => Self::Icon(IconMenuItem::new(text, true, Some(icon), accelerator)),
                None => Self::Plain(MenuItem::new(text, true, accelerator)),
            }
        }

        fn id(&self) -> &MenuId {
            match self {
                Self::Plain(item) => item.id(),
                Self::Icon(item) => item.id(),
            }
        }

        fn as_menu_item(&self) -> &dyn IsMenuItem {
            match self {
                Self::Plain(item) => item,
                Self::Icon(item) => item,
            }
        }
    }

    pub struct NativeMenu {
        menu_bar: Menu,
        actions: HashMap<MenuId, Action>,
        theme_items: Vec<(ThemePreset, CheckMenuItem)>,
        events: Receiver<MenuEvent>,
    }

    impl NativeMenu {
        /// Build the menu bar and install it as the app-global `NSApp` menu.
        /// `ctx` is cloned into the event handler solely to call
        /// `request_repaint()` — a native menu click doesn't originate from
        /// egui's own input polling, so without this the next frame could be
        /// delayed until unrelated window input woke the event loop back up.
        pub fn build(ctx: &egui::Context, current: ThemePreset) -> Result<Self, String> {
            let (tx, rx) = mpsc::channel::<MenuEvent>();
            let repaint_ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                let _ = tx.send(event);
                repaint_ctx.request_repaint();
            }));

            let mut actions = HashMap::new();
            let menu_bar = Menu::new();

            let app_menu = Submenu::new("OmniNote", true);
            app_menu
                .append_items(&[
                    &PredefinedMenuItem::about(None, None),
                    &PredefinedMenuItem::separator(),
                    &PredefinedMenuItem::quit(None),
                ])
                .map_err(|error| format!("muda: append App submenu items: {error}"))?;
            menu_bar
                .append(&app_menu)
                .map_err(|error| format!("muda: append App menu: {error}"))?;

            menu_bar
                .append(&Self::build_file_menu(&mut actions)?)
                .map_err(|error| format!("muda: append Arquivo menu: {error}"))?;
            menu_bar
                .append(&Self::build_edit_menu(&mut actions)?)
                .map_err(|error| format!("muda: append Editar menu: {error}"))?;
            let (theme_menu, theme_items) = Self::build_theme_menu(&mut actions, current)?;
            menu_bar
                .append(&theme_menu)
                .map_err(|error| format!("muda: append Tema menu: {error}"))?;

            menu_bar.init_for_nsapp();

            Ok(Self {
                menu_bar,
                actions,
                theme_items,
                events: rx,
            })
        }

        // Arquivo — no accelerators: the existing egui-level shortcuts
        // (Cmd+N/,/W, `app.rs`'s `consume_app_shortcut` loop) already own
        // these keys; these items are click-only affordances, not a second
        // owner of the binding.
        fn build_file_menu(actions: &mut HashMap<MenuId, Action>) -> Result<Submenu, String> {
            let file_menu = Submenu::new("Arquivo", true);
            let new_note = MenuItem::new("Nova nota", true, None);
            let settings = MenuItem::new("Configurações", true, None);
            let close = MenuItem::new("Fechar", true, None);
            actions.insert(new_note.id().clone(), Action::NewNote);
            actions.insert(settings.id().clone(), Action::Settings);
            actions.insert(close.id().clone(), Action::Close);
            file_menu
                .append_items(&[
                    &new_note,
                    &settings,
                    &PredefinedMenuItem::separator(),
                    &close,
                ])
                .map_err(|error| format!("muda: append Arquivo items: {error}"))?;
            Ok(file_menu)
        }

        fn build_edit_menu(actions: &mut HashMap<MenuId, Action>) -> Result<Submenu, String> {
            let edit_menu = Submenu::new("Editar", true);

            let select_all = FallibleIconMenuItem::new("Selecionar tudo", None, None);
            let copy = FallibleIconMenuItem::new("Copiar", None, None);
            actions.insert(select_all.id().clone(), Action::SelectAll);
            actions.insert(copy.id().clone(), Action::Copy);
            edit_menu
                .append(select_all.as_menu_item())
                .map_err(|error| format!("muda: append Editar select-all item: {error}"))?;
            edit_menu
                .append(copy.as_menu_item())
                .map_err(|error| format!("muda: append Editar copy item: {error}"))?;
            edit_menu
                .append(&PredefinedMenuItem::separator())
                .map_err(|error| format!("muda: append Editar separator: {error}"))?;

            for format in editor_actions_for(EditorEntryPoint::NativeMenu) {
                if matches!(format, MdFormat::H1 | MdFormat::Math) {
                    edit_menu
                        .append(&PredefinedMenuItem::separator())
                        .map_err(|error| format!("muda: append Editar separator: {error}"))?;
                }
                let item = FallibleIconMenuItem::new(format.label(), None, None);
                actions.insert(item.id().clone(), Action::Format(format));
                edit_menu
                    .append(item.as_menu_item())
                    .map_err(|error| format!("muda: append format item: {error}"))?;
            }

            Ok(edit_menu)
        }

        // Tema — one `CheckMenuItem` per `ThemePreset::all()`, one per line,
        // grouped with separators. muda has no native radio-group for
        // `CheckMenuItem`, so `sync_theme_check` manually unchecks the other
        // 8 on every selection.
        fn build_theme_menu(
            actions: &mut HashMap<MenuId, Action>,
            current: ThemePreset,
        ) -> Result<(Submenu, Vec<(ThemePreset, CheckMenuItem)>), String> {
            let theme_menu = Submenu::new("Tema", true);
            let mut theme_items = Vec::new();
            let groups: [&[ThemePreset]; 6] = [
                &[ThemePreset::ObsidianDark, ThemePreset::ObsidianLight],
                &[ThemePreset::AlmanacLight, ThemePreset::AlmanacDark],
                &[ThemePreset::Blueprint, ThemePreset::BlueprintLight],
                &[ThemePreset::Swiss],
                &[ThemePreset::HighContrast],
                &[ThemePreset::Custom],
            ];
            for (gi, group) in groups.iter().enumerate() {
                if gi > 0 {
                    theme_menu
                        .append(&PredefinedMenuItem::separator())
                        .map_err(|error| format!("muda: append Tema separator: {error}"))?;
                }
                for &preset in *group {
                    let item = CheckMenuItem::new(preset.label(), true, preset == current, None);
                    actions.insert(item.id().clone(), Action::Theme(preset));
                    theme_menu
                        .append(&item)
                        .map_err(|error| format!("muda: append Tema item: {error}"))?;
                    theme_items.push((preset, item));
                }
            }
            Ok((theme_menu, theme_items))
        }

        /// Refresh the Tema checkmarks after a preset change from ANY source —
        /// a native menu click, or the Settings-modal picker.
        pub fn sync_theme_check(&self, current: ThemePreset) {
            for (preset, item) in &self.theme_items {
                item.set_checked(*preset == current);
            }
        }

        /// Drain queued native menu clicks and apply them to `app`. Call once
        /// per frame, before the panels render, so a format click lands in
        /// `pending_editor_action` before `show_edit_panel` consumes it this
        /// same frame (`ui_editor.rs`).
        pub fn pump(&mut self, app: &mut OmniNoteApp, ctx: &egui::Context) {
            while let Ok(event) = self.events.try_recv() {
                let Some(action) = self.actions.get(&event.id) else {
                    continue;
                };
                match action {
                    Action::Theme(preset) => {
                        let preset = *preset;
                        if let Some(v) = &mut app.vault {
                            v.config.theme_preset = preset;
                            v.config.dark_mode = crate::app::theme_for_config(&v.config).dark;
                            crate::app::theme_for_config(&v.config).apply(ctx);
                            let _ = v.save_config();
                        }
                        self.sync_theme_check(preset);
                    }
                    Action::Format(format) => app.queue_editor_action(*format),
                    Action::SelectAll => {
                        if let Some(note) = &app.active_note {
                            app.editor_sel = Some(select_all_range(note.content.len()));
                        }
                    }
                    Action::Copy => {
                        if let (Some(note), Some(sel)) = (&app.active_note, app.editor_sel) {
                            ctx.copy_text(copy_slice(&note.content, sel));
                        }
                    }
                    Action::NewNote => app.show_new = true,
                    Action::Settings => app.show_settings = true,
                    Action::Close => {
                        if app.active_note.is_some() {
                            let _ = app.switch_active(None);
                        }
                    }
                }
            }
        }
    }

    impl Drop for NativeMenu {
        fn drop(&mut self) {
            self.menu_bar.remove_for_nsapp();
            MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        }
    }
}

#[cfg(target_os = "macos")]
pub use macos::NativeMenu;

/// No-op stand-in on platforms other than macOS — same public API as the real
/// `NativeMenu`, so `app.rs` never needs a `#[cfg]` of its own. See the module
/// doc comment for why this doesn't (yet) attach to a window on Linux/Windows.
#[cfg(not(target_os = "macos"))]
mod stub {
    use crate::app::OmniNoteApp;
    use omninote_core::types::ThemePreset;

    pub struct NativeMenu;

    impl NativeMenu {
        pub fn build(_ctx: &egui::Context, _current: ThemePreset) -> Result<Self, String> {
            Ok(Self)
        }
        pub fn sync_theme_check(&self, _current: ThemePreset) {}
        pub fn pump(&mut self, _app: &mut OmniNoteApp, _ctx: &egui::Context) {}
    }
}

#[cfg(not(target_os = "macos"))]
pub use stub::NativeMenu;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_icon_validation_rejects_zero_dimensions_and_empty_rgba() {
        assert!(validated_menu_icon_rgba(Vec::new(), 0, 16).is_none());
        assert!(validated_menu_icon_rgba(Vec::new(), 16, 0).is_none());
        assert!(validated_menu_icon_rgba(Vec::new(), 0, 0).is_none());
    }

    #[test]
    fn menu_icon_validation_accepts_exact_non_empty_rgba() {
        assert!(validated_menu_icon_rgba(vec![255; 2 * 3 * 4], 2, 3).is_some());
    }

    #[test]
    fn menu_icon_validation_rejects_mismatched_rgba_length() {
        assert!(validated_menu_icon_rgba(vec![0; 4], 2, 2).is_none());
        assert!(validated_menu_icon_rgba(vec![0; 16], 1, 1).is_none());
    }

    #[test]
    fn select_all_range_covers_whole_content() {
        assert_eq!(select_all_range(0), (0, 0));
        assert_eq!(select_all_range(42), (0, 42));
    }

    #[test]
    fn copy_slice_extracts_selection() {
        assert_eq!(copy_slice("hello world", (0, 5)), "hello");
        assert_eq!(copy_slice("hello world", (6, 11)), "world");
    }

    #[test]
    fn copy_slice_handles_reversed_and_out_of_range() {
        assert_eq!(copy_slice("hello", (5, 0)), "hello");
        assert_eq!(copy_slice("hello", (2, 999)), "llo");
    }

    #[test]
    fn copy_slice_snaps_to_char_boundaries() {
        // "café" — é is 2 bytes; a selection ending mid-char must not panic and
        // must snap forward to include the whole character.
        let s = "café";
        assert_eq!(copy_slice(s, (0, s.len() - 1)), "café");
    }

    #[test]
    fn copy_slice_empty_selection_is_empty_string() {
        assert_eq!(copy_slice("hello", (2, 2)), "");
    }
}
