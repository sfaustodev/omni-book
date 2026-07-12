//! Typed views over the discipline (sacred) files — CAD-25 Slice 5 §3.4.
//!
//! When a discipline file (`SPRINT.md`, `DIARY.md`, `HUMAN.md`, `PLAN.md`, …) is
//! the active note, [`OmniNoteApp::show_editor`] forks to a structured renderer
//! instead of the generic markdown editor. The parsing is **pure free functions**
//! (tested headless, no egui `Context`) sitting on top of
//! [`omninote_core::discipline`], which already reads/resolves the files.
//!
//! Views are read-only in v1.2 — the one exception is DIARY's `+ Append entry`,
//! which reuses [`omninote_core::discipline::diary_quick`]. Mutating sacred files
//! from the UI is out of scope (discipline rules #6/#7).

use crate::app::OmniNoteApp;
use egui::RichText;
use omninote_core::discipline::{self, DisciplineFile};
use omninote_core::types::Note;
use std::path::Path;

// ──────────────────────── data model (parser outputs) ────────────────────────

/// Lifecycle bucket for a sprint task, mapped from a free-text/emoji status cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Done,
    Doing,
    Todo,
    Blocked,
}

impl TaskStatus {
    /// Sort weight so active work floats to the top: Doing > Todo > Blocked > Done.
    fn order(self) -> u8 {
        match self {
            TaskStatus::Doing => 0,
            TaskStatus::Todo => 1,
            TaskStatus::Blocked => 2,
            TaskStatus::Done => 3,
        }
    }

    fn label(self) -> &'static str {
        match self {
            TaskStatus::Done => "Done",
            TaskStatus::Doing => "Doing",
            TaskStatus::Todo => "Todo",
            TaskStatus::Blocked => "Blocked",
        }
    }
}

/// One task row parsed from a SPRINT table (or a bullet-list fallback).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SprintTask {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub kind: Option<String>,
    pub owner: Option<String>,
    pub points: Option<u32>,
}

/// One DIARY entry under a date heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiaryEntry {
    pub time: Option<String>,
    pub labels: Vec<String>,
    pub snippet: String,
}

/// A day of DIARY entries (`## YYYY-MM-DD — title`), newest-first in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiaryDay {
    pub date: String,
    pub entries: Vec<DiaryEntry>,
}

/// One HUMAN.md question (`### Q-NN · …`), bucketed Open vs Resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HumanQ {
    pub id: String,
    pub question: String,
    pub resolved: bool,
}

/// Which tracker a ticket row came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Notion,
    Jira,
}

/// A merged ticket row from NOTION.md and/or JIRA.md.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub provider: Provider,
    pub id: String,
    pub title: String,
    pub status: String,
    pub meta: String,
}

// ──────────────────────── pure parsers (free fns) ────────────────────────

/// Map a status cell's text/emoji to a [`TaskStatus`]. Checked Done-first so a
/// row that mentions both a checkbox and a word lands on the strongest signal;
/// unknown text defaults to Todo (a backlog item is the safe assumption).
pub fn status_from_cell(cell: &str) -> TaskStatus {
    let c = cell.to_lowercase();
    let has = |needle: &str| c.contains(needle);
    // Blocked is checked before Done/Doing: "blocked" must never be read as a
    // generic word match, and a ⛔ row is unambiguously stuck.
    if has("⛔") || has("🚫") || has("block") {
        return TaskStatus::Blocked;
    }
    if cell.contains('✅') || has("done") || has("concluída") || has("concluida") || has("[x]") {
        return TaskStatus::Done;
    }
    if cell.contains('🚧')
        || cell.contains('🔄')
        || has("progress")
        || has("execução")
        || has("execucao")
        || has("em obra")
        || has("parcial")
    {
        return TaskStatus::Doing;
    }
    if cell.contains('🌱') || has("todo") || has("backlog") || has("a fazer") || has("[ ]") {
        return TaskStatus::Todo;
    }
    TaskStatus::Todo
}

/// Extract every `[...]` bracket-label token (the DIARY/SPRINT convention),
/// deduped and order-preserving. Skips `[[wikilinks]]` (double bracket) and
/// `[ ]` / `[x]` task checkboxes.
pub fn label_chips(s: &str) -> Vec<String> {
    let bytes = s.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            // Double bracket → wikilink; skip both and resume after them.
            if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                continue;
            }
            if let Some(close_rel) = s[i + 1..].find(']') {
                let inner = &s[i + 1..i + 1 + close_rel];
                let trimmed = inner.trim();
                let is_checkbox = trimmed.is_empty() || trimmed.eq_ignore_ascii_case("x");
                // A label can't span a line break or contain a nested '[' — that
                // signals malformed/markdown-link syntax, not a label.
                let clean = !inner.contains('\n') && !inner.contains('[');
                if !is_checkbox && clean && !out.iter().any(|e| e == trimmed) {
                    out.push(trimmed.to_string());
                }
                i += 1 + close_rel + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Whether a markdown table row is a separator (`|---|:--:|---|`).
fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
}

/// Split a markdown table row `| a | b | c |` into trimmed cell strings,
/// honouring GFM's escaped pipe (`\|` is a literal `|` inside a cell, not a
/// column boundary). Returns `None` for lines that aren't table rows.
/// (triad-codex Slice 5.)
fn split_row(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with('|') {
        return None;
    }
    // Strip the leading and (optional) trailing pipe, then split on unescaped '|'.
    let inner = t.trim_start_matches('|').trim_end_matches('|');
    Some(split_escaped_pipes(inner))
}

/// Split on `|` while treating `\|` as an escaped literal pipe (the `\` is
/// dropped, the `|` kept in the cell). Other backslashes pass through verbatim.
/// Cells are trimmed.
fn split_escaped_pipes(s: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cur.push('|');
                chars.next();
            }
            '|' => {
                cells.push(cur.trim().to_string());
                cur = String::new();
            }
            other => cur.push(other),
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

/// Locate, in a header row's cells, the column indices for ID / Title / Status.
/// Returns `None` unless all three are present (the signal that promotes a table
/// to a task list, distinguishing it from e.g. the NOTION Schema table).
fn task_header_columns(cells: &[String]) -> Option<(usize, usize, usize)> {
    let mut id_col = None;
    let mut title_col = None;
    let mut status_col = None;
    for (idx, cell) in cells.iter().enumerate() {
        let c = cell.to_lowercase();
        let c = c.trim();
        if c == "id" {
            id_col.get_or_insert(idx);
        } else if c == "tarefa" || c == "title" || c == "título" || c == "titulo" {
            title_col.get_or_insert(idx);
        } else if c == "status" {
            status_col.get_or_insert(idx);
        }
    }
    Some((id_col?, title_col?, status_col?))
}

