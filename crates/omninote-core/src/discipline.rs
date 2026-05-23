//! Sacred file access — read/append the discipline files. CAD-22.
//!
//! "Sacred files" = the 6 (+1 optional) markdown files that make up the
//! per-project discipline protocol:
//!
//! | File         | Purpose                                  |
//! |--------------|------------------------------------------|
//! | DIARY.md     | append-only execution log                |
//! | SPRINT.md    | active sprint rules + ordered task list  |
//! | HUMAN.md     | open questions for the human             |
//! | PLAN.md      | active sprint plan log                   |
//! | JIRA.md      | ticket index (Atlassian projects)        |
//! | NOTION.md    | ticket index (Notion projects)           |
//! | ETERNAL.md   | optional cross-sprint project status     |
//!
//! Resolution order: `<vault>/discipline/<FILE>` → `<vault>/<FILE>`. Writes
//! prefer `discipline/` if that directory exists.
//!
//! Parsing is **loose by design** — discipline files are human-edited
//! markdown, not a schema. `read_entries()` splits on `---` separators and
//! captures the first `## ` heading per chunk; everything else is body.
//! Callers that need richer structure should grep the body themselves.

use chrono::Datelike;
use std::path::{Path, PathBuf};

/// One of the 7 known discipline files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DisciplineFile {
    Diary,
    Sprint,
    Human,
    Plan,
    Jira,
    Notion,
    Eternal,
}

impl DisciplineFile {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Diary => "DIARY.md",
            Self::Sprint => "SPRINT.md",
            Self::Human => "HUMAN.md",
            Self::Plan => "PLAN.md",
            Self::Jira => "JIRA.md",
            Self::Notion => "NOTION.md",
            Self::Eternal => "ETERNAL.md",
        }
    }

    /// Parse from CLI-friendly slug (case-insensitive: `diary`, `sprint`, …).
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug.to_ascii_lowercase().as_str() {
            "diary" => Some(Self::Diary),
            "sprint" => Some(Self::Sprint),
            "human" => Some(Self::Human),
            "plan" => Some(Self::Plan),
            "jira" => Some(Self::Jira),
            "notion" => Some(Self::Notion),
            "eternal" => Some(Self::Eternal),
            _ => None,
        }
    }

    /// Find the file on disk. Prefers `<vault>/discipline/<FILE>`; falls
    /// back to `<vault>/<FILE>`. Returns `None` if neither exists.
    pub fn resolve_path(&self, vault_root: &Path) -> Option<PathBuf> {
        let in_dir = vault_root.join("discipline").join(self.filename());
        if in_dir.exists() {
            return Some(in_dir);
        }
        let in_root = vault_root.join(self.filename());
        if in_root.exists() {
            return Some(in_root);
        }
        None
    }

    /// Where to *write* — used when the file doesn't exist yet. Prefers
    /// `<vault>/discipline/` if that directory exists; otherwise root.
    pub fn write_target(&self, vault_root: &Path) -> PathBuf {
        let dir = vault_root.join("discipline");
        if dir.is_dir() {
            dir.join(self.filename())
        } else {
            vault_root.join(self.filename())
        }
    }
}

/// One semantic chunk of a discipline file — separated from neighbors by a
/// `---` line. `heading` is the first `## ` line in the chunk (empty if
/// none); `body` is everything else, trimmed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisciplineEntry {
    pub heading: String,
    pub body: String,
}

/// Pointer into a discipline file where a ticket ID was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketRef {
    pub ticket_id: String,
    pub file: PathBuf,
    /// 1-based line number of the first match.
    pub line_no: usize,
    /// A few lines of context around the match (current line + up to 4
    /// after, stopping at the next blank line).
    pub paragraph: String,
}

/// Read the full file content. Errors if the file doesn't exist.
pub fn read_raw(vault_root: &Path, file: DisciplineFile) -> Result<String, String> {
    let path = file
        .resolve_path(vault_root)
        .ok_or_else(|| format!("discipline file not found: {}", file.filename()))?;
    std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))
}

