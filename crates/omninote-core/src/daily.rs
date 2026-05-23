//! Daily notes engine. CAD-22.
//!
//! `ensure_daily()` creates `<vault>/<folder>/YYYY-MM-DD.md` if missing,
//! rendered from `<vault>/Templates/<template>.md` when available — or a
//! built-in starter when not. Idempotent: re-runs return `created: false`
//! plus the existing path.
//!
//! Filename uses ISO 8601 (`YYYY-MM-DD.md`) so directory listings sort
//! chronologically without metadata.

use crate::templates::{load_template, render, TemplateContext};
use chrono::{Datelike, NaiveDate};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Options for [`ensure_daily`]. All fields optional via `Default`.
#[derive(Clone, Debug)]
pub struct DailyOpts {
    /// `None` = today in local timezone.
    pub date: Option<NaiveDate>,
    /// Template name (without `.md`). `None` = `"daily"`.
    pub template_name: Option<String>,
    /// Folder name relative to vault root. Default `"Daily"`.
    pub folder: String,
}

impl Default for DailyOpts {
    fn default() -> Self {
        Self {
            date: None,
            template_name: None,
            folder: "Daily".into(),
        }
    }
}

/// Result of [`ensure_daily`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DailyResult {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Relative to vault root — useful for display + linking.
    pub rel_path: PathBuf,
    /// `false` if the file already existed before this call.
    pub created: bool,
    /// `Some(name)` if rendered from a template, `None` if from the built-in
    /// starter.
    pub template_used: Option<String>,
}

/// Default starter when no template is provided / found. Receives the same
/// `{{date}}` / `{{title}}` substitution treatment as user templates.
const STARTER_BODY: &str = "# {{date}}\n\n## Notas\n\n";

const DEFAULT_TEMPLATE_NAME: &str = "daily";

/// Create today's (or `opts.date`'s) daily note if missing. Returns the
/// resolved path either way. Folder + Templates dirs are auto-created on
/// first run so first-time users don't need manual setup.
pub fn ensure_daily(vault_root: &Path, opts: DailyOpts) -> Result<DailyResult, String> {
    let date = opts
        .date
        .unwrap_or_else(|| chrono::Local::now().date_naive());
    let filename = format!(
        "{:04}-{:02}-{:02}.md",
        date.year(),
        date.month(),
        date.day()
    );
    let folder_abs = vault_root.join(&opts.folder);
    std::fs::create_dir_all(&folder_abs).map_err(|e| format!("create daily folder: {e}"))?;

    let abs = folder_abs.join(&filename);
    let rel = Path::new(&opts.folder).join(&filename);

    if abs.exists() {
        return Ok(DailyResult {
            path: abs,
            rel_path: rel,
            created: false,
            template_used: detect_template_used(vault_root, &opts),
        });
    }

    let template_name = opts
        .template_name
        .clone()
        .unwrap_or_else(|| DEFAULT_TEMPLATE_NAME.into());
    let (body, used) = match load_template(vault_root, &template_name) {
        Ok(s) => (s, Some(template_name)),
        Err(_) => (STARTER_BODY.to_string(), None),
    };

    let local_dt = date
        .and_hms_opt(0, 0, 0)
        .and_then(|ndt| chrono::Local.from_local_datetime(&ndt).single())
        .unwrap_or_else(chrono::Local::now);

    let ctx = TemplateContext {
        title: filename.trim_end_matches(".md").to_string(),
        date: local_dt,
        extra: HashMap::new(),
    };
    let rendered = render(&body, &ctx);
    std::fs::write(&abs, rendered).map_err(|e| format!("write daily note: {e}"))?;

    Ok(DailyResult {
        path: abs,
        rel_path: rel,
        created: true,
        template_used: used,
    })
}

/// Reports `Some(template_name)` if a template exists for this `opts`. Used
/// to populate `template_used` on idempotent re-calls (file already exists).
fn detect_template_used(vault_root: &Path, opts: &DailyOpts) -> Option<String> {
    let name = opts
        .template_name
        .clone()
        .unwrap_or_else(|| DEFAULT_TEMPLATE_NAME.into());
    if load_template(vault_root, &name).is_ok() {
        Some(name)
    } else {
        None
    }
}

/// List all `YYYY-MM-DD.md` files under `<vault>/<folder>/`. Sorted oldest
/// first. Files not matching the ISO pattern are silently skipped. Used by
/// the calendar widget (CAD-25 Fase B) and CLI `discipline daily list`.
pub fn list_dailies(vault_root: &Path, folder: &str) -> Vec<NaiveDate> {
    let dir = vault_root.join(folder);
    let read = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut dates = Vec::new();
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if let Ok(d) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
            dates.push(d);
        }
    }
    dates.sort();
    dates
}