/// Parse SPRINT-style task tables and, failing that, a `- [ ]` / `- [x]` bullet
/// list. Only tables whose header carries ID + (Tarefa|Title) + Status are read
/// as task lists; other tables (e.g. the NOTION Schema table) are ignored.
pub fn sprint_tasks(raw: &str) -> Vec<SprintTask> {
    let mut tasks = Vec::new();
    let mut cols: Option<(usize, usize, usize)> = None;
    let mut saw_table = false;

    for line in raw.lines() {
        let Some(cells) = split_row(line) else {
            // A non-table line ends the current table context.
            cols = None;
            continue;
        };
        if is_separator_row(&cells) {
            continue;
        }
        match cols {
            None => {
                // Looking for a header that promotes this table to a task list.
                if let Some(found) = task_header_columns(&cells) {
                    cols = Some(found);
                    saw_table = true;
                }
            }
            Some((id_c, title_c, status_c)) => {
                let id = cells.get(id_c).cloned().unwrap_or_default();
                let title = cells.get(title_c).cloned().unwrap_or_default();
                let status_cell = cells.get(status_c).cloned().unwrap_or_default();
                // An empty ID row is a sub-header or spacer, not a task.
                if id.is_empty() {
                    continue;
                }
                tasks.push(SprintTask {
                    id,
                    title,
                    status: status_from_cell(&status_cell),
                    kind: None,
                    owner: None,
                    points: None,
                });
            }
        }
    }

    if !saw_table {
        tasks.extend(bullet_tasks(raw));
    }
    tasks
}

/// Fallback: parse GFM task bullets `- [ ] text` / `- [x] text`.
fn bullet_tasks(raw: &str) -> Vec<SprintTask> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim_start();
        let rest = t
            .strip_prefix("- ")
            .or_else(|| t.strip_prefix("* "))
            .or_else(|| t.strip_prefix("+ "));
        let Some(rest) = rest else { continue };
        let (status, text) = if let Some(body) = rest.strip_prefix("[x] ") {
            (TaskStatus::Done, body)
        } else if let Some(body) = rest.strip_prefix("[X] ") {
            (TaskStatus::Done, body)
        } else if let Some(body) = rest.strip_prefix("[ ] ") {
            (TaskStatus::Todo, body)
        } else {
            continue;
        };
        out.push(SprintTask {
            id: String::new(),
            title: text.trim().to_string(),
            status,
            kind: None,
            owner: None,
            points: None,
        });
    }
    out
}

/// Group DIARY.md into days keyed on `## YYYY-MM-DD` headings, newest-first
/// (file order already is). Each day's entries carry bracket-labels and an
/// 80-char snippet of the body sans the heading line.
pub fn diary_days(raw: &str) -> Vec<DiaryDay> {
    let mut days = Vec::new();
    for entry in discipline::parse_entries(raw) {
        let date = leading_date(&entry.heading);
        let Some(date) = date else { continue };
        // Body minus the heading line.
        let body_no_heading: String = entry
            .body
            .lines()
            .filter(|l| !l.trim_start().starts_with("## "))
            .collect::<Vec<_>>()
            .join("\n");
        let labels = label_chips(&entry.body);
        let snippet = snippet_80(&body_no_heading);
        days.push(DiaryDay {
            date,
            entries: vec![DiaryEntry {
                time: None,
                labels,
                snippet,
            }],
        });
    }
    days
}

/// `2026-06-27` if `s` starts with an ISO date, else `None`. The heading may
/// continue with ` — title`; only the date prefix is captured.
fn leading_date(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let ok = bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'-'
        && bytes[5].is_ascii_digit()
        && bytes[6].is_ascii_digit()
        && bytes[7] == b'-'
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit();
    if ok {
        Some(s[..10].to_string())
    } else {
        None
    }
}

/// First 80 chars of the first non-empty line, ellipsized if truncated.
fn snippet_80(body: &str) -> String {
    let first = body
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first.chars().count() <= 80 {
        first.to_string()
    } else {
        let kept: String = first.chars().take(79).collect();
        format!("{kept}…")
    }
}

/// Parse HUMAN.md `### Q-NN · …` blocks, bucketing them by the `## Open
/// questions` / `## Resolved` section markers. A question after the Resolved
/// marker (or whose heading says `resolved`) is flagged resolved.
pub fn human_questions(raw: &str) -> Vec<HumanQ> {
    let mut out = Vec::new();
    let mut in_resolved = false;
    for line in raw.lines() {
        let t = line.trim();
        let lower = t.to_lowercase();
        if t.starts_with("## ") {
            if lower.contains("resolved") || lower.contains("resolvid") {
                in_resolved = true;
            } else if lower.contains("open question") || lower.contains("abertas") {
                in_resolved = false;
            }
            continue;
        }
        if let Some(rest) = t.strip_prefix("### ") {
            // Heading shape: `Q-NN · question text · raised … · resolved …`.
            let rest = rest.trim();
            if let Some(id) = parse_q_id(rest) {
                let after_id = rest[id.len()..].trim_start();
                let after_id = after_id.trim_start_matches('·').trim();
                // Question = text up to the first ` · ` metadata separator.
                let question = after_id
                    .split('·')
                    .next()
                    .unwrap_or(after_id)
                    .trim()
                    .to_string();
                let resolved = in_resolved || lower.contains("resolved");
                out.push(HumanQ {
                    id,
                    question,
                    resolved,
                });
            }
        }
    }
    out
}

/// `Q-07` from the start of a heading body, else `None`.
fn parse_q_id(s: &str) -> Option<String> {
    if !s.starts_with("Q-") {
        return None;
    }
    let digits: String = s[2..].chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    Some(format!("Q-{digits}"))
}