/// Split a discipline file into chunks. Empty file → `Vec::new()`.
pub fn read_entries(
    vault_root: &Path,
    file: DisciplineFile,
) -> Result<Vec<DisciplineEntry>, String> {
    let raw = read_raw(vault_root, file)?;
    Ok(parse_entries(&raw))
}

/// Parser exposed for testing without a vault on disk.
pub fn parse_entries(raw: &str) -> Vec<DisciplineEntry> {
    // Split on lines that are exactly `---` (after trimming).
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in raw.lines() {
        if line.trim() == "---" {
            if !cur.is_empty() || !chunks.is_empty() {
                chunks.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push_str(line);
            cur.push('\n');
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
        .into_iter()
        .map(|chunk| {
            let heading = chunk
                .lines()
                .find(|l| l.starts_with("## "))
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .unwrap_or_default();
            DisciplineEntry {
                heading,
                body: chunk.trim().to_string(),
            }
        })
        .filter(|e| !(e.heading.is_empty() && e.body.is_empty()))
        .collect()
}

/// Append behavior depends on the file:
/// - DIARY: prepend (newest at top)
/// - HUMAN: insert under `## Open questions`
/// - others: append at end
///
/// Returns the path actually written to.
pub fn append_entry(
    vault_root: &Path,
    file: DisciplineFile,
    entry: &DisciplineEntry,
) -> Result<PathBuf, String> {
    let target = file
        .resolve_path(vault_root)
        .unwrap_or_else(|| file.write_target(vault_root));
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create parent for {}: {e}", target.display()))?;
    }
    let current = std::fs::read_to_string(&target).unwrap_or_default();
    let updated = match file {
        DisciplineFile::Diary => prepend_to_diary(&current, file, entry),
        DisciplineFile::Human => insert_into_human_open(&current, file, entry),
        _ => append_to_tail(&current, file, entry),
    };
    std::fs::write(&target, updated).map_err(|e| format!("write {}: {e}", target.display()))?;
    Ok(target)
}

/// Quick DIARY entry: today's date + 1-line note. Optionally tagged with a
/// ticket reference. Idempotency: each call creates a new entry — no dedup.
pub fn diary_quick(vault_root: &Path, text: &str, ticket: Option<&str>) -> Result<PathBuf, String> {
    let today = chrono::Local::now().date_naive();
    let heading = format!(
        "{:04}-{:02}-{:02} — quick entry",
        today.year(),
        today.month(),
        today.day()
    );
    let mut body_lines = String::new();
    if let Some(t) = ticket {
        body_lines.push_str(&format!("**Tickets touched:** {t}\n\n"));
    }
    body_lines.push_str(text.trim());
    body_lines.push('\n');
    let entry = DisciplineEntry {
        heading: heading.clone(),
        body: format!("## {heading}\n\n{body_lines}"),
    };
    append_entry(vault_root, DisciplineFile::Diary, &entry)
}

/// Append a new question to HUMAN.md, auto-numbering it `Q-NN` (next free
/// integer across all `### Q-\d+` occurrences). Returns the file path and
/// the assigned Q-NN label.
pub fn human_ask(vault_root: &Path, question: &str) -> Result<(PathBuf, String), String> {
    let current = read_raw(vault_root, DisciplineFile::Human).unwrap_or_default();
    let next = next_question_number(&current);
    let qn = format!("Q-{:02}", next);
    let today = chrono::Local::now().date_naive();
    let heading_line = format!(
        "{qn} · raised {:04}-{:02}-{:02}",
        today.year(),
        today.month(),
        today.day()
    );
    let block = format!("### {heading_line}\n\n**Pergunta:** {}\n", question.trim());
    let entry = DisciplineEntry {
        heading: heading_line,
        body: block,
    };
    let path = append_entry(vault_root, DisciplineFile::Human, &entry)?;
    Ok((path, qn))
}

/// Scan NOTION.md (preferred) then JIRA.md for a word-bounded match of
/// `ticket_id` (e.g. `CAD-22` won't match `CAD-2`). Returns first hit.
pub fn ticket_status(vault_root: &Path, ticket_id: &str) -> Option<TicketRef> {
    for file in [DisciplineFile::Notion, DisciplineFile::Jira] {
        let path = match file.resolve_path(vault_root) {
            Some(p) => p,
            None => continue,
        };
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some((line_no, paragraph)) = find_ticket(&raw, ticket_id) {
            return Some(TicketRef {
                ticket_id: ticket_id.to_string(),
                file: path,
                line_no,
                paragraph,
            });
        }
    }
    None
}

// ──────────────────────── internals ────────────────────────

fn prepend_to_diary(current: &str, _file: DisciplineFile, entry: &DisciplineEntry) -> String {
    let header = ensure_default_header(current, DisciplineFile::Diary);
    let body = format_entry_body(entry);
    if let Some(idx) = find_first_h2_line(&header) {
        let mut out = String::with_capacity(header.len() + body.len() + 16);
        out.push_str(&header[..idx]);
        out.push_str(&body);
        out.push_str("\n---\n\n");
        out.push_str(&header[idx..]);
        out
    } else {
        // No existing entries — append after the preamble (which itself ends
        // with `---\n\n` per default header).
        let mut out = header;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&body);
        out.push('\n');
        out
    }
}

fn insert_into_human_open(current: &str, _file: DisciplineFile, entry: &DisciplineEntry) -> String {
    let mut header = ensure_default_header(current, DisciplineFile::Human);
    let block = format_entry_body(entry);
    let open_marker = "## Open questions";
    let placeholder = "_(nenhuma";
    if let Some(open_idx) = header.find(open_marker) {
        // Find the end of the line for `## Open questions`.
        let after_open = open_idx + open_marker.len();
        let mut cursor = after_open;
        // Skip to end of line
        if let Some(nl) = header[cursor..].find('\n') {
            cursor += nl + 1;
        }
        // Skip blank lines
        while header[cursor..].starts_with('\n') {
            cursor += 1;
        }
        // If a placeholder `_(nenhuma...)_` line exists immediately after,
        // remove it.
        if header[cursor..].starts_with(placeholder) {
            if let Some(nl) = header[cursor..].find('\n') {
                let remove_end = cursor + nl + 1;
                header.replace_range(cursor..remove_end, "");
            }
        }
        let mut out = String::with_capacity(header.len() + block.len() + 8);
        out.push_str(&header[..cursor]);
        out.push_str(&block);
        if !block.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&header[cursor..]);
        out
    } else {
        // No "Open questions" section — append to end.
        append_to_tail(&header, DisciplineFile::Human, entry)
    }
}

