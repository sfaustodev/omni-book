//! Integration coverage for the `omninote capture` verb (CAD-24 Layer A).
//!
//! These drive the built binary as a subprocess (via the `CARGO_BIN_EXE_*`
//! path Cargo injects for integration tests), so they exercise the real clap
//! wiring + JSON envelope, not just the core seam. Each run is hermetic: the
//! vault is an explicit `--vault <tmp>` and the config-dir sources are
//! neutralized by pointing `HOME`/`XDG_CONFIG_HOME` at empty temp dirs.

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

/// Path to the freshly-built CLI binary under test.
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_omninote-cli")
}

/// Run `omninote capture` with the given args, vault, and a scrubbed config
/// environment so registry/last_vault never leak into resolution.
fn run_capture(vault: Option<&Path>, empty_config_home: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(bin());
    cmd.arg("capture")
        .args(args)
        // Neutralize every config-dir source on both macOS and Linux.
        .env("HOME", empty_config_home)
        .env("XDG_CONFIG_HOME", empty_config_home)
        .env_remove("OMNINOTE_VAULT");
    if let Some(v) = vault {
        cmd.args(["--vault", &v.to_string_lossy()]);
    }
    cmd.output().expect("failed to spawn omninote-cli")
}

#[test]
fn capture_json_envelope_shape_ok_with_total_lines_meta() {
    let vault = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    let out = run_capture(Some(vault.path()), cfg.path(), &["comprar leite", "--json"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let env: Value = serde_json::from_slice(&out.stdout).expect("stdout is a JSON envelope");
    assert_eq!(env["ok"], Value::Bool(true));
    // data.path points at the vault's Inbox.md; data.line_appended is the bullet.
    let path = env["data"]["path"].as_str().unwrap();
    assert!(path.ends_with("Inbox.md"));
    let bullet = env["data"]["line_appended"].as_str().unwrap();
    assert!(bullet.starts_with("- "));
    assert!(bullet.ends_with("· comprar leite"));
    // meta.total_lines is the running bullet count (1 after a single capture).
    assert_eq!(env["meta"]["total_lines"], Value::from(1));

    // The line really landed on disk.
    let inbox = std::fs::read_to_string(vault.path().join("Inbox.md")).unwrap();
    assert!(inbox.contains("comprar leite"));
}

#[test]
fn capture_json_total_lines_increments_across_calls() {
    let vault = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    run_capture(Some(vault.path()), cfg.path(), &["um", "--json"]);
    let out = run_capture(Some(vault.path()), cfg.path(), &["dois", "--json"]);
    let env: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(env["meta"]["total_lines"], Value::from(2));
}

#[test]
fn capture_json_error_on_empty_text() {
    let vault = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    // Whitespace-only text is rejected by the core's trim/empty rule.
    let out = run_capture(Some(vault.path()), cfg.path(), &["   ", "--json"]);
    assert!(!out.status.success());

    let env: Value = serde_json::from_slice(&out.stdout).expect("error envelope is JSON");
    assert_eq!(env["ok"], Value::Bool(false));
    assert_eq!(env["error"], Value::from("linha vazia"));
    assert!(env.get("data").is_none(), "error envelope carries no data");

    // A rejected capture must not create the inbox.
    assert!(!vault.path().join("Inbox.md").exists());
}

#[test]
fn capture_errors_when_no_vault_resolvable() {
    let cfg = tempfile::tempdir().unwrap();
    // No --vault, no env, empty config dir → nothing resolves.
    let out = run_capture(None, cfg.path(), &["orfã", "--json"]);
    assert!(!out.status.success());

    let env: Value = serde_json::from_slice(&out.stdout).expect("error envelope is JSON");
    assert_eq!(env["ok"], Value::Bool(false));
    let msg = env["error"].as_str().unwrap();
    assert!(msg.contains("no vault"), "got: {msg}");
}

#[test]
fn capture_human_mode_prints_confirmation_and_bullet() {
    let vault = tempfile::tempdir().unwrap();
    let cfg = tempfile::tempdir().unwrap();
    let out = run_capture(Some(vault.path()), cfg.path(), &["uma ideia"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Confirmation line then the rendered bullet.
    assert!(stdout.contains("✓ Inbox.md"));
    assert!(stdout.contains("· uma ideia"));
}