/// Merge ticket rows from NOTION.md (markdown table) and JIRA.md (`SCRUM-XX`
/// lines / `### SCRUM-XX` headings). Provider-tagged; sorted by status bucket
/// (Doing < Todo < Done) then id. Ticket IDs are word-bounded so `CAD-2` never
/// shadows `CAD-25`.
pub fn tickets_merged(notion_raw: &str, jira_raw: &str) -> Vec<Ticket> {
    let mut out: Vec<Ticket> = Vec::new();
    out.extend(notion_tickets(notion_raw));
    out.extend(jira_tickets(jira_raw));
    out.sort_by(|a, b| {
        let oa = status_from_cell(&a.status).order();
        let ob = status_from_cell(&b.status).order();
        oa.cmp(&ob).then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// Parse the NOTION ticket table: a row whose first cell is a `CAD-XX` id.
fn notion_tickets(raw: &str) -> Vec<Ticket> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let Some(cells) = split_row(line) else {
            continue;
        };
        if is_separator_row(&cells) {
            continue;
        }
        let Some(first) = cells.first() else { continue };
        if !looks_like_ticket_id(first, "CAD-") {
            continue;
        }
        let id = first.clone();
        let title = cells.get(1).cloned().unwrap_or_default();
        // Status is the first cell after the title that carries a status glyph or
        // keyword; the column index varies between tables, so detect by content.
        let status = cells
            .iter()
            .skip(2)
            .find(|c| is_status_cell(c))
            .cloned()
            .unwrap_or_default();
        out.push(Ticket {
            provider: Provider::Notion,
            id,
            title,
            status,
            meta: String::new(),
        });
    }
    out
}

/// Parse JIRA `SCRUM-XX` references — either a `### SCRUM-XX · title` heading or
/// a plain line that starts with the id.
fn jira_tickets(raw: &str) -> Vec<Ticket> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let t = line.trim().trim_start_matches('#').trim();
        let first_tok = t.split_whitespace().next().unwrap_or("");
        if !looks_like_ticket_id(first_tok, "SCRUM-") {
            continue;
        }
        let id = first_tok.to_string();
        let after = t[first_tok.len()..].trim_start();
        let after = after.trim_start_matches('·').trim();
        let title = after
            .split(['·', '|'])
            .next()
            .unwrap_or(after)
            .trim()
            .to_string();
        out.push(Ticket {
            provider: Provider::Jira,
            id,
            title,
            status: String::new(),
            meta: String::new(),
        });
    }
    out
}

/// `true` if `tok` is exactly `<prefix><digits...>` with at least one digit and
/// no trailing id chars (word-bounded — `CAD-2` ≠ `CAD-25`).
fn looks_like_ticket_id(tok: &str, prefix: &str) -> bool {
    let Some(rest) = tok.strip_prefix(prefix) else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    // Allow `CAD-23.1`-style dotted ids; reject a trailing letter/word char.
    rest.chars().all(|c| c.is_ascii_digit() || c == '.') && rest.chars().any(|c| c.is_ascii_digit())
}

/// Heuristic: a cell that looks like a status (glyph or keyword), used to pick
/// the status column out of a variable-width NOTION row.
fn is_status_cell(cell: &str) -> bool {
    !matches!(status_from_cell(cell), TaskStatus::Todo) || {
        let c = cell.to_lowercase();
        c.contains("todo") || c.contains("backlog") || c.contains("fazer") || cell.contains('🌱')
    }
}

// ──────────────────────── dispatch helper ────────────────────────

/// Which discipline file a note's relative path maps to, or `None` for an
/// ordinary note. The match is anchored to the sacred *location*: the file must
/// sit at the vault root (`DIARY.md`) or directly under `discipline/`
/// (`discipline/DIARY.md`) — the only two places `DisciplineFile::resolve_path`
/// looks. A stray `Projetos/DIARY.md` is therefore a normal note, not a typed
/// (read-only) view that could append to the wrong file. Pure and testable.
/// (triad-codex Slice 5.)
pub fn discipline_file_of(rel_path: &Path) -> Option<DisciplineFile> {
    let name = rel_path.file_name()?.to_str()?;
    // Parent must be empty (root) or exactly `discipline`.
    let parent_ok = match rel_path.parent() {
        None => true,
        Some(p) if p.as_os_str().is_empty() => true,
        Some(p) => p == Path::new("discipline"),
    };
    if !parent_ok {
        return None;
    }
    [
        DisciplineFile::Diary,
        DisciplineFile::Sprint,
        DisciplineFile::Human,
        DisciplineFile::Plan,
        DisciplineFile::Jira,
        DisciplineFile::Notion,
        DisciplineFile::Eternal,
    ]
    .into_iter()
    .find(|df| df.filename() == name)
}

pub(crate) fn has_typed_discipline_view(rel_path: &Path) -> bool {
    matches!(
        discipline_file_of(rel_path),
        Some(
            DisciplineFile::Sprint
                | DisciplineFile::Diary
                | DisciplineFile::Human
                | DisciplineFile::Plan
                | DisciplineFile::Eternal
        )
    )
}

// ──────────────────────── render methods (thin egui) ────────────────────────

impl OmniNoteApp {
    /// Dispatch a discipline note to its typed renderer. Returns `true` if it
    /// handled the note (caller skips the generic editor body), `false` to fall
    /// through. Honours the `discipline_typed` toggle (Typed ↔ Raw).
    pub fn show_discipline_typed(&mut self, ui: &mut egui::Ui, note: &Note) -> bool {
        if !self.discipline_typed {
            return false;
        }
        let Some(df) = discipline_file_of(&note.rel_path) else {
            return false;
        };
        match df {
            DisciplineFile::Sprint => self.show_sprint_view(ui, note),
            DisciplineFile::Diary => self.show_diary_view(ui, note),
            DisciplineFile::Human => self.show_human_view(ui, note),
            DisciplineFile::Plan | DisciplineFile::Eternal => self.show_plan_view(ui, note),
            // JIRA/NOTION reach their structured view through the Tickets panel,
            // not the per-note fork; show the generic body here.
            DisciplineFile::Jira | DisciplineFile::Notion => return false,
        }
        true
    }

    fn discipline_theme(&self) -> crate::theme::Theme {
        self.vault
            .as_ref()
            .map(|v| crate::app::theme_for_config(&v.config))
            .unwrap_or_else(crate::theme::Theme::obsidian_dark)
    }

