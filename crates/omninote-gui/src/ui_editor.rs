use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::types::NoteType;

#[derive(Clone, Copy)]
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
}

impl MdFormat {
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

/// Apply a markdown format over the byte range `sel` (start,end; may be empty
/// for a bare cursor) and return the new content. Wrapping formats surround the
/// selection; line-prefix formats insert at the start of every line the range
/// touches. Byte offsets are snapped to char boundaries so multibyte text is
/// never split. Pure → unit-testable.
pub fn apply_md_format(content: &str, sel: (usize, usize), fmt: MdFormat) -> String {
    // snap + order + clamp
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

    if fmt.is_line_prefix() {
        let prefix = match fmt {
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
        let first = content[..a].rfind('\n').map(|i| i + 1).unwrap_or(0);
        // Collect every line-start offset within [first, b]: `first`, plus each
        // index after a '\n' that falls before `b`.
        let mut starts = vec![first];
        for (i, ch) in content.char_indices() {
            // `i + 1 < b` (not `i < b`): a selection ending right after a '\n'
            // must not prefix the empty line that newline opens. CAD-25b Slice 4.
            if ch == '\n' && i + 1 > first && i + 1 < b {
                starts.push(i + 1);
            }
        }
        starts.sort_unstable();
        starts.dedup();
        // Insert from last to first so earlier offsets stay valid.
        let mut out = content.to_string();
        for &s in starts.iter().rev() {
            out.insert_str(s, prefix);
        }
        return out;
    }

    let (pre, post): (&str, String) = match fmt {
        MdFormat::Bold => ("**", "**".into()),
        MdFormat::Italic => ("_", "_".into()),
        MdFormat::Strike => ("~~", "~~".into()),
        MdFormat::InlineCode => ("`", "`".into()),
        MdFormat::CodeBlock => ("```\n", "\n```".into()),
        MdFormat::Link => ("[", "](url)".into()),
        _ => unreachable!(),
    };
    format!(
        "{}{}{}{}{}",
        &content[..a],
        pre,
        &content[a..b],
        post,
        &content[b..]
    )
}

/// v0.8 — items shown in the slash menu when user types `/` at start of a line.
/// Returns (label, snippet). Snippet replaces the `/` character.
fn slash_menu_items() -> &'static [(&'static str, &'static str)] {
    &[
        ("# H1", "# "),
        ("## H2", "## "),
        ("### H3", "### "),
        ("**negrito**", "****"),
        ("_itálico_", "__"),
        ("`código inline`", "``"),
        ("```bloco de código```", "```\n\n```"),
        ("> citação", "> "),
        ("- lista", "- "),
        ("1. lista numerada", "1. "),
        ("- [ ] todo", "- [ ] "),
        ("[link](url)", "[](url)"),
        ("[[wikilink]]", "[[]]"),
        ("--- divisor", "---\n"),
    ]
}

impl OmniNoteApp {
    pub fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_note.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(RichText::new("📓 OmniNote").size(24.0).weak());
                        ui.add_space(16.0);
                        ui.label(RichText::new("Cmd+N  Nova nota").size(12.0).weak());
                        ui.label(RichText::new("Cmd+K  Buscar").size(12.0).weak());
                        ui.label(RichText::new("Cmd+,  Configurações").size(12.0).weak());
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
                if self.discipline_typed && self.show_discipline_typed(ui, &note) {
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
        let output = egui::TextEdit::multiline(&mut note.content)
            .code_editor()
            .desired_rows(30)
            .desired_width(ui.available_width())
            .hint_text("Escreva em markdown...")
            .show(ui);

        if output.response.changed() {
            self.dirty = true;
        }

        // Cmd+= math substitution on current line
        // Extract cursor pos before output is dropped (output holds &mut note.content).
        // `ccursor.index` is a CHAR index; autoformat and the slash menu below index
        // `content` by BYTE, so convert once here — a char index used as a byte offset
        // corrupts/erases text (and can panic in replace_range) on non-ASCII lines.
        let cursor_pos = output.cursor_range.map(|r| {
            let char_idx = r.primary.ccursor.index;
            note.content
                .char_indices()
                .nth(char_idx)
                .map(|(byte_idx, _)| byte_idx)
                .unwrap_or(note.content.len())
        });
        let has_focus = output.response.has_focus();
        // Selection (byte range) for the format menu. cursor_range carries CHAR
        // indices; convert to bytes like the math/slash code does below.
        let char_to_byte = |ci: usize| {
            note.content
                .char_indices()
                .nth(ci)
                .map(|(b, _)| b)
                .unwrap_or(note.content.len())
        };
        let sel = output.cursor_range.map(|r| {
            let a = char_to_byte(r.primary.ccursor.index);
            let b = char_to_byte(r.secondary.ccursor.index);
            (a.min(b), a.max(b))
        });
        // Seed from a native "Editar" menu click (native_menu.rs) — it acts on
        // `editor_sel` exactly like a right-click pick below, since both steal
        // focus from the editor the same way.
        let mut pending_format: Option<MdFormat> = self.pending_native_format.take();
        output.response.context_menu(|ui| {
            ui.label(egui::RichText::new("Formatar").size(10.0).weak());
            ui.separator();
            use MdFormat::*;
            for fmt in [Bold, Italic, Strike, InlineCode, Link] {
                if ui.button(fmt.label()).clicked() {
                    pending_format = Some(fmt);
                    ui.close_menu();
                }
            }
            ui.separator();
            for fmt in [H1, H2, H3, Bullet, Numbered, Todo, Quote, CodeBlock] {
                if ui.button(fmt.label()).clicked() {
                    pending_format = Some(fmt);
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

        let math_sc = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Equals);
        if has_focus && ui.input_mut(|i| crate::app::consume_app_shortcut(i, &math_sc)) {
            let pos = cursor_pos.unwrap_or(note.content.len());
            if let Some((new_line, start, end)) =
                omninote_core::autoformat::try_math_substitute(&note.content, pos)
            {
                note.content.replace_range(start..end, &new_line);
                self.dirty = true;
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
        let mut pending_replacement: Option<(usize, &'static str)> = None;
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
                    for (label, snippet) in slash_menu_items() {
                        if ui.button(*label).clicked() {
                            pending_replacement = Some((slash_at, *snippet));
                        }
                    }
                });
        }
        if let Some((slash_at, snippet)) = pending_replacement {
            // Replace the "/" with the snippet
            note.content.replace_range(slash_at..slash_at + 1, snippet);
            self.dirty = true;
            self.slash_menu_pos = None;
        }

        // Attach file
        ui.horizontal(|ui| {
            if ui.button("📎 Anexar arquivo").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    if let Some(v) = &self.vault {
                        match v.import_attachment(&path) {
                            Ok(name) => {
                                let note = self.active_note.as_mut().unwrap();
                                note.content.push_str(&format!("\n![[{}]]", name));
                                note.frontmatter.attachments.push(name);
                                self.dirty = true;
                            }
                            Err(e) => self.error_msg = Some(e),
                        }
                    }
                }
            }
        });

        if let Some(fmt) = pending_format {
            let note = self.active_note.as_mut().unwrap();
            let range = self
                .editor_sel
                .map(|(a, b)| (a.min(note.content.len()), b.min(note.content.len())))
                .unwrap_or((note.content.len(), note.content.len()));
            note.content = apply_md_format(&note.content, range, fmt);
            self.dirty = true;
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

    #[test]
    fn bold_wraps_selection() {
        assert_eq!(
            apply_md_format("hello world", (0, 5), MdFormat::Bold),
            "**hello** world"
        );
    }
    #[test]
    fn italic_empty_inserts_markers_at_cursor() {
        assert_eq!(apply_md_format("ab", (1, 1), MdFormat::Italic), "a__b");
    }
    #[test]
    fn link_wraps_selection() {
        assert_eq!(
            apply_md_format("site", (0, 4), MdFormat::Link),
            "[site](url)"
        );
    }
    #[test]
    fn heading_prefixes_current_line() {
        assert_eq!(
            apply_md_format("foo\nbar", (4, 7), MdFormat::H2),
            "foo\n## bar"
        );
    }
    #[test]
    fn bullet_prefixes_each_line_in_selection() {
        assert_eq!(
            apply_md_format("a\nb", (0, 3), MdFormat::Bullet),
            "- a\n- b"
        );
    }
    #[test]
    fn multibyte_selection_not_split() {
        // "café" is 5 bytes; wrapping the whole word must not panic or split 'é'.
        assert_eq!(apply_md_format("café", (0, 5), MdFormat::Bold), "**café**");
    }
    #[test]
    fn range_past_end_is_clamped() {
        assert_eq!(apply_md_format("hi", (0, 99), MdFormat::InlineCode), "`hi`");
    }
    #[test]
    fn line_prefix_selection_ending_in_newline_skips_empty_line() {
        // CAD-25b Slice 4: a selection ending right after a '\n' must NOT prefix
        // the empty trailing line that newline opens.
        assert_eq!(
            apply_md_format("a\nb\n", (0, 4), MdFormat::Bullet),
            "- a\n- b\n"
        );
        // Single line selected through its trailing newline → only that line.
        assert_eq!(apply_md_format("foo\n", (0, 4), MdFormat::H2), "## foo\n");
    }
}
