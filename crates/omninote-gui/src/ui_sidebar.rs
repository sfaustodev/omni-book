use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::{ConfirmAction, NoteType};
use std::path::{Path, PathBuf};

impl OmniNoteApp {
    pub fn show_sidebar(&mut self, ctx: &egui::Context) {
        let theme = self.sidebar_theme();
        let resp = egui::SidePanel::left("sidebar")
            .exact_width(280.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;

                // Header: OMNINOTE wordmark + vault name in dim mono.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("OMNINOTE")
                            .monospace()
                            .strong()
                            .size(15.0)
                            .color(theme.text),
                    );
                    if let Some(v) = &self.vault {
                        let name = v
                            .root
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("vault");
                        ui.label(RichText::new(name).monospace().size(10.0).color(theme.dim));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("⚙")
                            .on_hover_text("Configurações (Cmd+,)")
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                        if ui
                            .small_button("☀/🌙")
                            .on_hover_text("Tema (Cmd+Shift+D)")
                            .clicked()
                        {
                            if let Some(v) = &mut self.vault {
                                crate::app::toggle_light_dark(&mut v.config);
                                crate::app::theme_for_config(&v.config).apply(ctx);
                            }
                        }
                        if ui
                            .small_button("📂")
                            .on_hover_text("Trocar vault")
                            .clicked()
                        {
                            self.pick_vault_with_ctx(ctx);
                        }
                        let rail_enabled = self.central_overlay == crate::app::CentralOverlay::None;
                        if ui
                            .add_enabled(rail_enabled, egui::Button::new("⊟").small())
                            .on_hover_text("Painel direito (backlinks/outline)")
                            .on_disabled_hover_text("Painel direito (indisponível em overlay)")
                            .clicked()
                        {
                            self.toggle_right_rail();
                        }
                    });
                });
                ui.add_space(4.0);

                // Search: terminal-prompt field — a leading `/` accent glyph and a
                // borderless input floating on the void (no framed box).
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(
                        RichText::new("/")
                            .monospace()
                            .color(theme.accent)
                            .size(crate::ui_a11y::scaled(ui, 13.0)),
                    );
                    let search = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .frame(false)
                            .hint_text(RichText::new("buscar… ^K").monospace().color(theme.faint))
                            .desired_width(f32::INFINITY),
                    );
                    // `command_only()` (not `.command`) so AltGr (= Ctrl+Alt)
                    // typing of a `k`-keyed character on intl layouts doesn't
                    // steal editor focus.
                    if ctx.input(|i| i.key_pressed(egui::Key::K) && i.modifiers.command_only()) {
                        search.request_focus();
                    }
                });
                ui.add_space(4.0);

                // Type filters: bare colored mono words, ink from note_type_color;
                // selected = full-brightness ink + a leading `>` (no box, no stroke).
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
                    if tag_filter(ui, &theme, "todos", theme.dim, self.type_filter.is_none()) {
                        self.type_filter = None;
                    }
                    for t in NoteType::all() {
                        let selected = self.type_filter == Some(t);
                        let ink = theme.note_type_color(t);
                        if tag_filter(ui, &theme, t.label(), ink, selected) {
                            self.type_filter = if selected { None } else { Some(t) };
                        }
                    }
                });
                ui.add_space(2.0);
                crate::ui_a11y::section_header(ui, &theme, "notes");

                // Note/folder tree
                egui::ScrollArea::vertical()
                    .id_salt("sidebar_scroll")
                    .show(ui, |ui| {
                        self.show_discipline_section(ui);
                        self.show_folder_tree(ui, PathBuf::new());
                        self.show_notes_in_folder(ui, &PathBuf::new());
                    });

                // Footer: mono action buttons with kbd_hint chips.
                ui.add_space(2.0);
                crate::ui_a11y::section_header(ui, &theme, "actions");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if footer_action(ui, &theme, "+ nota", Some("^N")) {
                        self.show_new = true;
                    }
                    if footer_action(ui, &theme, "+ pasta", None) {
                        if let Some(v) = &mut self.vault {
                            let _ = v.create_folder(None, "Nova pasta");
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if footer_action(ui, &theme, "importar", None) {
                        self.show_import = true;
                    }
                    if footer_action(ui, &theme, "timeline", Some("⇧^H")) {
                        self.toggle_central_overlay(crate::app::CentralOverlay::Timeline);
                    }
                });
            });

        // 1px hairline on the panel's right edge — the terminal frame separating
        // the sidebar from the editor.
        let panel_rect = resp.response.rect;
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("sidebar_edge"),
        ))
        .vline(
            panel_rect.right(),
            panel_rect.y_range(),
            egui::Stroke::new(1.0_f32, theme.border),
        );
    }

    /// Theme for the sidebar chrome, resolved from the active vault config (or the
    /// dark default before a vault is open).
    fn sidebar_theme(&self) -> crate::theme::Theme {
        self.vault
            .as_ref()
            .map(|v| crate::app::theme_for_config(&v.config))
            .unwrap_or_else(crate::theme::Theme::obsidian_dark)
    }

    fn show_folder_tree(&mut self, ui: &mut egui::Ui, parent: PathBuf) {
        let folders: Vec<PathBuf> = if let Some(v) = &self.vault {
            v.list_folders()
                .into_iter()
                .filter(|f| {
                    if parent == PathBuf::new() {
                        f.components().count() == 1
                    } else {
                        f.parent().unwrap_or(Path::new("")) == parent.as_path()
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let mut pending_folder_retype: Option<(PathBuf, NoteType)> = None;
        for folder in folders {
            let name = folder
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let folder_clone = folder.clone();

            let theme = self.sidebar_theme();
            let header =
                egui::CollapsingHeader::new(RichText::new(&name).monospace().color(theme.text))
                    .id_salt(format!("folder_{}", folder.to_string_lossy()))
                    .default_open(true)
                    .icon(move |ui, openness, resp| disclosure_marker(ui, &theme, openness, resp))
                    .show(ui, |ui| {
                        self.show_folder_tree(ui, folder_clone.clone());
                        self.show_notes_in_folder(ui, &folder_clone);
                    });

            header.header_response.context_menu(|ui| {
                if ui.button("📄+ Nova nota aqui").clicked() {
                    // Creating a note here replaces active_note, so flush the
                    // current buffer first. A pending external-change conflict
                    // blocks the flush — bail so the unsaved edits it protects
                    // aren't dropped (the modal stays up for the user to resolve).
                    if self.flush_active() {
                        if let Some(v) = &mut self.vault {
                            let rel = folder.clone();
                            match v.create_note(Some(&rel), "", NoteType::default()) {
                                Ok(note) => {
                                    self.active_note = Some(note);
                                    self.editing = true;
                                    self.dirty = false;
                                }
                                Err(e) => self.error_msg = Some(e),
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.menu_button("🏷 Categoria (pasta toda)", |ui| {
                    for t in NoteType::all() {
                        if ui.button(format!("{} {}", t.icon(), t.label())).clicked() {
                            pending_folder_retype = Some((folder.clone(), t));
                            ui.close_menu();
                        }
                    }
                });
                if ui.button("🗑 Deletar pasta").clicked() {
                    self.confirm_action = Some(ConfirmAction::DeleteFolder(folder.clone()));
                    ui.close_menu();
                }
            });
        }

        if let Some((folder, t)) = pending_folder_retype {
            let res = self
                .vault
                .as_mut()
                .map(|v| v.set_folder_note_type(&folder, t));
            match res {
                Some(Ok(n)) => {
                    if let Some(active) = &mut self.active_note {
                        if active.rel_path.starts_with(&folder)
                            && active.path.extension().and_then(|s| s.to_str()) == Some("md")
                        {
                            active.frontmatter.note_type = t;
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success(format!("{n} nota(s) → {}", t.label()));
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
    }

    fn show_notes_in_folder(&mut self, ui: &mut egui::Ui, folder: &Path) {
        let query_lower = self.query.to_lowercase();
        let type_filter = self.type_filter;
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());

        let notes: Vec<(String, String, NoteType)> = if let Some(v) = &self.vault {
            v.notes
                .iter()
                .filter(|n| {
                    let parent = n.rel_path.parent().unwrap_or(Path::new(""));
                    let in_folder = parent == folder;
                    let matches_q = query_lower.is_empty()
                        || n.title.to_lowercase().contains(&query_lower)
                        || n.content.to_lowercase().contains(&query_lower);
                    let matches_t = type_filter.is_none_or(|t| n.frontmatter.note_type == t);
                    in_folder && matches_q && matches_t
                })
                .map(|n| {
                    (
                        n.frontmatter.id.clone(),
                        n.title.clone(),
                        n.frontmatter.note_type,
                    )
                })
                .collect()
        } else {
            vec![]
        };

        let move_targets: Vec<PathBuf> = self
            .vault
            .as_ref()
            .map(|v| v.list_folders())
            .unwrap_or_default();

        let mut pending_select: Option<String> = None;
        let mut pending_delete: Option<String> = None;
        let mut pending_retype: Option<(String, NoteType)> = None;
        let mut pending_move: Option<(String, Option<PathBuf>)> = None;

        let theme = self.sidebar_theme();

        for (id, title, current_type) in notes {
            let is_active = active_id.as_deref() == Some(&id);
            let ink = theme.note_type_color(current_type);
            // Terminal note row: the `>` prompt + accent left-bar selection and the
            // focus border come from the shared helper; the closure only paints the
            // type-colored marker dot and the (truncated) title. AccessKit label is
            // the bare title so screen readers don't read the marker glyph.
            let (resp, activated) =
                crate::ui_a11y::clickable_row(ui, &theme, &title, is_active, |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(crate::ui_a11y::scaled_text(ui, "•", 11.0).color(ink));
                    let title_color = if is_active { theme.accent } else { theme.text };
                    // `truncate` keeps a long title inside the panel instead of
                    // bleeding under the scrollbar/editor edge.
                    ui.add(
                        egui::Label::new(
                            crate::ui_a11y::scaled_text(ui, &title, 13.0).color(title_color),
                        )
                        .truncate(),
                    );
                });
            if activated {
                pending_select = Some(id.clone());
            }
            resp.context_menu(|ui| {
                if ui.button("📖 Abrir").clicked() {
                    pending_select = Some(id.clone());
                    ui.close_menu();
                }
                ui.menu_button("🏷 Categoria", |ui| {
                    for t in NoteType::all() {
                        let marker = if t == current_type { "● " } else { "   " };
                        if ui
                            .button(format!("{}{} {}", marker, t.icon(), t.label()))
                            .clicked()
                        {
                            pending_retype = Some((id.clone(), t));
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("📁 Mover para", |ui| {
                    if ui.button("⌂ (raiz)").clicked() {
                        pending_move = Some((id.clone(), None));
                        ui.close_menu();
                    }
                    for f in &move_targets {
                        if ui.button(format!("📁 {}", f.to_string_lossy())).clicked() {
                            pending_move = Some((id.clone(), Some(f.clone())));
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button("🗑 Deletar").clicked() {
                    pending_delete = Some(id.clone());
                    ui.close_menu();
                }
            });
        }

        if let Some(id) = pending_select {
            self.select_note(&id);
        }
        if let Some((id, t)) = pending_retype {
            let res = self.vault.as_mut().map(|v| v.set_note_type(&id, t));
            match res {
                Some(Ok(())) => {
                    if let Some(active) = &mut self.active_note {
                        if active.frontmatter.id == id {
                            active.frontmatter.note_type = t;
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success(format!("Categoria → {}", t.label()));
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
        if let Some((id, dest)) = pending_move {
            // Moving the ACTIVE note re-syncs `active_note` from a fresh on-disk
            // copy below, which would clobber any unsaved buffer edits. Flush them
            // to disk first so the move carries the latest content; if a pending
            // external-change conflict blocks the flush, skip the move so the
            // edits and the conflict modal survive.
            let moves_active = self
                .active_note
                .as_ref()
                .is_some_and(|n| n.frontmatter.id == id);
            let flush_ok = !moves_active || self.flush_active();
            let res = if flush_ok {
                self.vault
                    .as_mut()
                    .map(|v| v.move_note_by_id(&id, dest.as_deref()))
            } else {
                None
            };
            match res {
                Some(Ok(())) => {
                    if let Some(active) = &mut self.active_note {
                        if active.frontmatter.id == id {
                            if let Some(v) = &self.vault {
                                if let Some(fresh) =
                                    v.notes.iter().find(|n| n.frontmatter.id == id).cloned()
                                {
                                    *active = fresh;
                                }
                            }
                        }
                    }
                    self.self_write_until =
                        std::time::Instant::now() + std::time::Duration::from_millis(400);
                    self.toast_success("Nota movida");
                }
                Some(Err(e)) => self.toast_error(e),
                None => {}
            }
        }
        if let Some(id) = pending_delete {
            self.confirm_action = Some(ConfirmAction::DeleteNote(id));
        }
    }
}

/// A bare-word type filter (`resumo`, `codigo`). `ink` colors the word (from
/// `note_type_color`); the SELECTED filter is marked by full-brightness ink + a
/// leading `>` prompt, the unselected ones are dimmed — NO box, NO stroke, the
/// void look carries structure through color and the marker alone. Keyboard focus
/// brightens the word to `accent` (still no rectangle). Returns true when clicked.
/// Focusable/keyboard-activatable so the filter is reachable by Tab.
fn tag_filter(
    ui: &mut egui::Ui,
    theme: &crate::theme::Theme,
    label: &str,
    ink: egui::Color32,
    selected: bool,
) -> bool {
    // A leading `>` marks the active filter; unselected words reserve the same
    // width with a space so the labels don't shift sideways when selection moves.
    let glyph = if selected { '>' } else { ' ' };
    let font = egui::FontId::monospace(crate::ui_a11y::scaled(ui, 11.0));
    let galley = ui.painter().layout_no_wrap(
        format!("{glyph}{label}"),
        font,
        egui::Color32::PLACEHOLDER, // recolored per-state at paint time below
    );
    let (rect, resp) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    // Color decided after interaction: hover/focus brighten the word to the accent
    // (the void-look focus signal — no outline box); otherwise full ink when active,
    // dimmed when resting.
    let color = if resp.hovered() || resp.has_focus() {
        theme.accent
    } else if selected {
        ink
    } else {
        ink.gamma_multiply(0.55)
    };
    ui.painter().galley(rect.min, galley, color);
    resp.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
    });
    let kbd = resp.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    resp.clicked() || kbd
}

/// A footer action: a bare mono `Button` (borderless — `button_frame: false` in
/// `Theme::apply`, so it renders as green text on the void, not a framed box) plus
/// an optional `kbd_hint` of bracketed keys. Returns true when clicked.
fn footer_action(
    ui: &mut egui::Ui,
    theme: &crate::theme::Theme,
    label: &str,
    keys: Option<&str>,
) -> bool {
    let clicked = ui
        .button(
            RichText::new(label)
                .monospace()
                .size(crate::ui_a11y::scaled(ui, 12.0)),
        )
        .clicked();
    if let Some(k) = keys {
        crate::ui_a11y::kbd_hint(ui, theme, k);
    }
    clicked
}

/// Disclosure marker for a folder `CollapsingHeader` — `v` when open, `>` when
/// closed, in `dim` mono. Replaces egui's default triangle so folders speak the
/// same terminal language as the note rows. `openness` is the 0→1 animation
/// progress; the glyph flips at the halfway point.
fn disclosure_marker(
    ui: &mut egui::Ui,
    theme: &crate::theme::Theme,
    openness: f32,
    resp: &egui::Response,
) {
    let glyph = if openness > 0.5 { "v" } else { ">" };
    let color = if resp.hovered() {
        theme.accent
    } else {
        theme.dim
    };
    let rect = resp.rect;
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::monospace(crate::ui_a11y::scaled(ui, 12.0)),
        color,
    );
}
