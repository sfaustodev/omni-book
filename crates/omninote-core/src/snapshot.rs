//! Snapshot diff: what changed in the vault recently, via `git log`.
//!
//! `omninote diff [--since 1d|7d]` shells out to `git` inside the vault root
//! and summarizes the changed files. Vaults that are not git repos degrade
//! gracefully: [`diff_since`] returns a [`SnapshotReport`] with `is_git=false`
//! and an empty change list instead of erroring.
//!
//! The parsing of the `--name-status` porcelain is pure and unit-tested; the
//! actual `git` invocation is a thin wrapper around it.

use serde::Serialize;
use std::path::Path;
use std::process::Command;

/// Field separator emitted between commit header fields (ASCII Unit Separator).
/// `git log` writes it literally with `%x1f`; never appears in paths.
const FIELD_SEP: char = '\u{1f}';

/// One changed file within the window. `status` is git's porcelain letter
/// (`A`/`M`/`D`/`R`/`C`/`T`). For renames/copies `old_path` is the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeEntry {
    pub status: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
}

/// Aggregated result of a diff window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotReport {
    /// The `--since` expression git was asked for (e.g. `"1 day ago"`).
    pub since: String,
    /// `false` when the vault is not a git work tree.
    pub is_git: bool,
    /// Number of commits in the window.
    pub commits: usize,
    /// Distinct changed files, most-recently-touched first.
    pub changed: Vec<ChangeEntry>,
}

/// Translate a short window token into a git `--since` value.
///
/// Accepts `<N><unit>` where unit is `d`(ay), `w`(eek), `h`(our), `m`(onth),
/// `y`(ear) — e.g. `1d`, `7d`, `2w`. Bare integers mean days. Anything else is
/// passed through verbatim so callers can hand git native phrases like
/// `"yesterday"`. Returns `None` only for empty input.
pub fn parse_since(token: &str) -> Option<String> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    if let Ok(n) = t.parse::<u64>() {
        return Some(format!("{n} days ago"));
    }
    // Split off the last *char* (not byte) so a single multibyte token like
    // "é" can't slice mid-codepoint and panic.
    let unit = t.chars().next_back().expect("t is non-empty");
    let num = &t[..t.len() - unit.len_utf8()];
    if let Ok(n) = num.parse::<u64>() {
        let word = match unit {
            'd' | 'D' => "day",
            'w' | 'W' => "week",
            'h' | 'H' => "hour",
            'm' => "month",
            'y' | 'Y' => "year",
            _ => return Some(t.to_string()),
        };
        return Some(format!("{n} {word}s ago"));
    }
    Some(t.to_string())
}

/// Parse `git log --name-status` output into a deduplicated change list and a
/// commit count.
///
/// Expects each commit to be preceded by a header line of the form
/// `<sep>HASH<FIELD_SEP>DATE<FIELD_SEP>SUBJECT` (produced by a `--pretty`
/// format whose first char is `sep`), followed by tab-separated name-status
/// rows. The leading `sep` lets us distinguish header lines from status rows
/// without ambiguity. The first occurrence of each path wins, so the newest
/// change to a file is the one reported.
pub fn parse_name_status(output: &str, header_sentinel: char) -> (usize, Vec<ChangeEntry>) {
    let mut commits = 0usize;
    let mut changed: Vec<ChangeEntry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for raw in output.lines() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.starts_with(header_sentinel) {
            commits += 1;
            continue;
        }
        if let Some(entry) = parse_status_line(line) {
            if seen.insert(entry.path.clone()) {
                changed.push(entry);
            }
        }
    }
    (commits, changed)
}