fn append_to_tail(current: &str, file: DisciplineFile, entry: &DisciplineEntry) -> String {
    let mut header = ensure_default_header(current, file);
    if !header.ends_with('\n') {
        header.push('\n');
    }
    if !header.ends_with("\n\n") {
        header.push('\n');
    }
    header.push_str("---\n\n");
    header.push_str(&format_entry_body(entry));
    if !header.ends_with('\n') {
        header.push('\n');
    }
    header
}

fn format_entry_body(entry: &DisciplineEntry) -> String {
    let body = entry.body.trim();
    if body.starts_with("## ") || body.starts_with("### ") {
        // Caller already provided a heading line — use body as-is.
        body.to_string()
    } else if entry.heading.is_empty() {
        body.to_string()
    } else {
        format!("## {}\n\n{body}", entry.heading.trim())
    }
}

fn ensure_default_header(current: &str, file: DisciplineFile) -> String {
    if !current.trim().is_empty() {
        return current.to_string();
    }
    match file {
        DisciplineFile::Diary => {
            "# DIARY\n\n> Append-only execution log. Newest entry at top.\n\n---\n\n".to_string()
        }
        DisciplineFile::Sprint => "# SPRINT\n\n".to_string(),
        DisciplineFile::Human => "# HUMAN — Perguntas pro humano\n\n\
            > Apenas decisões irreversíveis, contratos externos, segurança ou conflitos.\n\n\
            ---\n\n\
            ## Open questions\n\n\
            ---\n\n\
            ## Resolved\n\n"
            .to_string(),
        DisciplineFile::Plan => "# PLAN\n\n".to_string(),
        DisciplineFile::Jira => "# JIRA\n\n".to_string(),
        DisciplineFile::Notion => "# NOTION\n\n".to_string(),
        DisciplineFile::Eternal => "# ETERNAL\n\n".to_string(),
    }
}

