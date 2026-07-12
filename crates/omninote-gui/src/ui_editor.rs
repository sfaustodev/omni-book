use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::NoteType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MdFormat {
    Bold,
    Italic,
    Strike,
    InlineCode,
    H1,
    H2,
    H3,
    Bullet,
    Numbered,
    Todo,
    Quote,
    Link,
    CodeBlock,
    Wikilink,
    Divider,
    Math,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorEntryPoint {
    #[cfg(any(target_os = "macos", test))]
    NativeMenu,
    SlashMenu,
    CommandPalette,
    UiButton,
    Keyboard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditTarget {
    Selection(usize, usize),
    Cursor(usize),
    Slash(usize),
}

impl EditTarget {
    fn selection_hint(self) -> (usize, usize) {
        match self {
            Self::Selection(a, b) => (a, b),
            Self::Cursor(pos) => (pos, pos),
            Self::Slash(pos) => {
                let cursor = pos.saturating_add(1);
                (cursor, cursor)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditResult {
    pub content: String,
    pub selection: (usize, usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingEditorAction {
    note_id: String,
    format: MdFormat,
    target: EditTarget,
}

impl MdFormat {
    pub const ALL: [Self; 16] = [
        Self::Bold,
        Self::Italic,
        Self::Strike,
        Self::InlineCode,
        Self::CodeBlock,
        Self::H1,
        Self::H2,
        Self::H3,
        Self::Bullet,
        Self::Numbered,
        Self::Todo,
        Self::Quote,
        Self::Link,
        Self::Wikilink,
        Self::Divider,
        Self::Math,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            MdFormat::Bold => "𝐁  Negrito",
            MdFormat::Italic => "𝐼  Itálico",
            MdFormat::Strike => "S̶  Tachado",
            MdFormat::InlineCode => "‹›  Código",
            MdFormat::Link => "🔗  Link",
            MdFormat::CodeBlock => "▦  Bloco de código",
            MdFormat::H1 => "H1  Título",
            MdFormat::H2 => "H2  Subtítulo",
            MdFormat::H3 => "H3  Seção",
            MdFormat::Bullet => "•  Lista",
            MdFormat::Numbered => "1.  Lista numerada",
            MdFormat::Todo => "☐  Tarefa",
            MdFormat::Quote => "❝  Citação",
            MdFormat::Wikilink => "⟦⟧  Wikilink",
            MdFormat::Divider => "―  Divisor",
            MdFormat::Math => "∑  Avaliar linha",
        }
    }

    pub fn supports(self, entrypoint: EditorEntryPoint) -> bool {
        match entrypoint {
            #[cfg(any(target_os = "macos", test))]
            EditorEntryPoint::NativeMenu => true,
            EditorEntryPoint::CommandPalette | EditorEntryPoint::UiButton => true,
            EditorEntryPoint::SlashMenu => self != Self::Math,
            EditorEntryPoint::Keyboard => {
                matches!(self, Self::Bold | Self::Italic | Self::Math)
            }
        }
    }

    fn is_line_prefix(self) -> bool {
        matches!(
            self,
            MdFormat::H1
                | MdFormat::H2
                | MdFormat::H3
                | MdFormat::Bullet
                | MdFormat::Numbered
                | MdFormat::Todo
                | MdFormat::Quote
        )
    }
}

fn normalized_byte_range(content: &str, sel: (usize, usize)) -> (usize, usize) {
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
    (a, b)
}

fn char_index_to_byte(content: &str, char_index: usize) -> usize {
    content
        .char_indices()
        .nth(char_index)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(content.len())
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&ch| ch == marker).count();
    (count >= 3).then_some((marker, count))
}

fn inside_code_fence(content: &str, byte_pos: usize) -> bool {
    let (pos, _) = normalized_byte_range(content, (byte_pos, byte_pos));
    let mut open: Option<(char, usize)> = None;
    for line in content[..pos].split_inclusive('\n') {
        let Some((marker, count)) = fence_marker(line) else {
            continue;
        };
        match open {
            Some((open_marker, open_count)) if marker == open_marker && count >= open_count => {
                open = None;
            }
            None => open = Some((marker, count)),
            _ => {}
        }
    }
    open.is_some()
}

fn wrapping_markers(format: MdFormat) -> Option<(&'static str, &'static str)> {
    match format {
        MdFormat::Bold => Some(("**", "**")),
        MdFormat::Italic => Some(("_", "_")),
        MdFormat::Strike => Some(("~~", "~~")),
        MdFormat::InlineCode => Some(("`", "`")),
        MdFormat::CodeBlock => Some(("```\n", "\n```")),
        MdFormat::Link => Some(("[", "](url)")),
        MdFormat::Wikilink => Some(("[[", "]]")),
        _ => None,
    }
}

/// Applies one editor action at a validated byte target and returns the new
/// content plus the byte selection to restore. Invalid or stale targets are
/// no-ops, so UI adapters never call `replace_range` directly.
pub fn apply_editor_action(
    content: &str,
    target: EditTarget,
    format: MdFormat,
) -> Option<EditResult> {
    if matches!(target, EditTarget::Slash(_)) && !format.supports(EditorEntryPoint::SlashMenu) {
        return None;
    }

    let (working, sel) = match target {
        EditTarget::Slash(pos) => {
            if pos >= content.len()
                || !content.is_char_boundary(pos)
                || content.as_bytes().get(pos).copied() != Some(b'/')
            {
                return None;
            }
            let mut working = content.to_string();
            working.replace_range(pos..pos + 1, "");
            (working, (pos, pos))
        }
        EditTarget::Selection(a, b) => {
            (content.to_string(), normalized_byte_range(content, (a, b)))
        }
        EditTarget::Cursor(pos) => {
            let pos = normalized_byte_range(content, (pos, pos)).0;
            (content.to_string(), (pos, pos))
        }
    };
    let (a, b) = normalized_byte_range(&working, sel);

    if format == MdFormat::Math {
        let (new_line, start, end) = omninote_core::autoformat::try_math_substitute(&working, b)?;
        let mut out = working;
        out.replace_range(start..end, &new_line);
        let cursor = start + new_line.len();
        return Some(EditResult {
            content: out,
            selection: (cursor, cursor),
        });
    }

    if format == MdFormat::CodeBlock && inside_code_fence(&working, a) {
        return None;
    }

    if format.is_line_prefix() {
        let prefix = match format {
            MdFormat::H1 => "# ",
            MdFormat::H2 => "## ",
            MdFormat::H3 => "### ",
            MdFormat::Bullet => "- ",
            MdFormat::Numbered => "1. ",
            MdFormat::Todo => "- [ ] ",
            MdFormat::Quote => "> ",
            _ => unreachable!(),
        };
        // First line start = byte after the last '\n' before `a` (or 0).
        let first = working[..a].rfind('\n').map(|i| i + 1).unwrap_or(0);
        // Collect every line-start offset within [first, b]: `first`, plus each
        // index after a '\n' that falls before `b`.
        let mut starts = vec![first];
        for (i, ch) in working.char_indices() {
            // `i + 1 < b` (not `i < b`): a selection ending right after a '\n'
            // must not prefix the empty line that newline opens. CAD-25b Slice 4.
            if ch == '\n' && i + 1 > first && i + 1 < b {
                starts.push(i + 1);
            }
        }
        starts.sort_unstable();
        starts.dedup();
        let shift_a = starts.iter().filter(|&&start| start <= a).count() * prefix.len();
        let shift_b = starts.iter().filter(|&&start| start <= b).count() * prefix.len();
        let mut out = working;
        for &s in starts.iter().rev() {
            out.insert_str(s, prefix);
        }
        return Some(EditResult {
            content: out,
            selection: (a + shift_a, b + shift_b),
        });
    }

    if format == MdFormat::Divider {
        let line_start = working[..a].rfind('\n').map(|index| index + 1).unwrap_or(0);
        let mut out = working;
        out.insert_str(line_start, "---\n");
        return Some(EditResult {
            content: out,
            selection: (a + 4, b + 4),
        });
    }

    let (pre, post) = wrapping_markers(format)?;
    let mut out = String::with_capacity(working.len() + pre.len() + post.len());
    out.push_str(&working[..a]);
    out.push_str(pre);
    out.push_str(&working[a..b]);
    out.push_str(post);
    out.push_str(&working[b..]);
    Some(EditResult {
        content: out,
        selection: (a + pre.len(), b + pre.len()),
    })
}

type EditorSnapshot = (egui::text::CCursorRange, String);

fn editor_snapshot(content: &str, selection: (usize, usize)) -> EditorSnapshot {
    let (a, b) = normalized_byte_range(content, selection);
    let a_chars = content[..a].chars().count();
    let b_chars = content[..b].chars().count();
    (
        egui::text::CCursorRange::two(
            egui::text::CCursor::new(a_chars),
            egui::text::CCursor::new(b_chars),
        ),
        content.to_string(),
    )
}

fn record_programmatic_edit(
    undoer: &mut egui::util::undoer::Undoer<EditorSnapshot>,
    before: EditorSnapshot,
    after: EditorSnapshot,
) {
    undoer.add_undo(&before);
    undoer.feed_state(0.0, &after);
    undoer.add_undo(&after);
}

pub(crate) fn content_editor_active_state(
    editing: bool,
    has_active_note: bool,
    overlay: crate::app::CentralOverlay,
    typed_view_active: bool,
) -> bool {
    editing && has_active_note && overlay == crate::app::CentralOverlay::None && !typed_view_active
}

fn content_editor_id(note_id: &str) -> egui::Id {
    egui::Id::new(("note_content", note_id))
}

/// v0.8 — items shown in the slash menu when user types `/` at start of a line.
/// Returns (label, snippet). Snippet replaces the `/` character.
pub fn editor_actions_for(entrypoint: EditorEntryPoint) -> Vec<MdFormat> {
    MdFormat::ALL
        .into_iter()
        .filter(|format| format.supports(entrypoint))
        .collect()
}

fn slash_menu_items() -> Vec<MdFormat> {
    editor_actions_for(EditorEntryPoint::SlashMenu)
}

impl OmniNoteApp {
    pub(crate) fn content_editor_active(&self) -> bool {
        let typed_view_active = self.discipline_typed
            && self.active_note.as_ref().is_some_and(|note| {
                crate::ui_discipline::has_typed_discipline_view(&note.rel_path)
            });
        content_editor_active_state(
            self.editing,
            self.active_note.is_some(),
            self.central_overlay,
            typed_view_active,
        )
    }

    pub(crate) fn queue_editor_action(&mut self, format: MdFormat) {
        if !self.content_editor_active() {
            self.pending_editor_action = None;
            return;
        }
        let Some(note) = self.active_note.as_ref() else {
            self.pending_editor_action = None;
            return;
        };
        let target = self
            .editor_sel
            .map(|(a, b)| EditTarget::Selection(a, b))
            .unwrap_or(EditTarget::Cursor(note.content.len()));
        self.pending_editor_action = Some(PendingEditorAction {
            note_id: note.frontmatter.id.clone(),
            format,
            target,
        });
    }

    pub(crate) fn clear_editor_transients(&mut self) {
        self.editor_sel = None;
        self.slash_menu_pos = None;
        self.pending_editor_action = None;
    }

    pub fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_note.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        let new_shortcut =
                            crate::ui_a11y::command_shortcut(ctx, egui::Key::N, false);
                        let search_shortcut =
                            crate::ui_a11y::command_shortcut(ctx, egui::Key::K, false);
                        let settings_shortcut =
                            crate::ui_a11y::command_shortcut(ctx, egui::Key::Comma, false);
                        ui.add_space(100.0);
                        ui.label(RichText::new("📓 OmniNote").size(24.0).weak());
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(format!("{new_shortcut}  Nova nota"))
                                .size(12.0)
                                .weak(),
                        );
                        ui.label(
                            RichText::new(format!("{search_shortcut}  Buscar"))
                                .size(12.0)
                                .weak(),
                        );
                        ui.label(
                            RichText::new(format!("{settings_shortcut}  Configurações"))
                                .size(12.0)
                                .weak(),
                        );
                    });
                });
                return;
            }

            // Tab strip + breadcrumb (CAD-25 Slice 2) replace the old sticky header.
            self.show_tab_strip(ui);
            self.show_breadcrumb(ui);

            // Typed-view fork (Slice 5): a discipline file renders structured
            // unless the Typed↔Raw toggle forced Raw. Clone the note to satisfy the
            // borrow checker — the renderers read it while borrowing `&mut self`.
            // Respect show_discipline_typed's bool: JIRA/NOTION return false (their
            // structured view is the Tickets panel, not a per-note fork), so they
            // fall through to the generic markdown body instead of a blank editor.
            if let Some(note) = self.active_note.clone() {
                if !self.editing && self.discipline_typed && self.show_discipline_typed(ui, &note) {
                    self.pending_editor_action = None;
                    return;
                }
            }

            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .show(ui, |ui| {
                    if self.editing {
                        self.show_edit_panel(ui);
                    } else {
                        self.show_view_panel(ui);
                    }
                });
        });
    }

    fn show_edit_panel(&mut self, ui: &mut egui::Ui) {
        let queued_action = self.pending_editor_action.take();
        // Resolve the note-type ink before the mutable borrow of `active_note`
        // below — the "Tipo:" label is tinted with it (the per-type content hue).
        let type_color = self.active_note.as_ref().map(|n| {
            let theme = self
                .vault
                .as_ref()
                .map(|v| crate::app::theme_for_config(&v.config))
                .unwrap_or_else(crate::theme::Theme::obsidian_dark);
            theme.note_type_color(n.frontmatter.note_type)
        });
        let note = match self.active_note.as_mut() {
            Some(n) => n,
            None => return,
        };
        let type_color = type_color.unwrap_or(egui::Color32::GRAY);

        // Title
        if ui
            .add(
                egui::TextEdit::singleline(&mut note.title)
                    .font(egui::TextStyle::Heading)
                    .hint_text("Título da nota")
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.dirty = true;
        }

        // Metadata row
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tipo:").color(type_color));
            let current = note.frontmatter.note_type;
            egui::ComboBox::from_id_salt("note_type_combo")
                .selected_text(format!("{} {}", current.icon(), current.label()))
                .show_ui(ui, |ui| {
                    for t in NoteType::all() {
                        if ui
                            .selectable_label(
                                note.frontmatter.note_type == t,
                                format!("{} {}", t.icon(), t.label()),
                            )
                            .clicked()
                        {
                            note.frontmatter.note_type = t;
                            self.dirty = true;
                        }
                    }
                });

            ui.label("Tags:");
            let tags_str = note.frontmatter.tags.join(", ");
            let mut tags_edit = tags_str.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut tags_edit)
                        .hint_text("rust, prog, ...")
                        .desired_width(160.0),
                )
                .changed()
                && tags_edit != tags_str
            {
                note.frontmatter.tags = tags_edit
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.dirty = true;
            }
        });

        // Citation-only fields
        if note.frontmatter.note_type == NoteType::Citacao {
            ui.horizontal(|ui| {
                ui.label("Fonte:");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut note.frontmatter.source)
                            .desired_width(150.0),
                    )
                    .changed()
                {
                    self.dirty = true;
                }
                ui.label("URL:");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut note.frontmatter.source_link)
                            .desired_width(200.0),
                    )
                    .changed()
                {
                    self.dirty = true;
                }
            });
        }
        ui.separator();

        // Content editor — use show() to access cursor_range for math substitution
        let editor_id = content_editor_id(&note.frontmatter.id);
        let output = egui::TextEdit::multiline(&mut note.content)
            .id(editor_id)
            .code_editor()
            .desired_rows(30)
            .desired_width(ui.available_width())
            .hint_text("Escreva em markdown...")
            .show(ui);
        let mut editor_state = output.state.clone();

        if output.response.changed() {
            self.dirty = true;
        }

        // Cmd+= math substitution on current line
        // Extract cursor pos before output is dropped (output holds &mut note.content).
        // `ccursor.index` is a CHAR index; autoformat and the slash menu below index
        // `content` by BYTE, so convert once here — a char index used as a byte offset
        // corrupts/erases text (and can panic in replace_range) on non-ASCII lines.
        let cursor_pos = output
            .cursor_range
            .map(|r| char_index_to_byte(&note.content, r.primary.ccursor.index));
        let has_focus = output.response.has_focus();
        // Selection (byte range) for the format menu. cursor_range carries CHAR
        // indices; convert to bytes like the math/slash code does below.
        let sel = output.cursor_range.map(|r| {
            let a = char_index_to_byte(&note.content, r.primary.ccursor.index);
            let b = char_index_to_byte(&note.content, r.secondary.ccursor.index);
            (a.min(b), a.max(b))
        });
        // Seed from a native "Editar" menu click (native_menu.rs) — it acts on
        // `editor_sel` exactly like a right-click pick below, since both steal
        // focus from the editor the same way.
        let fallback_target = sel
            .or(self.editor_sel)
            .map(|(a, b)| EditTarget::Selection(a, b))
            .unwrap_or(EditTarget::Cursor(note.content.len()));
        let mut pending_action = queued_action.and_then(|pending| {
            (pending.note_id == note.frontmatter.id).then_some((pending.format, pending.target))
        });
        output.response.context_menu(|ui| {
            ui.label(egui::RichText::new("Formatar").size(10.0).weak());
            ui.separator();
            for fmt in editor_actions_for(EditorEntryPoint::UiButton) {
                if ui.button(fmt.label()).clicked() {
                    pending_action = Some((fmt, fallback_target));
                    ui.close_menu();
                }
            }
        });
        drop(output);

        // Persist the selection only while the editor actually reported one — when
        // the context menu opens it steals focus and cursor_range goes None, so
        // keeping the last value lets the chosen format act on the prior selection.
        if let Some(s) = sel {
            self.editor_sel = Some(s);
        }

        for format in editor_actions_for(EditorEntryPoint::Keyboard) {
            let key = match format {
                MdFormat::Bold => egui::Key::B,
                MdFormat::Italic => egui::Key::I,
                MdFormat::Math => egui::Key::Equals,
                _ => continue,
            };
            let shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key);
            if has_focus && ui.input_mut(|i| crate::app::consume_app_shortcut(i, &shortcut)) {
                let target = if format == MdFormat::Math {
                    EditTarget::Cursor(cursor_pos.unwrap_or(note.content.len()))
                } else {
                    fallback_target
                };
                pending_action = Some((format, target));
            }
        }

        // v0.8 — Slash menu: detect "/" at start of a line
        if has_focus {
            if let Some(pos) = cursor_pos {
                let bytes = note.content.as_bytes();
                if pos > 0 && bytes[pos - 1] == b'/' {
                    let at_line_start = pos == 1 || bytes[pos - 2] == b'\n';
                    if at_line_start && self.slash_menu_pos != Some(pos - 1) {
                        self.slash_menu_pos = Some(pos - 1);
                    }
                } else if let Some(slash_at) = self.slash_menu_pos {
                    // Close if cursor moved away from the slash position area
                    let still_valid = pos >= slash_at
                        && pos <= note.content.len()
                        && bytes.get(slash_at).copied() == Some(b'/');
                    if !still_valid {
                        self.slash_menu_pos = None;
                    }
                }
            }
        }

        // Esc closes the slash menu
        if self.slash_menu_pos.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.slash_menu_pos = None;
        }

        // Render slash menu popup
        let mut pending_slash_action: Option<(usize, MdFormat)> = None;
        if let Some(slash_at) = self.slash_menu_pos {
            egui::Window::new("Inserir bloco")
                .id(egui::Id::new("slash_menu"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
                .show(ui.ctx(), |ui| {
                    ui.label(
                        egui::RichText::new("/ no início da linha — Esc fecha")
                            .size(10.0)
                            .weak(),
                    );
                    ui.separator();
                    for format in slash_menu_items() {
                        if ui.button(format.label()).clicked() {
                            pending_slash_action = Some((slash_at, format));
                        }
                    }
                });
        }
        if let Some((slash_at, format)) = pending_slash_action {
            pending_action = Some((format, EditTarget::Slash(slash_at)));
            self.slash_menu_pos = None;
        }

        let attach_clicked = ui
            .horizontal(|ui| ui.button("📎 Anexar arquivo").clicked())
            .inner;
        if attach_clicked {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                if let Some(vault) = &self.vault {
                    match vault.import_attachment(&path) {
                        Ok(name) => {
                            note.content.push_str(&format!("\n![[{}]]", name));
                            note.frontmatter.attachments.push(name);
                            self.dirty = true;
                        }
                        Err(error) => self.error_msg = Some(error),
                    }
                }
            }
        }

        if let Some((format, target)) = pending_action {
            let before_content = note.content.clone();
            if let Some(result) = apply_editor_action(&before_content, target, format) {
                let before = editor_snapshot(&before_content, target.selection_hint());
                let after = editor_snapshot(&result.content, result.selection);
                let mut undoer = editor_state.undoer();
                record_programmatic_edit(&mut undoer, before, after.clone());
                editor_state.set_undoer(undoer);
                editor_state.cursor.set_char_range(Some(after.0));
                editor_state.store(ui.ctx(), editor_id);
                note.content = result.content;
                self.editor_sel = Some(result.selection);
                self.dirty = true;
            }
        }
    }

    fn show_view_panel(&mut self, ui: &mut egui::Ui) {
        let note = match self.active_note.clone() {
            Some(n) => n,
            None => return,
        };

        ui.heading(&note.title);

        if !note.frontmatter.tags.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for tag in &note.frontmatter.tags {
                    if ui.link(format!("#{}", tag)).clicked() {
                        self.query = tag.clone();
                    }
                }
            });
        }

        if note.frontmatter.note_type == NoteType::Citacao && !note.frontmatter.source.is_empty() {
            ui.horizontal(|ui| {
                ui.label("📖");
                ui.label(&note.frontmatter.source);
                if !note.frontmatter.source_link.is_empty() {
                    ui.hyperlink_to("🔗 link", &note.frontmatter.source_link);
                }
            });
        }
        ui.separator();

        // Inline renderer (CAD-25 Slice 3a/3b): wikilinks/embeds/#tags rendered in
        // the text flow, with hover previews. Resolution is delegated to the core
        // VaultIndex (closure resolves target → note, returning title + excerpt);
        // md_render doesn't reimplement resolution.
        let action = {
            let note_res = |target: &str, heading: Option<&str>| {
                let v = self.vault.as_ref()?;
                let rel = v.index.resolve(target)?;
                let n = v.notes.iter().find(|n| &n.rel_path == rel)?;
                // `![[Note#H]]` previews just that section; plain links the head.
                let excerpt = match heading {
                    Some(h) => omninote_core::wikilinks::section_under_heading(&n.content, h)
                        .map(|s| crate::md_render::preview_excerpt(&s))
                        .unwrap_or_default(),
                    None => crate::md_render::preview_excerpt(&n.content),
                };
                Some(crate::md_render::LinkPreview {
                    title: n.title.clone(),
                    excerpt,
                })
            };
            let asset_res = |filename: &str| {
                // attachment_path validates against traversal (CWE-22) + confirms
                // the file is inside the vault's _attachments dir.
                let v = self.vault.as_ref()?;
                let path = v.attachment_path(filename)?;
                Some(format!("file://{}", path.to_string_lossy()))
            };
            let resolvers = crate::md_render::Resolvers {
                note: &note_res,
                asset_uri: &asset_res,
            };
            crate::md_render::render_body(ui, &mut self.md_cache, &note.content, &resolvers)
        };
        match action {
            Some(crate::md_render::MdAction::Navigate(target)) => {
                self.select_note_by_target(&target);
            }
            Some(crate::md_render::MdAction::FilterTag(tag)) => self.query = tag,
            None => {}
        }

        // Backlinks moved to the right rail (ui_right_rail.rs), which resolves
        // them target-side via the core index instead of a substring match.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formatted(content: &str, selection: (usize, usize), format: MdFormat) -> String {
        apply_editor_action(
            content,
            EditTarget::Selection(selection.0, selection.1),
            format,
        )
        .map(|result| result.content)
        .unwrap_or_else(|| content.to_owned())
    }

    #[test]
    fn formatting_entrypoint_matrix_is_explicit() {
        use EditorEntryPoint::*;
        for action in MdFormat::ALL {
            for entrypoint in [NativeMenu, SlashMenu, CommandPalette, UiButton, Keyboard] {
                let expected = match entrypoint {
                    NativeMenu | CommandPalette | UiButton => true,
                    SlashMenu => action != MdFormat::Math,
                    Keyboard => {
                        matches!(action, MdFormat::Bold | MdFormat::Italic | MdFormat::Math)
                    }
                };
                assert_eq!(
                    action.supports(entrypoint),
                    expected,
                    "routing cell {action:?} x {entrypoint:?}"
                );
            }
        }
    }

    #[test]
    fn formatting_semantics_are_exact_for_every_action() {
        use MdFormat::*;
        let cases = [
            (Bold, "**word**", (2, 6)),
            (Italic, "_word_", (1, 5)),
            (Strike, "~~word~~", (2, 6)),
            (InlineCode, "`word`", (1, 5)),
            (H1, "# word", (2, 6)),
            (H2, "## word", (3, 7)),
            (H3, "### word", (4, 8)),
            (Bullet, "- word", (2, 6)),
            (Numbered, "1. word", (3, 7)),
            (Todo, "- [ ] word", (6, 10)),
            (Quote, "> word", (2, 6)),
            (Link, "[word](url)", (1, 5)),
            (CodeBlock, "```\nword\n```", (4, 8)),
            (Wikilink, "[[word]]", (2, 6)),
            (Divider, "---\nword", (4, 8)),
        ];
        for (action, expected_content, expected_selection) in cases {
            let result = apply_editor_action("word", EditTarget::Selection(0, 4), action)
                .unwrap_or_else(|| panic!("{action:?} should change canonical selection"));
            assert_eq!(result.content, expected_content, "content for {action:?}");
            assert_eq!(
                result.selection, expected_selection,
                "selection for {action:?}"
            );
        }

        let math = apply_editor_action("2+2=", EditTarget::Cursor(4), Math)
            .expect("math action should substitute a complete expression");
        assert_eq!(math.content, "2+2= 4");
        assert_eq!(math.selection, (6, 6));
    }

    #[test]
    fn formatting_gauntlet_covers_every_action_entrypoint_fixture_cell() {
        use EditorEntryPoint::*;
        let fixtures = [
            ("empty-note", "", EditTarget::Selection(0, 0), "/", 0),
            ("cursor-zero", "abc", EditTarget::Cursor(0), "/abc", 0),
            ("cursor-eof", "abc", EditTarget::Cursor(3), "abc\n/", 4),
            (
                "empty-selection",
                "abc",
                EditTarget::Selection(1, 1),
                "a\n/bc",
                2,
            ),
            (
                "multi-line-selection",
                "a\nb",
                EditTarget::Selection(0, 3),
                "/a\nb",
                0,
            ),
            (
                "multibyte-mid-codepoint",
                "🙂 café ação",
                EditTarget::Selection(1, 8),
                "🙂\n/café ação",
                5,
            ),
            (
                "inside-existing-code-block",
                "```\ncode\n```",
                EditTarget::Selection(5, 8),
                "```\n/code\n```",
                4,
            ),
            (
                "valid-math-at-eof",
                "2+2=",
                EditTarget::Cursor(4),
                "/2+2=",
                0,
            ),
        ];
        let entrypoints = [NativeMenu, SlashMenu, CommandPalette, UiButton, Keyboard];
        let mut cells = 0;

        for action in MdFormat::ALL {
            for entrypoint in entrypoints {
                for (fixture, content, target, slash_content, slash_at) in fixtures {
                    cells += 1;
                    if !action.supports(entrypoint) {
                        assert!(
                            !editor_actions_for(entrypoint).contains(&action),
                            "unsupported cell is exposed: {action:?} x {entrypoint:?} x {fixture}"
                        );
                        continue;
                    }

                    let (cell_content, cell_target) = if entrypoint == SlashMenu {
                        (slash_content, EditTarget::Slash(slash_at))
                    } else {
                        (content, target)
                    };
                    let outcome = std::panic::catch_unwind(|| {
                        apply_editor_action(cell_content, cell_target, action)
                    });
                    let result = outcome.unwrap_or_else(|_| {
                        panic!("formatting cell {action:?} x {entrypoint:?} x {fixture} panicked")
                    });
                    if let Some(result) = result {
                        let (a, b) = result.selection;
                        assert!(
                            a <= b && b <= result.content.len(),
                            "invalid range: {action:?} x {entrypoint:?} x {fixture}"
                        );
                        assert!(result.content.is_char_boundary(a));
                        assert!(result.content.is_char_boundary(b));

                        let before = editor_snapshot(cell_content, cell_target.selection_hint());
                        let after = editor_snapshot(&result.content, result.selection);
                        let mut undoer = egui::util::undoer::Undoer::default();
                        record_programmatic_edit(&mut undoer, before.clone(), after.clone());
                        assert_eq!(
                            undoer.undo(&after),
                            Some(&before),
                            "undo cell {action:?} x {entrypoint:?} x {fixture}"
                        );
                        assert_eq!(
                            undoer.redo(&before),
                            Some(&after),
                            "redo cell {action:?} x {entrypoint:?} x {fixture}"
                        );
                    }
                }
            }
        }
        assert_eq!(cells, 16 * 5 * 8);
    }

    #[test]
    fn stale_slash_target_is_a_noop_for_every_action() {
        for action in MdFormat::ALL {
            assert_eq!(
                apply_editor_action("á", EditTarget::Slash(99), action),
                None,
                "stale slash cell {action:?}"
            );
            assert_eq!(
                apply_editor_action("á", EditTarget::Slash(1), action),
                None,
                "mid-codepoint slash cell {action:?}"
            );
        }
    }

    #[test]
    fn code_block_inside_existing_fence_is_a_noop() {
        assert_eq!(
            apply_editor_action(
                "```\ncode\n```",
                EditTarget::Selection(5, 8),
                MdFormat::CodeBlock,
            ),
            None
        );
    }

    #[test]
    fn consecutive_formats_reuse_the_updated_selection() {
        let bold =
            apply_editor_action("word", EditTarget::Selection(0, 4), MdFormat::Bold).unwrap();
        let italic = apply_editor_action(
            &bold.content,
            EditTarget::Selection(bold.selection.0, bold.selection.1),
            MdFormat::Italic,
        )
        .unwrap();
        assert_eq!(italic.content, "**_word_**");
        assert_eq!(italic.selection, (3, 7));
    }

    #[test]
    fn programmatic_edits_round_trip_through_undo_and_redo() {
        for action in MdFormat::ALL {
            let (before_content, target) = if action == MdFormat::Math {
                ("2+2=", EditTarget::Cursor(4))
            } else {
                ("word", EditTarget::Selection(0, 4))
            };
            let result = apply_editor_action(before_content, target, action)
                .unwrap_or_else(|| panic!("{action:?} needs a canonical edit result"));
            let before = editor_snapshot(before_content, target.selection_hint());
            let after = editor_snapshot(&result.content, result.selection);
            let mut undoer = egui::util::undoer::Undoer::default();
            record_programmatic_edit(&mut undoer, before.clone(), after.clone());
            assert_eq!(undoer.undo(&after), Some(&before), "undo {action:?}");
            assert_eq!(undoer.redo(&before), Some(&after), "redo {action:?}");
        }
    }

    #[test]
    fn programmatic_edit_after_undo_discards_the_old_redo_branch() {
        let before = editor_snapshot("word", (0, 4));
        let bold =
            apply_editor_action("word", EditTarget::Selection(0, 4), MdFormat::Bold).unwrap();
        let bold_snapshot = editor_snapshot(&bold.content, bold.selection);
        let mut undoer = egui::util::undoer::Undoer::default();
        record_programmatic_edit(&mut undoer, before.clone(), bold_snapshot.clone());
        assert_eq!(undoer.undo(&bold_snapshot), Some(&before));

        let italic =
            apply_editor_action("word", EditTarget::Selection(0, 4), MdFormat::Italic).unwrap();
        let italic_snapshot = editor_snapshot(&italic.content, italic.selection);
        record_programmatic_edit(&mut undoer, before, italic_snapshot.clone());

        assert!(!undoer.has_redo(&italic_snapshot));
        assert_eq!(undoer.redo(&italic_snapshot), None);
    }

    #[test]
    fn slash_undo_restores_the_caret_after_the_trigger() {
        let target = EditTarget::Slash(0);
        let result = apply_editor_action("/word", target, MdFormat::Bold).unwrap();
        let before = editor_snapshot("/word", target.selection_hint());
        let after = editor_snapshot(&result.content, result.selection);
        let mut undoer = egui::util::undoer::Undoer::default();
        record_programmatic_edit(&mut undoer, before.clone(), after.clone());

        let restored = undoer.undo(&after).expect("slash edit should be undoable");
        assert_eq!(restored.0.primary.index, 1);
        assert_eq!(restored.0.secondary.index, 1);
        assert_eq!(restored.1, "/word");
    }

    #[test]
    fn content_editor_id_is_scoped_to_note() {
        assert_ne!(content_editor_id("note-a"), content_editor_id("note-b"));
        assert_eq!(content_editor_id("note-a"), content_editor_id("note-a"));
    }

    #[test]
    fn egui_char_index_converts_to_utf8_byte_offset() {
        let content = "🙂 café";
        assert_eq!(char_index_to_byte(content, 0), 0);
        assert_eq!(char_index_to_byte(content, 1), 4);
        assert_eq!(char_index_to_byte(content, 5), 8);
        assert_eq!(char_index_to_byte(content, content.chars().count()), 10);
        assert_eq!(char_index_to_byte(content, usize::MAX), 10);
    }

    #[test]
    fn bold_wraps_selection() {
        assert_eq!(
            formatted("hello world", (0, 5), MdFormat::Bold),
            "**hello** world"
        );
    }
    #[test]
    fn italic_empty_inserts_markers_at_cursor() {
        assert_eq!(formatted("ab", (1, 1), MdFormat::Italic), "a__b");
    }
    #[test]
    fn link_wraps_selection() {
        assert_eq!(formatted("site", (0, 4), MdFormat::Link), "[site](url)");
    }
    #[test]
    fn heading_prefixes_current_line() {
        assert_eq!(formatted("foo\nbar", (4, 7), MdFormat::H2), "foo\n## bar");
    }
    #[test]
    fn bullet_prefixes_each_line_in_selection() {
        assert_eq!(formatted("a\nb", (0, 3), MdFormat::Bullet), "- a\n- b");
    }
    #[test]
    fn multibyte_selection_not_split() {
        // "café" is 5 bytes; wrapping the whole word must not panic or split 'é'.
        assert_eq!(formatted("café", (0, 5), MdFormat::Bold), "**café**");
    }
    #[test]
    fn range_past_end_is_clamped() {
        assert_eq!(formatted("hi", (0, 99), MdFormat::InlineCode), "`hi`");
    }
    #[test]
    fn line_prefix_selection_ending_in_newline_skips_empty_line() {
        // CAD-25b Slice 4: a selection ending right after a '\n' must NOT prefix
        // the empty trailing line that newline opens.
        assert_eq!(formatted("a\nb\n", (0, 4), MdFormat::Bullet), "- a\n- b\n");
        // Single line selected through its trailing newline → only that line.
        assert_eq!(formatted("foo\n", (0, 4), MdFormat::H2), "## foo\n");
    }
}