/// Parse a single tab-separated `--name-status` row. Returns `None` for rows
/// that don't look like a status line.
fn parse_status_line(line: &str) -> Option<ChangeEntry> {
    let mut cols = line.split('\t');
    let status_raw = cols.next()?.trim();
    if status_raw.is_empty() {
        return None;
    }
    // Rename/copy carry a similarity score suffix, e.g. `R100`, `C75`.
    let kind = status_raw.chars().next()?;
    match kind {
        'R' | 'C' => {
            let old_path = cols.next()?.to_string();
            let new_path = cols.next()?.to_string();
            Some(ChangeEntry {
                status: kind.to_string(),
                path: new_path,
                old_path: Some(old_path),
            })
        }
        'A' | 'M' | 'D' | 'T' | 'U' => {
            let path = cols.next()?.to_string();
            if path.is_empty() {
                return None;
            }
            Some(ChangeEntry {
                status: kind.to_string(),
                path,
                old_path: None,
            })
        }
        _ => None,
    }
}

/// Whether `dir` is inside a git work tree.
pub fn is_git_repo(dir: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success() && o.stdout.starts_with(b"true"))
        .unwrap_or(false)
}

/// Run the diff for `since` (a token like `1d`) inside `vault_root`.
///
/// Non-git vaults return `is_git=false` with no changes — never an error. A
/// genuine git failure (e.g. corrupt repo) surfaces as `Err`.
pub fn diff_since(vault_root: &Path, since_token: &str) -> Result<SnapshotReport, String> {
    let since = parse_since(since_token).unwrap_or_else(|| "7 days ago".to_string());

    if !is_git_repo(vault_root) {
        return Ok(SnapshotReport {
            since,
            is_git: false,
            commits: 0,
            changed: Vec::new(),
        });
    }

    let pretty = format!("--pretty=format:{FIELD_SEP}%H{FIELD_SEP}%aI{FIELD_SEP}%s");
    let output = Command::new("git")
        .arg("-C")
        .arg(vault_root)
        .arg("log")
        .arg(format!("--since={since}"))
        .arg("--name-status")
        .arg(&pretty)
        .output()
        .map_err(|e| format!("run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed: {}", stderr.trim()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let (commits, changed) = parse_name_status(&text, FIELD_SEP);
    Ok(SnapshotReport {
        since,
        is_git: true,
        commits,
        changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_bare_int_is_days() {
        assert_eq!(parse_since("1").as_deref(), Some("1 days ago"));
        assert_eq!(parse_since("30").as_deref(), Some("30 days ago"));
    }

    #[test]
    fn parse_since_day_token() {
        assert_eq!(parse_since("1d").as_deref(), Some("1 days ago"));
        assert_eq!(parse_since("7d").as_deref(), Some("7 days ago"));
        assert_eq!(parse_since("7D").as_deref(), Some("7 days ago"));
    }

    #[test]
    fn parse_since_other_units() {
        assert_eq!(parse_since("2w").as_deref(), Some("2 weeks ago"));
        assert_eq!(parse_since("12h").as_deref(), Some("12 hours ago"));
        assert_eq!(parse_since("3m").as_deref(), Some("3 months ago"));
        assert_eq!(parse_since("1y").as_deref(), Some("1 years ago"));
    }

    #[test]
    fn parse_since_passthrough_for_phrases() {
        assert_eq!(parse_since("yesterday").as_deref(), Some("yesterday"));
        // Unknown unit letter passes through verbatim.
        assert_eq!(parse_since("5x").as_deref(), Some("5x"));
    }

    #[test]
    fn parse_since_multibyte_does_not_panic() {
        // Regression: a single multibyte char must not slice mid-codepoint.
        assert_eq!(parse_since("é").as_deref(), Some("é"));
        assert_eq!(parse_since("界").as_deref(), Some("界"));
        assert_eq!(parse_since("🦀").as_deref(), Some("🦀"));
        // Multibyte trailing char with a numeric prefix also passes through.
        assert_eq!(parse_since("7é").as_deref(), Some("7é"));
    }

    #[test]
    fn parse_since_empty_is_none() {
        assert!(parse_since("").is_none());
        assert!(parse_since("   ").is_none());
    }

    #[test]
    fn parse_since_trims_whitespace() {
        assert_eq!(parse_since("  7d  ").as_deref(), Some("7 days ago"));
    }

    const SEP: char = '\u{1f}';

    fn header(hash: &str) -> String {
        format!("{SEP}{hash}{SEP}2026-05-30T10:00:00+00:00{SEP}subject line")
    }

    #[test]
    fn parse_name_status_counts_commits_and_files() {
        let out = format!(
            "{h1}\nA\tnotes/a.md\nM\tnotes/b.md\n{h2}\nD\tnotes/c.md\n",
            h1 = header("abc"),
            h2 = header("def"),
        );
        let (commits, changed) = parse_name_status(&out, SEP);
        assert_eq!(commits, 2);
        assert_eq!(changed.len(), 3);
        assert_eq!(changed[0].status, "A");
        assert_eq!(changed[0].path, "notes/a.md");
        assert_eq!(changed[2].status, "D");
    }

    #[test]
    fn parse_name_status_dedups_keeping_first() {
        let out = format!(
            "{h1}\nM\tnotes/a.md\n{h2}\nA\tnotes/a.md\n",
            h1 = header("new"),
            h2 = header("old"),
        );
        let (commits, changed) = parse_name_status(&out, SEP);
        assert_eq!(commits, 2);
        assert_eq!(changed.len(), 1);
        // First (newest) wins → status M from the top commit.
        assert_eq!(changed[0].status, "M");
    }

    #[test]
    fn parse_name_status_handles_rename() {
        let out = format!("{h}\nR100\told/name.md\tnew/name.md\n", h = header("abc"),);
        let (_, changed) = parse_name_status(&out, SEP);
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].status, "R");
        assert_eq!(changed[0].path, "new/name.md");
        assert_eq!(changed[0].old_path.as_deref(), Some("old/name.md"));
    }

    #[test]
    fn parse_name_status_handles_copy() {
        let out = format!("{h}\nC75\tsrc.md\tcopy.md\n", h = header("abc"));
        let (_, changed) = parse_name_status(&out, SEP);
        assert_eq!(changed[0].status, "C");
        assert_eq!(changed[0].path, "copy.md");
        assert_eq!(changed[0].old_path.as_deref(), Some("src.md"));
    }

    #[test]
    fn parse_name_status_empty_output() {
        let (commits, changed) = parse_name_status("", SEP);
        assert_eq!(commits, 0);
        assert!(changed.is_empty());
    }

    #[test]
    fn parse_name_status_ignores_blank_and_garbage_lines() {
        let out = format!("{h}\n\nA\tnotes/a.md\n   \nZ\tweird\n", h = header("abc"),);
        let (commits, changed) = parse_name_status(&out, SEP);
        assert_eq!(commits, 1);
        // `Z` status is not a recognized kind → skipped.
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].path, "notes/a.md");
    }

    #[test]
    fn parse_name_status_strips_cr() {
        let out = format!("{h}\r\nA\tnotes/a.md\r\n", h = header("abc"));
        let (commits, changed) = parse_name_status(&out, SEP);
        assert_eq!(commits, 1);
        assert_eq!(changed[0].path, "notes/a.md");
    }

    #[test]
    fn non_git_dir_degrades_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(tmp.path()));
        let report = diff_since(tmp.path(), "1d").unwrap();
        assert!(!report.is_git);
        assert_eq!(report.commits, 0);
        assert!(report.changed.is_empty());
        assert_eq!(report.since, "1 days ago");
    }

    #[test]
    fn report_serializes_to_expected_shape() {
        let report = SnapshotReport {
            since: "1 days ago".into(),
            is_git: true,
            commits: 1,
            changed: vec![ChangeEntry {
                status: "A".into(),
                path: "a.md".into(),
                old_path: None,
            }],
        };
        let v = serde_json::to_value(&report).unwrap();
        assert_eq!(v["is_git"], serde_json::json!(true));
        assert_eq!(v["changed"][0]["status"], serde_json::json!("A"));
        // old_path omitted when None.
        assert!(v["changed"][0].get("old_path").is_none());
    }
}
