//! Integration coverage for the `--json` error envelope on verbs beyond
//! `capture`. A vault-resolution failure under `--json` must print the
//! structured `{ok:false,error}` envelope on stdout and exit non-zero, never a
//! raw `anyhow` line on stderr that would break a JSON consumer.
//!
//! Each run is hermetic: no `--vault`, no env, and `HOME`/`XDG_CONFIG_HOME`
//! point at an empty temp dir so neither the registry nor `last_vault` resolves.

use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_omninote-cli")
}

/// Run the CLI with a scrubbed config environment (nothing resolvable) and the
/// given args. No `--vault` is supplied, so vault resolution returns "no vault".
fn run_no_vault(empty_config_home: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .env("HOME", empty_config_home)
        .env("XDG_CONFIG_HOME", empty_config_home)
        .env_remove("OMNINOTE_VAULT")
        .output()
        .expect("failed to spawn omninote-cli")
}

/// Assert the process failed and printed a `{ok:false,error:"…no vault…"}`
/// envelope on stdout (not stderr).
fn assert_no_vault_envelope(out: &Output) {
    assert!(
        !out.status.success(),
        "expected non-zero exit; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let env: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("stdout must be a JSON envelope ({e}): {:?}", out.stdout));
    assert_eq!(
        env["ok"],
        Value::Bool(false),
        "envelope must signal failure"
    );
    let msg = env["error"].as_str().expect("error message present");
    assert!(msg.contains("no vault"), "got: {msg}");
    assert!(env.get("data").is_none(), "error envelope carries no data");
}

#[test]
fn note_search_json_error_on_no_vault() {
    let cfg = tempfile::tempdir().unwrap();
    let out = run_no_vault(cfg.path(), &["note", "search", "qualquer", "--json"]);
    assert_no_vault_envelope(&out);
}

#[test]
fn vault_info_json_error_on_no_vault() {
    let cfg = tempfile::tempdir().unwrap();
    let out = run_no_vault(cfg.path(), &["vault", "info", "--json"]);
    assert_no_vault_envelope(&out);
}

#[test]
fn link_unresolved_json_error_on_no_vault() {
    let cfg = tempfile::tempdir().unwrap();
    let out = run_no_vault(cfg.path(), &["link", "unresolved", "--json"]);
    assert_no_vault_envelope(&out);
}

#[test]
fn daily_json_error_on_no_vault() {
    let cfg = tempfile::tempdir().unwrap();
    let out = run_no_vault(cfg.path(), &["daily", "--json"]);
    assert_no_vault_envelope(&out);
}

#[test]
fn note_search_no_vault_without_json_uses_plain_stderr() {
    // The non-JSON path must still print a plain message to stderr and leave
    // stdout empty — the envelope is strictly a `--json` concern.
    let cfg = tempfile::tempdir().unwrap();
    let out = run_no_vault(cfg.path(), &["note", "search", "qualquer"]);
    assert!(!out.status.success());
    assert!(
        out.stdout.is_empty(),
        "non-JSON failure must not write an envelope to stdout"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no vault"), "stderr: {stderr}");
}