    pub fn show_sprint_view(&mut self, ui: &mut egui::Ui, note: &Note) {
        let theme = self.discipline_theme();
        let tasks = sprint_tasks(&note.content);

        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "◈", 18.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, &note.title, 18.0).strong());
            let active = tasks
                .iter()
                .any(|t| matches!(t.status, TaskStatus::Doing | TaskStatus::Todo));
            let (txt, col) = if active {
                ("ACTIVE", theme.accent)
            } else {
                ("CLOSED", theme.dim)
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    crate::ui_a11y::scaled_text(ui, txt, 11.0)
                        .color(col)
                        .strong(),
                );
            });
        });
        ui.separator();

        let count = |s: TaskStatus| tasks.iter().filter(|t| t.status == s).count();
        let done = count(TaskStatus::Done);
        let doing = count(TaskStatus::Doing);
        let blocked = count(TaskStatus::Blocked);
        let todo = count(TaskStatus::Todo);
        self.draw_progress_bar(ui, &theme, done, doing, blocked, todo);

        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("sprint_view_scroll")
            .show(ui, |ui| {
                let mut pending: Option<String> = None;
                for status in [
                    TaskStatus::Doing,
                    TaskStatus::Todo,
                    TaskStatus::Blocked,
                    TaskStatus::Done,
                ] {
                    let section: Vec<&SprintTask> =
                        tasks.iter().filter(|t| t.status == status).collect();
                    if section.is_empty() {
                        continue;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        crate::ui_a11y::scaled_text(ui, status.label(), 12.0)
                            .color(status_color(&theme, status))
                            .strong(),
                    );
                    for t in section {
                        if self.draw_task_row(ui, &theme, t) {
                            pending = Some(format!("SPECS/{}", t.id));
                        }
                    }
                }
                if let Some(target) = pending {
                    self.select_note_by_target(&target);
                }
            });
    }

    fn draw_progress_bar(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::Theme,
        done: usize,
        doing: usize,
        blocked: usize,
        todo: usize,
    ) {
        let total = (done + doing + blocked + todo).max(1) as f32;
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 10.0), egui::Sense::hover());
        let rounding = egui::Rounding::same(5.0);
        ui.painter().rect_filled(rect, rounding, theme.panel);
        let mut x = rect.left();
        for (n, col) in [
            (done, status_color(theme, TaskStatus::Done)),
            (doing, status_color(theme, TaskStatus::Doing)),
            (blocked, status_color(theme, TaskStatus::Blocked)),
            (todo, status_color(theme, TaskStatus::Todo)),
        ] {
            if n == 0 {
                continue;
            }
            let seg_w = w * (n as f32 / total);
            let seg = egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + seg_w, rect.bottom()),
            );
            ui.painter().rect_filled(seg, rounding, col);
            x += seg_w;
        }
        ui.horizontal(|ui| {
            for (label, n, status) in [
                ("done", done, TaskStatus::Done),
                ("doing", doing, TaskStatus::Doing),
                ("blocked", blocked, TaskStatus::Blocked),
                ("todo", todo, TaskStatus::Todo),
            ] {
                ui.label(
                    crate::ui_a11y::scaled_text(ui, "●", 10.0).color(status_color(theme, status)),
                );
                ui.label(crate::ui_a11y::scaled_text(ui, format!("{label} {n}"), 10.0).weak());
            }
        });
    }

    /// One task row. When it carries an id the whole row is a focusable,
    /// keyboard-activatable target (Enter/Space) that routes to the spec; an
    /// id-less row is static. Returns true when activated. (triad a11y Slice 5.)
    fn draw_task_row(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::Theme,
        t: &SprintTask,
    ) -> bool {
        let body = |ui: &mut egui::Ui| {
            ui.label(
                crate::ui_a11y::scaled_text(ui, "▣", 13.0).color(status_color(theme, t.status)),
            );
            if !t.id.is_empty() {
                ui.label(
                    crate::ui_a11y::scaled_text(ui, &t.id, 12.0)
                        .monospace()
                        .color(theme.accent),
                );
            }
            ui.label(crate::ui_a11y::scaled_text(ui, &t.title, 13.0).color(theme.text));
            if let Some(pts) = t.points {
                ui.label(crate::ui_a11y::scaled_text(ui, format!("{pts}pt"), 10.0).weak());
            }
        };
        if t.id.is_empty() {
            ui.horizontal(body);
            false
        } else {
            let label = format!("{} {}", t.id, t.title);
            let (_, activated) = crate::ui_a11y::clickable_row(ui, theme, &label, false, body);
            activated
        }
    }

    pub fn show_diary_view(&mut self, ui: &mut egui::Ui, note: &Note) {
        let theme = self.discipline_theme();
        let days = diary_days(&note.content);
        let project = self
            .vault
            .as_ref()
            .and_then(|v| v.root.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "✎", 16.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, format!("DIARY · {project}"), 16.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Append entry").clicked() {
                    self.capture_open = false; // ensure single dialog
                    self.diary_append_open = true;
                    self.diary_append_text.clear();
                }
                let last = days.first().map(|d| d.date.as_str()).unwrap_or("—");
                ui.label(
                    crate::ui_a11y::scaled_text(
                        ui,
                        format!("{} entradas · {}", days.len(), last),
                        11.0,
                    )
                    .weak(),
                );
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("diary_view_scroll")
            .show(ui, |ui| {
                for day in &days {
                    egui::CollapsingHeader::new(RichText::new(&day.date).strong())
                        .id_salt(format!("diary_day_{}", day.date))
                        .default_open(true)
                        .show(ui, |ui| {
                            for entry in &day.entries {
                                if !entry.labels.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for label in &entry.labels {
                                            chip(ui, &theme, label);
                                        }
                                    });
                                }
                                ui.label(
                                    crate::ui_a11y::scaled_text(ui, &entry.snippet, 12.0)
                                        .color(theme.dim),
                                );
                            }
                        });
                }
            });
    }

    pub fn show_human_view(&mut self, ui: &mut egui::Ui, note: &Note) {
        let theme = self.discipline_theme();
        let questions = human_questions(&note.content);
        let open: Vec<&HumanQ> = questions.iter().filter(|q| !q.resolved).collect();
        let resolved: Vec<&HumanQ> = questions.iter().filter(|q| q.resolved).collect();

        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "☻", 16.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, &note.title, 16.0).strong());
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("human_view_scroll")
            .show(ui, |ui| {
                ui.label(
                    crate::ui_a11y::scaled_text(
                        ui,
                        format!("Open questions ({})", open.len()),
                        13.0,
                    )
                    .strong(),
                );
                for q in &open {
                    ui.horizontal(|ui| {
                        chip(ui, &theme, &q.id);
                        ui.label(
                            crate::ui_a11y::scaled_text(ui, &q.question, 13.0).color(theme.text),
                        );
                    });
                }
                ui.add_space(8.0);
                egui::CollapsingHeader::new(
                    RichText::new(format!("Resolved ({})", resolved.len())).weak(),
                )
                .id_salt("human_resolved")
                .default_open(false)
                .show(ui, |ui| {
                    for q in &resolved {
                        ui.horizontal(|ui| {
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, &q.id, 11.0)
                                    .monospace()
                                    .color(theme.dim),
                            );
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, &q.question, 12.0).color(theme.dim),
                            );
                        });
                    }
                });
            });
    }

    pub fn show_plan_view(&mut self, ui: &mut egui::Ui, note: &Note) {
        let theme = self.discipline_theme();
        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "◇", 16.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, &note.title, 16.0).strong());
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("plan_view_scroll")
            .show(ui, |ui| {
                for (i, entry) in discipline::parse_entries(&note.content)
                    .into_iter()
                    .enumerate()
                {
                    let title = if entry.heading.is_empty() {
                        "(sem título)".to_string()
                    } else {
                        entry.heading.clone()
                    };
                    egui::CollapsingHeader::new(RichText::new(title).strong())
                        .id_salt(format!("plan_entry_{i}"))
                        .default_open(i == 0)
                        .show(ui, |ui| {
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, &entry.body, 12.0).color(theme.dim),
                            );
                        });
                }
            });
    }

    /// Full-panel Tickets view (Cmd+Shift+J). Reads NOTION.md + JIRA.md
    /// (Ok-or-empty so a Notion-only vault doesn't error), merges, filters by the
    /// chip + query, and renders provider-tagged rows. Row click routes to a local
    /// `SPECS/<id>` mirror if present, else opens nothing (URL plumbing deferred).
    pub fn show_tickets_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.discipline_theme();
        // Read the tracker files from the already-loaded `v.notes` (in memory)
        // rather than `std::fs` — immediate mode repaints this panel every frame,
        // and per-frame disk I/O + reparse is the busy-loop pattern. `reload_notes`
        // keeps the in-memory copy fresh, so cache invalidation is free. If a file
        // somehow isn't in `notes`, fall back to a one-off disk read.
        let (notion_raw, jira_raw) = match &self.vault {
            Some(v) => (
                discipline_content(v, DisciplineFile::Notion),
                discipline_content(v, DisciplineFile::Jira),
            ),
            None => (String::new(), String::new()),
        };
        let tickets = tickets_merged(&notion_raw, &jira_raw);

        ui.horizontal(|ui| {
            ui.label(crate::ui_a11y::scaled_text(ui, "◧", 18.0).color(theme.accent));
            ui.label(crate::ui_a11y::scaled_text(ui, "Tickets", 18.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Sync controls are visible-but-disabled stubs (titlebar precedent).
                ui.add_enabled(false, egui::Button::new("⤴"))
                    .on_disabled_hover_text("Sync push (em breve)");
                ui.add_enabled(false, egui::Button::new("⟲"))
                    .on_disabled_hover_text("Sync pull (em breve)");
            });
        });

        ui.horizontal(|ui| {
            use crate::app::TicketFilter;
            let mut f = self.tickets_filter;
            ui.selectable_value(&mut f, TicketFilter::All, "All");
            ui.selectable_value(&mut f, TicketFilter::Doing, "Doing");
            ui.selectable_value(&mut f, TicketFilter::Todo, "Todo");
            ui.selectable_value(&mut f, TicketFilter::Done, "Done");
            self.tickets_filter = f;
            ui.add(
                egui::TextEdit::singleline(&mut self.tickets_query)
                    .hint_text("🔍 filtrar…")
                    .desired_width(160.0),
            );
        });
        ui.separator();

        let filter = self.tickets_filter;
        let query = self.tickets_query.to_lowercase();
        let mut pending: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_salt("tickets_scroll")
            .show(ui, |ui| {
                let mut shown = 0usize;
                for t in &tickets {
                    if !ticket_matches_filter(t, filter) {
                        continue;
                    }
                    if !query.is_empty()
                        && !t.id.to_lowercase().contains(&query)
                        && !t.title.to_lowercase().contains(&query)
                    {
                        continue;
                    }
                    shown += 1;
                    let (tag, tag_col) = match t.provider {
                        Provider::Notion => ("N", theme.accent),
                        Provider::Jira => ("J", theme.provider_jira()),
                    };
                    let row_label = format!("{} {} {}", tag, t.id, t.title);
                    let (_, activated) =
                        crate::ui_a11y::clickable_row(ui, &theme, &row_label, false, |ui| {
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, tag, 11.0)
                                    .color(tag_col)
                                    .monospace(),
                            );
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, &t.id, 12.0)
                                    .monospace()
                                    .color(theme.accent),
                            );
                            ui.label(
                                crate::ui_a11y::scaled_text(ui, &t.title, 13.0).color(theme.text),
                            );
                            if !t.status.is_empty() {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            crate::ui_a11y::scaled_text(ui, &t.status, 11.0)
                                                .color(theme.dim),
                                        );
                                    },
                                );
                            }
                        });
                    if activated {
                        pending = Some(t.id.clone());
                    }
                }
                if shown == 0 {
                    ui.label(
                        crate::ui_a11y::scaled_text(ui, "Nenhum ticket nesse filtro.", 13.0)
                            .color(theme.dim),
                    );
                }
            });

        if let Some(id) = pending {
            // Local spec mirror first; URL fallback deferred (SPECS/ may be absent).
            let target = format!("SPECS/{id}");
            self.select_note_by_target(&target);
        }
    }

    /// DIARY `+ Append entry` modal — the one mutating affordance, reusing
    /// `discipline::diary_quick`. The body is multiline, so plain Enter must
    /// insert a newline; submission requires Cmd/Ctrl+Enter (a footer button is
    /// also offered). Esc cancels. (triad-agy/codex Slice 5.)
    pub fn show_diary_append(&mut self, ctx: &egui::Context) {
        if !self.diary_append_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.diary_append_open = false;
            return;
        }
        // Cmd/Ctrl+Enter submits; a bare Enter falls through to the TextEdit as a
        // newline. `command` maps to Cmd on macOS and Ctrl elsewhere. `!alt` keeps
        // AltGr (= Ctrl+Alt) from triggering submit, consistent with the app's
        // other shortcuts.
        let mut submit = ctx.input(|i| {
            i.key_pressed(egui::Key::Enter)
                && !i.modifiers.alt
                && (i.modifiers.command || i.modifiers.ctrl)
        });
        egui::Window::new("diary_append")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 120.0])
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label(RichText::new("Nova entrada → DIARY.md").weak().size(11.0));
                let edit = ui.add(
                    egui::TextEdit::multiline(&mut self.diary_append_text)
                        .hint_text("o que aconteceu…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(3),
                );
                edit.request_focus();
                ui.horizontal(|ui| {
                    if ui.button("Adicionar").clicked() {
                        submit = true;
                    }
                    ui.label(
                        RichText::new("⌘/Ctrl+Enter envia · Esc cancela")
                            .weak()
                            .size(10.0),
                    );
                });
            });
        if submit && !self.diary_append_text.trim().is_empty() {
            let text = self.diary_append_text.trim().to_string();
            match self.append_diary_entry(&text) {
                Ok(()) => self.toast_success("Entrada adicionada ao DIARY"),
                Err(e) => self.toast_error(format!("Falha no DIARY: {e}")),
            }
            self.diary_append_open = false;
            self.diary_append_text.clear();
        }
    }

    /// Append `text` as a new DIARY entry, preserving any unsaved edits to the
    /// active note. Flushes first so the reload below (re-reading notes from disk)
    /// can't replace a dirty in-memory buffer with stale on-disk content; then
    /// prepends via `diary_quick`, marks the write as ours for the watcher,
    /// reloads, and re-syncs the active note so a later autosave can't clobber it.
    pub(crate) fn append_diary_entry(&mut self, text: &str) -> Result<(), String> {
        if self.dirty && !self.flush_active() {
            return Err("não foi possível salvar edições pendentes".into());
        }
        self.clear_editor_transients();
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());
        let root = self
            .vault
            .as_ref()
            .map(|v| v.root.clone())
            .ok_or("sem vault")?;
        discipline::diary_quick(&root, text, None)?;
        // Our own write — keep the watcher from reading it back as an external
        // change and popping the conflict modal.
        self.self_write_until = std::time::Instant::now() + std::time::Duration::from_millis(400);
        if let Some(v) = &mut self.vault {
            v.reload_notes();
        }
        // Re-sync the active note (likely DIARY.md) from the reloaded vault so the
        // typed view shows the new entry and a later autosave can't clobber it.
        if let Some(id) = active_id {
            let fresh = self
                .vault
                .as_ref()
                .and_then(|v| v.notes.iter().find(|n| n.frontmatter.id == id).cloned());
            if let Some(fresh) = fresh {
                self.active_note = Some(fresh);
                self.dirty = false;
            }
        }
        Ok(())
    }

    /// Sidebar 'DISCIPLINES' section — lists only sacred files that resolve on
    /// disk; a row selects that note, triggering the typed-view fork in the editor.
    pub fn show_discipline_section(&mut self, ui: &mut egui::Ui) {
        let entries: Vec<(DisciplineFile, String)> = match &self.vault {
            Some(v) => [
                DisciplineFile::Sprint,
                DisciplineFile::Diary,
                DisciplineFile::Human,
                DisciplineFile::Plan,
                DisciplineFile::Jira,
                DisciplineFile::Notion,
                DisciplineFile::Eternal,
            ]
            .into_iter()
            .filter(|df| df.resolve_path(&v.root).is_some())
            .map(|df| (df, df.filename().to_string()))
            .collect(),
            None => Vec::new(),
        };
        if entries.is_empty() {
            return;
        }

        let theme = self.discipline_theme();
        let mut pending: Option<String> = None;
        let active_id = self.active_note.as_ref().map(|n| n.frontmatter.id.clone());
        egui::CollapsingHeader::new(RichText::new("DISCIPLINES").size(11.0).strong())
            .id_salt("disciplines_section")
            .default_open(true)
            .show(ui, |ui| {
                for (df, name) in &entries {
                    let id = self.vault.as_ref().and_then(|v| {
                        let path = df.resolve_path(&v.root)?;
                        v.notes
                            .iter()
                            .find(|n| n.path == path)
                            .map(|n| n.frontmatter.id.clone())
                    });
                    // The active sacred file is announced as selected to screen
                    // readers — the row is otherwise indistinguishable to AccessKit.
                    let selected = id.is_some() && id == active_id;
                    let label = format!("◈ {name}");
                    let (_, activated) =
                        crate::ui_a11y::clickable_row(ui, &theme, &label, selected, |ui| {
                            ui.label(RichText::new(&label).color(theme.text));
                        });
                    if activated {
                        if let Some(id) = id {
                            pending = Some(id);
                        }
                    }
                }
            });
        if let Some(id) = pending {
            // select_note clears central_overlay on success, so the chosen sacred
            // file surfaces in the editor even if Tickets/Timeline was open.
            self.select_note(&id);
        }
    }
}