fn find_first_h2_line(s: &str) -> Option<usize> {
    // Find the byte offset of the first line starting with `## `.
    let mut offset = 0;
    for line in s.split_inclusive('\n') {
        let trimmed = line.trim_start_matches('\u{feff}');
        if trimmed.starts_with("## ") {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

fn next_question_number(human_raw: &str) -> u32 {
    let mut max = 0u32;
    for line in human_raw.lines() {
        // Match patterns like `### Q-12 · …` or `### Q-12`
        let l = line.trim_start_matches('#').trim_start();
        if let Some(rest) = l.strip_prefix("Q-") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u32>() {
                if n > max {
                    max = n;
                }
            }
        }
    }
    max + 1
}

fn find_ticket(raw: &str, ticket_id: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = raw.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        if word_bounded_contains(line, ticket_id) {
            // Capture a small paragraph: this line + up to 4 lines after,
            // stopping at the first blank line.
            let mut para = String::new();
            para.push_str(line);
            para.push('\n');
            for follow in lines.iter().skip(idx + 1).take(4) {
                if follow.trim().is_empty() {
                    break;
                }
                para.push_str(follow);
                para.push('\n');
            }
            return Some((idx + 1, para.trim_end().to_string()));
        }
    }
    None
}

fn word_bounded_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let nb = needle.as_bytes();
    let hb = haystack.as_bytes();
    if hb.len() < nb.len() {
        return false;
    }
    for start in 0..=(hb.len() - nb.len()) {
        if &hb[start..start + nb.len()] == nb {
            let before_ok = start == 0 || !is_id_char(hb[start - 1]);
            let end = start + nb.len();
            let after_ok = end == hb.len() || !is_id_char(hb[end]);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn is_id_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with_discipline_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("discipline")).unwrap();
        tmp
    }

    // ─── resolve_path / write_target ───

    #[test]
    fn resolve_path_prefers_discipline_subfolder() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(tmp.path().join("DIARY.md"), "ROOT").unwrap();
        std::fs::write(tmp.path().join("discipline/DIARY.md"), "SUB").unwrap();
        let p = DisciplineFile::Diary.resolve_path(tmp.path()).unwrap();
        assert!(p.ends_with("discipline/DIARY.md"));
    }

    #[test]
    fn resolve_path_falls_back_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("DIARY.md"), "ROOT").unwrap();
        let p = DisciplineFile::Diary.resolve_path(tmp.path()).unwrap();
        assert_eq!(p, tmp.path().join("DIARY.md"));
    }

    #[test]
    fn resolve_path_returns_none_if_neither_exists() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(DisciplineFile::Eternal.resolve_path(tmp.path()).is_none());
    }

    #[test]
    fn write_target_prefers_discipline_subfolder_when_dir_exists() {
        let tmp = vault_with_discipline_dir();
        let p = DisciplineFile::Sprint.write_target(tmp.path());
        assert!(p.ends_with("discipline/SPRINT.md"));
    }

    #[test]
    fn write_target_falls_back_to_root_when_no_subfolder() {
        let tmp = tempfile::tempdir().unwrap();
        let p = DisciplineFile::Sprint.write_target(tmp.path());
        assert_eq!(p, tmp.path().join("SPRINT.md"));
    }

    #[test]
    fn from_slug_round_trip() {
        assert_eq!(
            DisciplineFile::from_slug("diary"),
            Some(DisciplineFile::Diary)
        );
        assert_eq!(
            DisciplineFile::from_slug("DIARY"),
            Some(DisciplineFile::Diary)
        );
        assert_eq!(
            DisciplineFile::from_slug("eternal"),
            Some(DisciplineFile::Eternal)
        );
        assert_eq!(DisciplineFile::from_slug("nope"), None);
    }

    // ─── parse_entries ───

    #[test]
    fn parse_entries_splits_on_triple_dash() {
        let raw = "# Header\n\n---\n\n## E1\nbody1\n\n---\n\n## E2\nbody2\n";
        let entries = parse_entries(raw);
        assert_eq!(entries.len(), 3); // header chunk + 2 entries
        assert_eq!(entries[0].heading, "");
        assert_eq!(entries[1].heading, "E1");
        assert_eq!(entries[2].heading, "E2");
    }

    #[test]
    fn parse_entries_handles_no_entries() {
        assert!(parse_entries("").is_empty());
        assert_eq!(parse_entries("# Title\n").len(), 1);
    }

    #[test]
    fn parse_entries_extracts_h2_heading() {
        let raw = "---\n\n## My Title\n\nbody here\n";
        let e = parse_entries(raw);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].heading, "My Title");
    }

    // ─── append + diary_quick ───

    #[test]
    fn diary_quick_creates_file_if_missing() {
        let tmp = vault_with_discipline_dir();
        let path = diary_quick(tmp.path(), "first entry", None).unwrap();
        assert!(path.exists());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("first entry"));
        assert!(body.contains("# DIARY"));
    }

    #[test]
    fn diary_quick_prepends_above_existing_entries() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/DIARY.md"),
            "# DIARY\n\n> log\n\n---\n\n## 2026-05-01 — old\n\nold body\n",
        )
        .unwrap();
        let path = diary_quick(tmp.path(), "fresh news", Some("CAD-22")).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        let new_idx = body.find("fresh news").unwrap();
        let old_idx = body.find("old body").unwrap();
        assert!(new_idx < old_idx, "new entry must appear above old one");
        assert!(body.contains("**Tickets touched:** CAD-22"));
    }

    #[test]
    fn diary_quick_ticket_inserted_when_provided() {
        let tmp = vault_with_discipline_dir();
        let path = diary_quick(tmp.path(), "did the thing", Some("CAD-22")).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("CAD-22"));
    }

    // ─── human_ask ───

    #[test]
    fn human_ask_starts_at_q01_when_empty() {
        let tmp = vault_with_discipline_dir();
        let (_p, qn) = human_ask(tmp.path(), "Como decidir X?").unwrap();
        assert_eq!(qn, "Q-01");
    }

    #[test]
    fn human_ask_increments_past_existing() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/HUMAN.md"),
            "# HUMAN\n\n## Open questions\n\n### Q-07 · old\nbody\n\n## Resolved\n\n### Q-05 · res\nx\n",
        )
        .unwrap();
        let (_p, qn) = human_ask(tmp.path(), "nova").unwrap();
        assert_eq!(qn, "Q-08");
    }

    #[test]
    fn human_ask_replaces_nenhuma_placeholder() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/HUMAN.md"),
            "# HUMAN\n\n---\n\n## Open questions\n\n_(nenhuma — tudo respondido)_\n\n---\n\n## Resolved\n\n",
        )
        .unwrap();
        let (path, qn) = human_ask(tmp.path(), "Vale fazer X?").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(qn, "Q-01");
        assert!(body.contains("Vale fazer X?"));
        assert!(!body.contains("_(nenhuma"), "placeholder must be removed");
    }

    #[test]
    fn human_ask_appends_when_no_open_section() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(tmp.path().join("discipline/HUMAN.md"), "# HUMAN\n").unwrap();
        let (path, qn) = human_ask(tmp.path(), "tinha que perguntar").unwrap();
        assert_eq!(qn, "Q-01");
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("tinha que perguntar"));
    }

    // ─── append_entry ───

    #[test]
    fn append_entry_sprint_appends_at_end() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/SPRINT.md"),
            "# SPRINT\n\n## §0\nrules\n",
        )
        .unwrap();
        let entry = DisciplineEntry {
            heading: "§99 new rule".into(),
            body: "the rule body".into(),
        };
        append_entry(tmp.path(), DisciplineFile::Sprint, &entry).unwrap();
        let body = std::fs::read_to_string(tmp.path().join("discipline/SPRINT.md")).unwrap();
        let rules_idx = body.find("rules").unwrap();
        let new_idx = body.find("the rule body").unwrap();
        assert!(rules_idx < new_idx, "new entry must come after existing");
    }

    // ─── ticket_status ───

    #[test]
    fn ticket_status_finds_in_notion() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/NOTION.md"),
            "# NOTION\n\n| CAD-22 | Daily notes | In Progress |\n| CAD-23 | AI | Backlog |\n",
        )
        .unwrap();
        let t = ticket_status(tmp.path(), "CAD-22").unwrap();
        assert_eq!(t.ticket_id, "CAD-22");
        assert!(t.paragraph.contains("Daily notes"));
        assert_eq!(t.line_no, 3);
    }

    #[test]
    fn ticket_status_finds_in_jira_when_no_notion() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/JIRA.md"),
            "# JIRA\n\nSCRUM-157 · panel UX\n",
        )
        .unwrap();
        let t = ticket_status(tmp.path(), "SCRUM-157").unwrap();
        assert!(t.paragraph.contains("panel UX"));
    }

    #[test]
    fn ticket_status_word_boundary_distinguishes_cad22_from_cad2() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(
            tmp.path().join("discipline/NOTION.md"),
            "# NOTION\n\nCAD-2 done\nCAD-22 in progress\n",
        )
        .unwrap();
        let t = ticket_status(tmp.path(), "CAD-2").unwrap();
        assert!(t.paragraph.contains("CAD-2 done"));
        let t2 = ticket_status(tmp.path(), "CAD-22").unwrap();
        assert!(t2.paragraph.contains("CAD-22 in progress"));
        // Reverse-check: searching CAD-2 should NOT match the CAD-22 line first.
        let t = ticket_status(tmp.path(), "CAD-2").unwrap();
        assert_eq!(
            t.line_no, 3,
            "must hit 'CAD-2 done', not 'CAD-22 in progress'"
        );
    }

    #[test]
    fn ticket_status_returns_none_on_miss() {
        let tmp = vault_with_discipline_dir();
        std::fs::write(tmp.path().join("discipline/NOTION.md"), "# NOTION\n").unwrap();
        assert!(ticket_status(tmp.path(), "CAD-999").is_none());
    }

    // ─── small helpers ───

    #[test]
    fn word_bounded_handles_edges() {
        assert!(word_bounded_contains("CAD-2 here", "CAD-2"));
        assert!(word_bounded_contains("at end CAD-2", "CAD-2"));
        assert!(!word_bounded_contains("CAD-22", "CAD-2"));
        assert!(!word_bounded_contains("xCAD-2", "CAD-2"));
    }

    #[test]
    fn next_question_number_handles_gaps_and_higher_max() {
        let raw = "### Q-01 · x\n### Q-03 · y\n### Q-07 · z\n";
        assert_eq!(next_question_number(raw), 8);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn parse_entries_never_panics(raw in proptest::prelude::any::<String>()) {
            let _ = parse_entries(&raw);
        }

        #[test]
        fn word_bounded_contains_never_panics(h in proptest::prelude::any::<String>(), n in proptest::prelude::any::<String>()) {
            let _ = word_bounded_contains(&h, &n);
        }
    }
}
