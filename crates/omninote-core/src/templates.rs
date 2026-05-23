//! Template rendering for daily notes and `omninote template apply`. CAD-22.
//!
//! Pure-string `render()` substitutes `{{...}}` placeholders against a
//! [`TemplateContext`]. I/O helpers (`list_templates`, `load_template`) read
//! the vault's `Templates/` folder (Obsidian convention).
//!
//! Supported placeholders:
//! - `{{date}}` → `2026-05-23` (ISO `%Y-%m-%d`)
//! - `{{date:FMT}}` → chrono `strftime` format
//! - `{{time}}` → `14:32` (`%H:%M`)
//! - `{{time:FMT}}` → chrono `strftime` format
//! - `{{title}}` → `ctx.title`
//! - `{{KEY}}` → `ctx.extra.get(KEY)` if present, else kept literal
//!
//! Unknown / unparseable placeholders are kept verbatim — never panics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Context passed to [`render`]. Owned strings/dates so callers don't fight
/// the borrow checker when stitching multiple values together.
#[derive(Clone, Debug)]
pub struct TemplateContext {
    pub title: String,
    pub date: chrono::DateTime<chrono::Local>,
    /// Arbitrary `{{KEY}}` → value map. Lookup keys are case-sensitive.
    pub extra: HashMap<String, String>,
}

impl TemplateContext {
    /// Convenience: title + today's local datetime + empty extras.
    pub fn now(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            date: chrono::Local::now(),
            extra: HashMap::new(),
        }
    }
}

/// Metadata for one template file under `<vault>/Templates/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateMeta {
    /// Filename stem (no `.md`). Used as `template apply <NAME>` argument.
    pub name: String,
    pub path: PathBuf,
}

const DEFAULT_DATE_FMT: &str = "%Y-%m-%d";
const DEFAULT_TIME_FMT: &str = "%H:%M";

/// Render `template` substituting `{{...}}` placeholders. Never panics —
/// malformed input is kept literal.
pub fn render(template: &str, ctx: &TemplateContext) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for opening `{{`
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Find the matching `}}`. Earliest match wins.
            if let Some(end_rel) = find_close(&template[i + 2..]) {
                let inner = &template[i + 2..i + 2 + end_rel];
                match substitute(inner, ctx) {
                    Some(value) => {
                        out.push_str(&value);
                    }
                    None => {
                        // Unknown placeholder — keep literal `{{inner}}`
                        out.push_str("{{");
                        out.push_str(inner);
                        out.push_str("}}");
                    }
                }
                i = i + 2 + end_rel + 2;
                continue;
            }
        }
        // Plain char — copy as-is. Use char boundaries to keep UTF-8 valid.
        let ch_end = next_char_boundary(template, i);
        out.push_str(&template[i..ch_end]);
        i = ch_end;
    }
    out
}