/// Body of a discipline file taken from the in-memory `vault.notes` (no per-frame
/// disk I/O). Falls back to a one-off `read_raw` if the file resolves on disk but
/// isn't in `notes`. Empty string when the file is absent. The ticket parsers
/// only read table rows / `SCRUM-` lines, which live in the body, so the
/// frontmatter that `read_raw` would also include is irrelevant here.
fn discipline_content(v: &omninote_core::vault::Vault, file: DisciplineFile) -> String {
    match file.resolve_path(&v.root) {
        Some(path) => v
            .notes
            .iter()
            .find(|n| n.path == path)
            .map(|n| n.content.clone())
            .unwrap_or_else(|| discipline::read_raw(&v.root, file).unwrap_or_default()),
        None => String::new(),
    }
}

/// Whether a ticket passes the panel's status chip filter.
fn ticket_matches_filter(t: &Ticket, filter: crate::app::TicketFilter) -> bool {
    use crate::app::TicketFilter;
    let status = status_from_cell(&t.status);
    match filter {
        TicketFilter::All => true,
        TicketFilter::Doing => status == TaskStatus::Doing,
        TicketFilter::Todo => status == TaskStatus::Todo,
        TicketFilter::Done => status == TaskStatus::Done,
    }
}

/// A theme-tokened pill (accent wash). Used for DIARY labels and HUMAN Q-ids.
fn chip(ui: &mut egui::Ui, theme: &crate::theme::Theme, text: &str) {
    let size = crate::ui_a11y::scaled(ui, 11.0);
    egui::Frame::none()
        .fill(theme.row_selected())
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(6.0, 1.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(theme.accent).size(size));
        });
}

