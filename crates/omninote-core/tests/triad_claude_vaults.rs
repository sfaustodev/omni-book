//! Adversarial coverage for the multi-vault registry (triad gate).
//! Covers agy's intended target (agy was unavailable — see triad report):
//! name/path edge cases, TOML round-trip integrity, and I/O failure modes.

use omninote_core::vaults::{load, save, VaultEntry, VaultRegistry};

#[test]
fn duplicate_name_updates_path_keeps_single_entry() {
    let mut reg = VaultRegistry::default();
    assert!(reg.add("work", "/a").unwrap());
    assert!(!reg.add("work", "/b").unwrap());
    assert_eq!(reg.vaults.len(), 1);
    assert_eq!(reg.get("work").unwrap().path.to_str(), Some("/b"));
}

#[test]
fn unicode_and_emoji_names_are_usable() {
    let mut reg = VaultRegistry::default();
    reg.add("trabalho-工作-📓", "/n/uni").unwrap();
    assert!(reg.get("trabalho-工作-📓").is_some());
    reg.switch("trabalho-工作-📓").unwrap();
    assert_eq!(reg.active_entry().unwrap().path.to_str(), Some("/n/uni"));
}

#[test]
fn whitespace_padded_name_is_stored_verbatim_not_trimmed() {
    // Documents actual behavior: add() rejects only all-whitespace names; a
    // padded name is kept raw, so lookups must use the exact string. (Trimming
    // would be a UX nicety, not a correctness fix — single-user, self-entered.)
    let mut reg = VaultRegistry::default();
    reg.add("  work  ", "/p").unwrap();
    assert!(reg.get("  work  ").is_some());
    assert!(reg.get("work").is_none());
}

#[test]
fn paths_with_spaces_and_unicode_survive() {
    let mut reg = VaultRegistry::default();
    reg.add("a", "/Users/me/My Notes/工作 vault").unwrap();
    assert_eq!(
        reg.get("a").unwrap().path.to_str(),
        Some("/Users/me/My Notes/工作 vault")
    );
}

#[test]
fn active_dangles_to_none_after_removing_active() {
    let mut reg = VaultRegistry::default();
    reg.add("a", "/a").unwrap();
    reg.add("b", "/b").unwrap();
    reg.switch("b").unwrap();
    assert!(reg.remove("b"));
    assert!(reg.active.is_none());
    assert!(reg.active_entry().is_none());
}

#[test]
fn switch_to_empty_string_errors() {
    let mut reg = VaultRegistry::default();
    reg.add("a", "/a").unwrap();
    assert!(reg.switch("").is_err());
    assert_eq!(reg.active.as_deref(), Some("a"));
}

#[test]
fn name_with_toml_special_chars_round_trips_losslessly() {
    // A name carrying quotes/brackets/backslash/newline must be escaped by the
    // serializer, not injected into the TOML structure — round-trip proves it.
    let hostile = "a\"b[c]\\d\ne\tf";
    let mut reg = VaultRegistry::default();
    reg.add(hostile, "/p").unwrap();
    let text = reg.to_toml().unwrap();
    let back = VaultRegistry::from_toml(&text).unwrap();
    assert_eq!(back, reg);
    assert!(back.get(hostile).is_some());
}

#[test]
fn load_on_a_directory_errors_gracefully() {
    // Reading a directory as a config file must surface Err, never panic.
    let dir = tempfile::tempdir().unwrap();
    assert!(load(dir.path()).is_err());
}

#[test]
fn save_then_load_preserves_entry_order() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("vaults.toml");
    let mut reg = VaultRegistry::default();
    for (n, p) in [("z", "/z"), ("a", "/a"), ("m", "/m")] {
        reg.add(n, p).unwrap();
    }
    save(&path, &reg).unwrap();
    let back = load(&path).unwrap();
    let names: Vec<&str> = back.vaults.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, vec!["z", "a", "m"]);
}

#[test]
fn registry_with_many_entries_round_trips() {
    let mut reg = VaultRegistry::default();
    for i in 0..50 {
        reg.add(format!("vault-{i}"), format!("/n/{i}")).unwrap();
    }
    reg.switch("vault-37").unwrap();
    let back = VaultRegistry::from_toml(&reg.to_toml().unwrap()).unwrap();
    assert_eq!(back.vaults.len(), 50);
    assert_eq!(back.active.as_deref(), Some("vault-37"));
    assert_eq!(back, reg);
}

#[test]
fn parsed_entry_equals_constructed_entry() {
    let reg = VaultRegistry::from_toml("[[vault]]\nname = \"x\"\npath = \"/x\"\n").unwrap();
    assert_eq!(
        reg.vaults[0],
        VaultEntry {
            name: "x".into(),
            path: "/x".into()
        }
    );
}
