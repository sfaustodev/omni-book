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

/// Run the CLI against an explicit valid `--vault` (so resolution + open
/// succeed and an OPERATIONAL error — bad date, missing template, bad arg — is
/// what reaches the `--json` envelope chokepoint). `HOME` is still scrubbed so
/// the registry can't interfere.
fn run_with_vault(home: &Path, vault: &Path, args: &[&str]) -> Output {
    let mut full = vec!["--vault", vault.to_str().unwrap()];
    full.extend_from_slice(args);
    Command::new(bin())
        .args(&full)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env_remove("OMNINOTE_VAULT")
        .output()
        .expect("failed to spawn omninote-cli")
}

/// Assert the process failed and printed a `{ok:false,error}` envelope on
/// stdout whose message contains `needle`. The operational-error analogue of
/// `assert_no_vault_envelope`.
fn assert_error_envelope(out: &Output, needle: &str) {
    assert!(
        !out.status.success(),
        "expected non-zero exit; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let env: Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout must be a JSON envelope ({e}): {:?} / stderr: {}",
            out.stdout,
            String::from_utf8_lossy(&out.stderr)
        )
    });
    assert_eq!(
        env["ok"],
        Value::Bool(false),
        "envelope must signal failure"
    );
    let msg = env["error"].as_str().expect("error message present");
    assert!(msg.contains(needle), "got: {msg}");
    assert!(env.get("data").is_none(), "error envelope carries no data");
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

// ── Operational errors (vault resolves; the verb itself fails) must ALSO be
//    enveloped under --json via the top-level chokepoint, not escape as a raw
//    anyhow line. One per verb family the round-4 review flagged. ──

#[test]
fn daily_bad_date_json_error_is_enveloped() {
    let home = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    let out = run_with_vault(
        home.path(),
        vault.path(),
        &["daily", "--date", "nope", "--json"],
    );
    assert_error_envelope(&out, "invalid date");
}

#[test]
fn template_apply_missing_json_error_is_enveloped() {
    let home = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    let out = run_with_vault(
        home.path(),
        vault.path(),
        &["template", "apply", "definitely-missing", "--json"],
    );
    assert_error_envelope(&out, "template");
}

#[test]
fn discipline_show_bad_arg_json_error_is_enveloped() {
    let home = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    let out = run_with_vault(
        home.path(),
        vault.path(),
        &["discipline", "show", "bogusfile", "--json"],
    );
    assert_error_envelope(&out, "unknown discipline file");
}

#[test]
fn vault_list_corrupt_registry_json_error_is_enveloped() {
    // `vault list` reads the registry directly (no `--vault`). A corrupt
    // `vaults.toml` must surface as a `{ok:false,error}` envelope, not a raw
    // anyhow line. On this platform the registry lives under
    // `$HOME/Library/Application Support/omninote/vaults.toml` (dirs::config_dir
    // ignores XDG_CONFIG_HOME on macOS — verified at runtime), but the CLI also
    // honors `$HOME/.config` on Linux; write to both so the test is portable.
    let home = tempfile::tempdir().unwrap();
    for sub in [
        home.path().join("Library/Application Support/omninote"),
        home.path().join(".config/omninote"),
    ] {
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("vaults.toml"), "this is = not [ valid toml").unwrap();
    }
    let out = run_no_vault(home.path(), &["vault", "list", "--json"]);
    assert_error_envelope(&out, "vaults.toml");
}