/// Status semaphore color, resolved through the theme's semantic accessors so
/// the high-contrast preset stays legible.
fn status_color(theme: &crate::theme::Theme, status: TaskStatus) -> egui::Color32 {
    match status {
        TaskStatus::Done => theme.status_done(),
        TaskStatus::Doing => theme.status_doing(),
        TaskStatus::Blocked => theme.status_blocked(),
        TaskStatus::Todo => theme.status_todo(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ─── status_from_cell ───

    #[test]
    fn status_from_cell_maps_all_buckets() {
        assert_eq!(status_from_cell("✅ Done"), TaskStatus::Done);
        assert_eq!(status_from_cell("✅ Concluída"), TaskStatus::Done);
        assert_eq!(status_from_cell("🎯 Pronta [x]"), TaskStatus::Done);
        assert_eq!(status_from_cell("🔄 In progress"), TaskStatus::Doing);
        assert_eq!(status_from_cell("🚧 Em obra"), TaskStatus::Doing);
        assert_eq!(status_from_cell("🔄 Parcial"), TaskStatus::Doing);
        assert_eq!(status_from_cell("⛔ blocked"), TaskStatus::Blocked);
        assert_eq!(status_from_cell("🌱 Backlog"), TaskStatus::Todo);
        assert_eq!(status_from_cell("A fazer"), TaskStatus::Todo);
        assert_eq!(status_from_cell("[ ]"), TaskStatus::Todo);
        // Unknown defaults to Todo.
        assert_eq!(status_from_cell("qualquer coisa"), TaskStatus::Todo);
        assert_eq!(status_from_cell(""), TaskStatus::Todo);
    }

    // ─── sprint_tasks ───

    #[test]
    fn sprint_tasks_parses_real_sprint_table() {
        let raw = "\
## §1 Tasks

| # | ID | Tarefa | Status | Prio | Estimativa | Notas |
|---|------|--------|--------|------|------------|-------|
| 1 | CAD-23.1 | RAG search | ✅ Done (PR #17) | ⚡ | 12h | mergeado |
| 6 | CAD-25 | UI Design v2 | 🔄 Em execução | ⚡ | ~50h | DESBLOQUEADO |
";
        let tasks = sprint_tasks(raw);
        assert_eq!(tasks.len(), 2);
        let t1 = tasks.iter().find(|t| t.id == "CAD-23.1").unwrap();
        assert_eq!(t1.status, TaskStatus::Done);
        let t6 = tasks.iter().find(|t| t.id == "CAD-25").unwrap();
        assert_eq!(t6.status, TaskStatus::Doing);
        assert_eq!(t6.title, "UI Design v2");
    }

    #[test]
    fn sprint_tasks_ignores_notion_schema_table() {
        // The NOTION Schema table `| Notion | Local |` has no ID/Status header,
        // so it must not be read as a task list.
        let raw = "\
| Notion | Local |
|--------|-------|
| ID auto (`CAD-XX`) | nome do arquivo |
| Status | tabela em SPRINT.md |
";
        assert!(sprint_tasks(raw).is_empty());
    }

    #[test]
    fn sprint_tasks_bullet_fallback() {
        let raw = "Sem tabela aqui.\n- [x] feito\n- [ ] a fazer\n- nada de checkbox\n";
        let tasks = sprint_tasks(raw);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert_eq!(tasks[0].title, "feito");
        assert_eq!(tasks[1].status, TaskStatus::Todo);
        assert_eq!(tasks[1].title, "a fazer");
    }

    #[test]
    fn sprint_tasks_table_present_suppresses_bullet_fallback() {
        // A real task table plus stray bullets elsewhere → only table rows count.
        let raw = "\
| ID | Tarefa | Status |
|----|--------|--------|
| CAD-1 | x | ✅ Done |

- [ ] this bullet must be ignored because a table was found
";
        let tasks = sprint_tasks(raw);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "CAD-1");
    }

    // ─── label_chips ───

    #[test]
    fn label_chips_extracts_dedups_skips_wikilinks_and_checkboxes() {
        let s = "## title [busy-loop-90cpu] body [busy-loop-90cpu] [pr-split] \
                 [[WikiLink]] - [x] done - [ ] todo [front-back-parallel]";
        let chips = label_chips(s);
        assert_eq!(
            chips,
            vec![
                "busy-loop-90cpu".to_string(),
                "pr-split".to_string(),
                "front-back-parallel".to_string()
            ]
        );
        assert!(!chips.iter().any(|c| c == "WikiLink"));
        assert!(!chips.iter().any(|c| c == "x"));
    }

    // ─── diary_days ───

    #[test]
    fn diary_days_groups_by_date_newest_first() {
        let raw = "\
# DIARY

---

## 2026-06-27 — close-out

**Lição [label-a]:** algo recente que é bem comprido de propósito para forçar o corte do snippet em oitenta caracteres exatamente aqui agora.

---

## 2026-06-03 — older

corpo antigo
";
        let days = diary_days(raw);
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date, "2026-06-27");
        assert_eq!(days[1].date, "2026-06-03");
        assert!(days[0].entries[0].labels.contains(&"label-a".to_string()));
    }

    #[test]
    fn diary_days_snippet_under_80_and_excludes_heading() {
        let raw = "---\n\n## 2026-06-27 — my title here\n\ncorpo curto\n";
        let days = diary_days(raw);
        assert_eq!(days.len(), 1);
        let snip = &days[0].entries[0].snippet;
        assert!(snip.chars().count() <= 80);
        assert!(!snip.contains("my title here"), "heading must be excluded");
        assert_eq!(snip, "corpo curto");
    }

    #[test]
    fn diary_days_long_snippet_truncated_to_80() {
        let body = "x".repeat(200);
        let raw = format!("---\n\n## 2026-06-27 — t\n\n{body}\n");
        let days = diary_days(&raw);
        assert_eq!(days[0].entries[0].snippet.chars().count(), 80);
        assert!(days[0].entries[0].snippet.ends_with('…'));
    }

    #[test]
    fn diary_days_ignores_non_date_chunks() {
        let raw = "---\n\n## Not a date heading\n\nbody\n";
        assert!(diary_days(raw).is_empty());
    }

    // ─── human_questions ───

    #[test]
    fn human_questions_splits_open_vs_resolved() {
        let raw = "\
# HUMAN

## Open questions

### Q-30 · Pergunta aberta · raised 2026-06-01

corpo

## Resolved

### Q-01 · Pergunta velha · raised 2026-05-01 · resolved 2026-05-22

corpo
";
        let qs = human_questions(raw);
        assert_eq!(qs.len(), 2);
        let q30 = qs.iter().find(|q| q.id == "Q-30").unwrap();
        assert!(!q30.resolved);
        assert_eq!(q30.question, "Pergunta aberta");
        let q01 = qs.iter().find(|q| q.id == "Q-01").unwrap();
        assert!(q01.resolved);
        assert_eq!(q01.question, "Pergunta velha");
    }

    #[test]
    fn human_questions_empty_and_sectionless_no_panic() {
        assert!(human_questions("").is_empty());
        assert!(human_questions("# HUMAN\n\njust prose, no questions\n").is_empty());
        // A question with no section marker defaults to open.
        let qs = human_questions("### Q-05 · solta\n");
        assert_eq!(qs.len(), 1);
        assert!(!qs[0].resolved);
    }

    // ─── tickets_merged ───

    #[test]
    fn tickets_merged_combines_providers_word_bounded() {
        let notion = "\
| ID | Title | Phase | Status | Priority |
|----|-------|-------|--------|----------|
| CAD-2 | Sidebar | v0.1 | 🚧 Em obra | ⚡ Alta |
| CAD-25 | UI v2 | v0.2 | 🌱 Backlog | ⚡ Alta |
";
        let jira = "### SCRUM-241 · panel UX\nbody\n";
        let tickets = tickets_merged(notion, jira);
        assert_eq!(tickets.len(), 3);
        // Word-bounded: CAD-2 and CAD-25 are distinct rows.
        assert!(tickets.iter().any(|t| t.id == "CAD-2"));
        assert!(tickets.iter().any(|t| t.id == "CAD-25"));
        let scrum = tickets.iter().find(|t| t.id == "SCRUM-241").unwrap();
        assert_eq!(scrum.provider, Provider::Jira);
        assert_eq!(scrum.title, "panel UX");
        let cad2 = tickets.iter().find(|t| t.id == "CAD-2").unwrap();
        assert_eq!(cad2.provider, Provider::Notion);
        assert_eq!(cad2.title, "Sidebar");
    }

    #[test]
    fn tickets_merged_sorts_doing_before_todo_before_done() {
        let notion = "\
| ID | Title | Status |
|----|-------|--------|
| CAD-9 | done one | ✅ Concluída |
| CAD-8 | todo one | 🌱 Backlog |
| CAD-7 | doing one | 🔄 In progress |
";
        let tickets = tickets_merged(notion, "");
        let order: Vec<&str> = tickets.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(order, vec!["CAD-7", "CAD-8", "CAD-9"]);
    }

    #[test]
    fn tickets_merged_tolerates_empty_jira() {
        let notion = "| ID | Title | Status |\n|--|--|--|\n| CAD-1 | x | 🌱 Backlog |\n";
        let tickets = tickets_merged(notion, "");
        assert_eq!(tickets.len(), 1);
    }

    // ─── discipline_file_of ───

    #[test]
    fn discipline_file_of_matches_filenames() {
        assert_eq!(
            discipline_file_of(&PathBuf::from("discipline/SPRINT.md")),
            Some(DisciplineFile::Sprint)
        );
        assert_eq!(
            discipline_file_of(&PathBuf::from("SPRINT.md")),
            Some(DisciplineFile::Sprint)
        );
        assert_eq!(
            discipline_file_of(&PathBuf::from("discipline/DIARY.md")),
            Some(DisciplineFile::Diary)
        );
        assert_eq!(discipline_file_of(&PathBuf::from("Notes/x.md")), None);
        assert_eq!(discipline_file_of(&PathBuf::from("sprint.md")), None);
    }

    #[test]
    fn discipline_file_of_rejects_non_sacred_location() {
        // A sacred *filename* in a non-sacred location is an ordinary note, not a
        // typed (read-only) view — guards against a stray Projetos/DIARY.md
        // hijacking the fork or the `+ Append entry` write. (triad-codex Slice 5.)
        assert_eq!(
            discipline_file_of(&PathBuf::from("Projetos/DIARY.md")),
            None
        );
        assert_eq!(
            discipline_file_of(&PathBuf::from("Archive/SPRINT.md")),
            None
        );
        assert_eq!(
            discipline_file_of(&PathBuf::from("discipline/sub/DIARY.md")),
            None
        );
        // The two sacred locations still resolve.
        assert_eq!(
            discipline_file_of(&PathBuf::from("DIARY.md")),
            Some(DisciplineFile::Diary)
        );
        assert_eq!(
            discipline_file_of(&PathBuf::from("discipline/DIARY.md")),
            Some(DisciplineFile::Diary)
        );
    }

    // ─── split_row / escaped pipes ───

    #[test]
    fn split_row_respects_escaped_pipe() {
        // A `\|` inside a cell is a literal pipe, not a column boundary, so the
        // Status column doesn't shift and the title keeps its pipe. (triad-codex.)
        let cells = split_row(r"| CAD-1 | Foo \| Bar | ✅ Done |").unwrap();
        assert_eq!(cells, vec!["CAD-1", "Foo | Bar", "✅ Done"]);
    }

    #[test]
    fn sprint_tasks_escaped_pipe_keeps_columns_aligned() {
        let raw = "\
| ID | Tarefa | Status |
|----|--------|--------|
| CAD-9 | Pipe \\| in title | ✅ Done |
";
        let tasks = sprint_tasks(raw);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "CAD-9");
        assert_eq!(tasks[0].title, "Pipe | in title");
        // Crucially the status column wasn't shifted by the escaped pipe.
        assert_eq!(tasks[0].status, TaskStatus::Done);
    }

    #[test]
    fn split_escaped_pipes_plain_and_trailing() {
        assert_eq!(split_escaped_pipes("a | b | c"), vec!["a", "b", "c"]);
        // A trailing escaped pipe produces a cell ending in '|'.
        assert_eq!(split_escaped_pipes(r"a \| "), vec!["a |"]);
        // A lone backslash (not before '|') is preserved.
        assert_eq!(split_escaped_pipes(r"a\b | c"), vec![r"a\b", "c"]);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn sprint_tasks_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = sprint_tasks(&raw);
        }

        #[test]
        fn label_chips_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = label_chips(&raw);
        }

        #[test]
        fn human_questions_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = human_questions(&raw);
        }

        #[test]
        fn tickets_merged_never_panics(a in proptest::prelude::any::<String>(), b in proptest::prelude::any::<String>()) {
            let _ = tickets_merged(&a, &b);
        }

        #[test]
        fn diary_days_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = diary_days(&raw);
        }

        #[test]
        fn split_escaped_pipes_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = split_escaped_pipes(&raw);
        }
    }
}