use chrono::TimeZone;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_today_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let res = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        assert!(res.created);
        assert!(res.path.exists());
        let body = std::fs::read_to_string(&res.path).unwrap();
        assert!(body.contains("# "));
        assert!(body.contains("## Notas"));
    }

    #[test]
    fn idempotent_on_second_call() {
        let tmp = tempfile::tempdir().unwrap();
        let first = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        let second = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.path, second.path);
    }

    #[test]
    fn uses_custom_template_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let tdir = tmp.path().join("Templates");
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(tdir.join("daily.md"), "# Daily: {{date}}\n\n- focus:").unwrap();
        let res = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        assert!(res.created);
        let body = std::fs::read_to_string(&res.path).unwrap();
        assert!(body.starts_with("# Daily: 2"));
        assert!(body.contains("- focus:"));
        assert_eq!(res.template_used.as_deref(), Some("daily"));
    }

    #[test]
    fn falls_back_to_starter_when_no_template() {
        let tmp = tempfile::tempdir().unwrap();
        let res = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        assert!(res.template_used.is_none());
    }

    #[test]
    fn respects_custom_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let opts = DailyOpts {
            folder: "journal/2026".into(),
            ..Default::default()
        };
        let res = ensure_daily(tmp.path(), opts).unwrap();
        assert!(res.path.starts_with(tmp.path().join("journal/2026")));
        assert!(res.rel_path.starts_with("journal/2026"));
    }

    #[test]
    fn respects_custom_date() {
        let tmp = tempfile::tempdir().unwrap();
        let date = NaiveDate::from_ymd_opt(2020, 1, 15).unwrap();
        let opts = DailyOpts {
            date: Some(date),
            ..Default::default()
        };
        let res = ensure_daily(tmp.path(), opts).unwrap();
        assert_eq!(res.path.file_name().unwrap(), "2020-01-15.md");
        let body = std::fs::read_to_string(&res.path).unwrap();
        assert!(body.contains("2020-01-15"));
    }

    #[test]
    fn respects_custom_template_name() {
        let tmp = tempfile::tempdir().unwrap();
        let tdir = tmp.path().join("Templates");
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(tdir.join("custom.md"), "CUSTOM {{date}}").unwrap();
        let opts = DailyOpts {
            template_name: Some("custom".into()),
            ..Default::default()
        };
        let res = ensure_daily(tmp.path(), opts).unwrap();
        let body = std::fs::read_to_string(&res.path).unwrap();
        assert!(body.starts_with("CUSTOM "));
        assert_eq!(res.template_used.as_deref(), Some("custom"));
    }

    #[test]
    fn idempotent_preserves_user_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let first = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        std::fs::write(&first.path, "EDITED BY USER\n").unwrap();
        let second = ensure_daily(tmp.path(), DailyOpts::default()).unwrap();
        assert!(!second.created);
        let body = std::fs::read_to_string(&second.path).unwrap();
        assert_eq!(body, "EDITED BY USER\n", "must not overwrite user edits");
    }

    #[test]
    fn list_dailies_returns_sorted_dates() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Daily");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("2026-05-23.md"), "x").unwrap();
        std::fs::write(dir.join("2026-05-01.md"), "x").unwrap();
        std::fs::write(dir.join("2025-12-31.md"), "x").unwrap();
        let dates = list_dailies(tmp.path(), "Daily");
        assert_eq!(dates.len(), 3);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
        assert_eq!(dates[2], NaiveDate::from_ymd_opt(2026, 5, 23).unwrap());
    }

    #[test]
    fn list_dailies_ignores_non_iso_filenames() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Daily");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("2026-05-23.md"), "x").unwrap();
        std::fs::write(dir.join("readme.md"), "x").unwrap();
        std::fs::write(dir.join("note.txt"), "x").unwrap();
        std::fs::write(dir.join("2026-13-99.md"), "x").unwrap(); // invalid date
        let dates = list_dailies(tmp.path(), "Daily");
        assert_eq!(dates.len(), 1);
    }

    #[test]
    fn list_dailies_empty_when_folder_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(list_dailies(tmp.path(), "Daily").is_empty());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 64, ..proptest::test_runner::Config::default() })]

        #[test]
        fn ensure_daily_never_panics(
            template_name in proptest::option::of("[a-zA-Z0-9_-]{1,20}"),
            folder in "[a-zA-Z0-9_./-]{1,30}",
        ) {
            let tmp = tempfile::tempdir().unwrap();
            let opts = DailyOpts {
                date: None,
                template_name,
                folder,
            };
            let _ = ensure_daily(tmp.path(), opts);
        }
    }
}
