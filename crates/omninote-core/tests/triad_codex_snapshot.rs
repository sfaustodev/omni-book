use omninote_core::snapshot::*;
use std::fs;
use std::path::Path;
use std::process::Command;

const SEP: char = '\u{1f}';

fn header(hash: &str) -> String {
    format!("{SEP}{hash}{SEP}2026-05-30T12:00:00+00:00{SEP}snapshot")
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("run git {args:?}: {err}"));

    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn parse_since_passes_through_hostile_or_non_shorthand_tokens() {
    assert_eq!(parse_since("+7d").as_deref(), Some("7 days ago"));

    for token in [
        "99999999999999999999d",
        "-7d",
        "１２d",
        "1d\n--all",
        "1d\0--all",
        "10dd",
        "d",
    ] {
        assert_eq!(parse_since(token).as_deref(), Some(token));
    }
}

#[test]
fn parse_name_status_skips_malformed_status_rows() {
    let out = format!(
        "{h}\nA\nR100\told.md\nZ\tweird.md\nX\tother.md\n",
        h = header("abc")
    );

    let (commits, changed) = parse_name_status(&out, SEP);

    assert_eq!(commits, 1);
    assert!(changed.is_empty());
}

#[test]
fn parse_name_status_accepts_header_sentinel_inside_path_column() {
    let path = format!("{SEP}starts-with-sentinel.md");
    let out = format!("{h}\nA\t{path}\n", h = header("abc"));

    let (commits, changed) = parse_name_status(&out, SEP);

    assert_eq!(commits, 1);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].status, "A");
    assert_eq!(changed[0].path, path);
}

#[test]
fn parse_name_status_handles_crlf_and_dedups_three_commits_to_newest() {
    let out = format!(
        "{h1}\r\nM\tnotes/same.md\r\n{h2}\r\nD\tnotes/same.md\r\n{h3}\r\nA\tnotes/same.md\r\n",
        h1 = header("newest"),
        h2 = header("middle"),
        h3 = header("oldest"),
    );

    let (commits, changed) = parse_name_status(&out, SEP);

    assert_eq!(commits, 3);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].status, "M");
    assert_eq!(changed[0].path, "notes/same.md");
}

#[test]
fn diff_since_reports_real_git_repo_changes_and_non_git_dir_gracefully() {
    let git_repo = tempfile::tempdir().unwrap();
    run_git(git_repo.path(), &["init"]);
    run_git(
        git_repo.path(),
        &["config", "user.email", "codex@example.test"],
    );
    run_git(git_repo.path(), &["config", "user.name", "Codex Snapshot"]);

    fs::write(git_repo.path().join("same.md"), "v1\n").unwrap();
    run_git(git_repo.path(), &["add", "same.md"]);
    run_git(git_repo.path(), &["commit", "-m", "first"]);

    fs::write(git_repo.path().join("same.md"), "v2\n").unwrap();
    fs::write(git_repo.path().join("second.md"), "new\n").unwrap();
    run_git(git_repo.path(), &["add", "same.md", "second.md"]);
    run_git(git_repo.path(), &["commit", "-m", "second"]);

    assert!(is_git_repo(git_repo.path()));
    let report = diff_since(git_repo.path(), "1y").unwrap();
    assert!(report.is_git);
    assert_eq!(report.since, "1 years ago");
    assert_eq!(report.commits, 2);
    assert_eq!(report.changed.len(), 2);
    assert_eq!(report.changed[0].status, "M");
    assert_eq!(report.changed[0].path, "same.md");
    assert_eq!(report.changed[1].status, "A");
    assert_eq!(report.changed[1].path, "second.md");

    let non_git = tempfile::tempdir().unwrap();
    assert!(!is_git_repo(non_git.path()));
    let non_git_report = diff_since(non_git.path(), "1d").unwrap();
    assert!(!non_git_report.is_git);
    assert_eq!(non_git_report.commits, 0);
    assert!(non_git_report.changed.is_empty());
}