/// Position of next `}}` in `s`, or None if absent. Returns byte offset of
/// the first `}`. Skips over any embedded `{{` (treats them as literal — we
/// don't support nested placeholders).
fn find_close(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'}' && b[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Return `Some(value)` for a recognized placeholder body, `None` otherwise.
fn substitute(inner: &str, ctx: &TemplateContext) -> Option<String> {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Split on first `:` to support `{{date:FMT}}` / `{{time:FMT}}`
    let (key, rest) = match trimmed.split_once(':') {
        Some((k, v)) => (k.trim(), Some(v)),
        None => (trimmed, None),
    };
    match key {
        "date" => {
            let fmt = rest.unwrap_or(DEFAULT_DATE_FMT);
            Some(format_date_safe(&ctx.date, fmt, DEFAULT_DATE_FMT))
        }
        "time" => {
            let fmt = rest.unwrap_or(DEFAULT_TIME_FMT);
            Some(format_date_safe(&ctx.date, fmt, DEFAULT_TIME_FMT))
        }
        "title" => Some(ctx.title.clone()),
        k => ctx.extra.get(k).cloned(),
    }
}

/// chrono's `format()` panics on invalid format strings via the `Display`
/// impl. Validate by calling `try_to_rfc3339` indirectly — we use the
/// `try_to_string` pattern by checking each format item. Simpler: catch
/// using `format::strftime::StrftimeItems` which surfaces errors before
/// rendering.
fn format_date_safe(dt: &chrono::DateTime<chrono::Local>, fmt: &str, fallback: &str) -> String {
    use chrono::format::StrftimeItems;
    // Parse items; bail to fallback if any item is `Item::Error`.
    let items: Vec<_> = StrftimeItems::new(fmt).collect();
    if items
        .iter()
        .any(|it| matches!(it, chrono::format::Item::Error))
    {
        return dt.format(fallback).to_string();
    }
    // `format_with_items` is panic-free for validated items.
    dt.format_with_items(items.into_iter()).to_string()
}

/// Walk to the next UTF-8 char boundary at or after `i`. `s` is a valid
/// `&str`, so the byte at `i` is the start of a char.
fn next_char_boundary(s: &str, i: usize) -> usize {
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

/// List `*.md` templates in `<vault>/Templates/`. Returns empty vec if
/// folder absent (no error — templates are optional).
pub fn list_templates(vault_root: &Path) -> Vec<TemplateMeta> {
    let dir = vault_root.join("Templates");
    let read = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        out.push(TemplateMeta { name, path });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Read `<vault>/Templates/<name>.md`. Error if missing or unreadable.
pub fn load_template(vault_root: &Path, name: &str) -> Result<String, String> {
    let safe = name.trim().trim_end_matches(".md");
    if safe.is_empty() {
        return Err("template name is empty".into());
    }
    let path = vault_root.join("Templates").join(format!("{safe}.md"));
    std::fs::read_to_string(&path).map_err(|e| format!("read template {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ctx_at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> TemplateContext {
        let dt = chrono::Local.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap();
        TemplateContext {
            title: "T".into(),
            date: dt,
            extra: HashMap::new(),
        }
    }

    #[test]
    fn render_empty_template_returns_empty() {
        assert_eq!(render("", &TemplateContext::now("x")), "");
    }

    #[test]
    fn render_no_placeholders_passes_through() {
        let s = "Hello\nWorld\n# Title\n- item";
        assert_eq!(render(s, &TemplateContext::now("x")), s);
    }

    #[test]
    fn render_date_default_iso() {
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        assert_eq!(render("{{date}}", &ctx), "2026-05-23");
    }

    #[test]
    fn render_date_custom_fmt() {
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        assert_eq!(render("{{date:%d/%m/%Y}}", &ctx), "23/05/2026");
    }

    #[test]
    fn render_time_default() {
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        assert_eq!(render("{{time}}", &ctx), "14:32");
    }

    #[test]
    fn render_time_with_seconds() {
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        assert_eq!(render("{{time:%H:%M:%S}}", &ctx), "14:32:00");
    }

    #[test]
    fn render_title_substitution() {
        let mut ctx = TemplateContext::now("My Note");
        ctx.title = "My Note".into();
        assert_eq!(render("# {{title}}", &ctx), "# My Note");
    }

    #[test]
    fn render_unknown_placeholder_kept_literal() {
        let ctx = TemplateContext::now("x");
        assert_eq!(render("{{unknown}}", &ctx), "{{unknown}}");
    }

    #[test]
    fn render_extra_var_substituted() {
        let mut ctx = TemplateContext::now("x");
        ctx.extra.insert("project".into(), "OmniNote".into());
        assert_eq!(render("Project: {{project}}", &ctx), "Project: OmniNote");
    }

    #[test]
    fn render_invalid_date_fmt_falls_back() {
        // chrono treats `%Q` as Item::Error → falls back to default ISO.
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        let out = render("{{date:%Q}}", &ctx);
        assert_eq!(out, "2026-05-23"); // default fallback
    }

    #[test]
    fn render_open_brace_without_close_kept_literal() {
        let ctx = TemplateContext::now("x");
        assert_eq!(render("{{open never closes", &ctx), "{{open never closes");
    }

    #[test]
    fn render_close_without_open_kept_literal() {
        let ctx = TemplateContext::now("x");
        assert_eq!(render("nothing }} here", &ctx), "nothing }} here");
    }

    #[test]
    fn render_multiple_placeholders_one_line() {
        let ctx = ctx_at(2026, 5, 23, 14, 32);
        let out = render("# {{date}} — {{time}}\n## {{title}}", &ctx);
        // title=T by default
        assert_eq!(out, "# 2026-05-23 — 14:32\n## T");
    }

    #[test]
    fn render_handles_utf8_safely() {
        let ctx = TemplateContext::now("Anão 日本");
        let out = render("Olá {{title}} café", &ctx);
        assert_eq!(out, "Olá Anão 日本 café");
    }

    #[test]
    fn render_empty_placeholder_kept_literal() {
        let ctx = TemplateContext::now("x");
        assert_eq!(render("{{}}", &ctx), "{{}}");
    }

    #[test]
    fn list_templates_returns_empty_when_no_folder() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(list_templates(tmp.path()).is_empty());
    }

    #[test]
    fn list_templates_picks_md_only_and_sorts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("zulu.md"), "z").unwrap();
        std::fs::write(dir.join("alpha.md"), "a").unwrap();
        std::fs::write(dir.join("readme.txt"), "ignore me").unwrap();
        let list = list_templates(tmp.path());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "alpha");
        assert_eq!(list[1].name, "zulu");
    }

    #[test]
    fn load_template_reads_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("daily.md"), "# {{date}}\n").unwrap();
        let got = load_template(tmp.path(), "daily").unwrap();
        assert_eq!(got, "# {{date}}\n");
    }

    #[test]
    fn load_template_accepts_dot_md_suffix() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("meeting.md"), "ok").unwrap();
        let got = load_template(tmp.path(), "meeting.md").unwrap();
        assert_eq!(got, "ok");
    }

    #[test]
    fn load_template_errs_on_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_template(tmp.path(), "nope").is_err());
    }

    #[test]
    fn load_template_errs_on_empty_name() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_template(tmp.path(), "").is_err());
        assert!(load_template(tmp.path(), "   ").is_err());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 256, ..proptest::test_runner::Config::default() })]

        #[test]
        fn render_never_panics(template in proptest::prelude::any::<String>(), title in proptest::prelude::any::<String>()) {
            let mut ctx = TemplateContext::now(title);
            ctx.extra.insert("k".into(), "v".into());
            let _ = render(&template, &ctx);
        }

        #[test]
        fn render_preserves_no_placeholder_strings(s in "[a-zA-Z0-9 \n.-]{0,200}") {
            // Strings without `{` should round-trip unchanged.
            let ctx = TemplateContext::now("t");
            proptest::prop_assert_eq!(render(&s, &ctx), s);
        }
    }
}
