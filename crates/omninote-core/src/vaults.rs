//! Multi-vault registry persisted at `~/.config/omninote/vaults.toml`.
//!
//! This file tracks vault *paths* and which one is active — it is NOT a cache
//! of note content (the filesystem stays the single source of truth). Each
//! entry is a `name` + absolute `path`; `active` names the currently selected
//! entry.
//!
//! On-disk shape:
//! ```toml
//! active = "work"
//!
//! [[vault]]
//! name = "work"
//! path = "/Users/me/notes/work"
//!
//! [[vault]]
//! name = "personal"
//! path = "/Users/me/notes/personal"
//! ```
//!
//! All mutation helpers operate on the in-memory [`VaultRegistry`]; [`load`]
//! and [`save`] handle the TOML I/O so the pure logic stays unit-testable
//! without touching the real config dir.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One registered vault: a display name plus its root path on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub path: PathBuf,
}

/// The full registry. `vault` is the list of entries; `active` is the name of
/// the selected one (if any).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultRegistry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    #[serde(default, rename = "vault")]
    pub vaults: Vec<VaultEntry>,
}

impl VaultRegistry {
    /// Parse a registry from TOML text. Empty / whitespace input yields an
    /// empty registry rather than an error (a missing config is not a fault).
    pub fn from_toml(text: &str) -> Result<Self, String> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        toml::from_str(text).map_err(|e| format!("parse vaults.toml: {e}"))
    }

    /// Serialize to pretty TOML.
    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("serialize vaults.toml: {e}"))
    }

    /// Look up an entry by name.
    pub fn get(&self, name: &str) -> Option<&VaultEntry> {
        self.vaults.iter().find(|v| v.name == name)
    }

    /// The active entry, if `active` is set and still present in the list.
    pub fn active_entry(&self) -> Option<&VaultEntry> {
        self.active.as_deref().and_then(|n| self.get(n))
    }

    /// Add (or update) a vault. If `name` already exists its path is replaced.
    /// When the registry was empty, the new entry also becomes active. Returns
    /// `true` if a brand-new entry was inserted, `false` if an existing one was
    /// updated in place.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        path: impl Into<PathBuf>,
    ) -> Result<bool, String> {
        let name = name.into();
        let path = path.into();
        if name.trim().is_empty() {
            return Err("vault name is empty".into());
        }
        let was_empty = self.vaults.is_empty();
        let inserted = if let Some(existing) = self.vaults.iter_mut().find(|v| v.name == name) {
            existing.path = path;
            false
        } else {
            self.vaults.push(VaultEntry {
                name: name.clone(),
                path,
            });
            true
        };
        if was_empty {
            self.active = Some(name);
        }
        Ok(inserted)
    }

    /// Set the active vault by name. Errors if no such entry exists.
    pub fn switch(&mut self, name: &str) -> Result<(), String> {
        if self.get(name).is_none() {
            return Err(format!("no vault named '{name}' — run `vault add` first"));
        }
        self.active = Some(name.to_string());
        Ok(())
    }

    /// Remove a vault by name. Returns `true` if something was removed. Clears
    /// `active` when the removed entry was the active one.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.vaults.len();
        self.vaults.retain(|v| v.name != name);
        let removed = self.vaults.len() != before;
        if removed && self.active.as_deref() == Some(name) {
            self.active = None;
        }
        removed
    }
}

/// Path to `vaults.toml` under the user config dir
/// (`~/.config/omninote/vaults.toml` on Linux/macOS). `None` if the platform
/// has no config dir.
pub fn registry_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("omninote").join("vaults.toml"))
}

/// Load the registry from `path`. A missing file yields an empty registry.
pub fn load(path: &Path) -> Result<VaultRegistry, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => VaultRegistry::from_toml(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(VaultRegistry::default()),
        Err(e) => Err(format!("read {}: {e}", path.display())),
    }
}

/// Persist the registry to `path`, creating parent dirs as needed.
pub fn save(path: &Path, registry: &VaultRegistry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let text = registry.to_toml()?;
    std::fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_parses_to_default() {
        assert_eq!(
            VaultRegistry::from_toml("").unwrap(),
            VaultRegistry::default()
        );
        assert_eq!(
            VaultRegistry::from_toml("   \n  ").unwrap(),
            VaultRegistry::default()
        );
    }

    #[test]
    fn parses_full_registry() {
        let text = r#"
active = "work"

[[vault]]
name = "work"
path = "/n/work"

[[vault]]
name = "home"
path = "/n/home"
"#;
        let reg = VaultRegistry::from_toml(text).unwrap();
        assert_eq!(reg.active.as_deref(), Some("work"));
        assert_eq!(reg.vaults.len(), 2);
        assert_eq!(reg.vaults[0].name, "work");
        assert_eq!(reg.vaults[1].path, PathBuf::from("/n/home"));
    }

    #[test]
    fn invalid_toml_errors() {
        assert!(VaultRegistry::from_toml("active = ").is_err());
    }

    #[test]
    fn round_trips_through_toml() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        reg.add("b", "/b").unwrap();
        reg.switch("b").unwrap();
        let text = reg.to_toml().unwrap();
        let back = VaultRegistry::from_toml(&text).unwrap();
        assert_eq!(back, reg);
    }

    #[test]
    fn default_serializes_without_active() {
        let reg = VaultRegistry::default();
        let text = reg.to_toml().unwrap();
        assert!(!text.contains("active"));
    }

    #[test]
    fn add_returns_true_for_new_false_for_update() {
        let mut reg = VaultRegistry::default();
        assert!(reg.add("a", "/a").unwrap());
        assert!(!reg.add("a", "/a2").unwrap());
        assert_eq!(reg.vaults.len(), 1);
        assert_eq!(reg.vaults[0].path, PathBuf::from("/a2"));
    }

    #[test]
    fn first_add_becomes_active() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        assert_eq!(reg.active.as_deref(), Some("a"));
        // A second add does not change active.
        reg.add("b", "/b").unwrap();
        assert_eq!(reg.active.as_deref(), Some("a"));
    }

    #[test]
    fn add_rejects_empty_name() {
        let mut reg = VaultRegistry::default();
        assert!(reg.add("", "/a").is_err());
        assert!(reg.add("   ", "/a").is_err());
    }

    #[test]
    fn switch_sets_active() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        reg.add("b", "/b").unwrap();
        reg.switch("b").unwrap();
        assert_eq!(reg.active.as_deref(), Some("b"));
        assert_eq!(reg.active_entry().unwrap().path, PathBuf::from("/b"));
    }

    #[test]
    fn switch_unknown_errors() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        assert!(reg.switch("ghost").is_err());
        assert_eq!(reg.active.as_deref(), Some("a"));
    }

    #[test]
    fn get_and_active_entry() {
        let mut reg = VaultRegistry::default();
        assert!(reg.active_entry().is_none());
        reg.add("a", "/a").unwrap();
        assert_eq!(reg.get("a").unwrap().path, PathBuf::from("/a"));
        assert!(reg.get("nope").is_none());
        assert_eq!(reg.active_entry().unwrap().name, "a");
    }

    #[test]
    fn active_entry_none_when_dangling() {
        // active names an entry that isn't in the list.
        let reg = VaultRegistry {
            active: Some("ghost".into()),
            vaults: vec![VaultEntry {
                name: "a".into(),
                path: "/a".into(),
            }],
        };
        assert!(reg.active_entry().is_none());
    }

    #[test]
    fn remove_clears_active_when_removing_active() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        reg.add("b", "/b").unwrap();
        reg.switch("b").unwrap();
        assert!(reg.remove("b"));
        assert!(reg.active.is_none());
        assert_eq!(reg.vaults.len(), 1);
    }

    #[test]
    fn remove_keeps_active_when_removing_other() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        reg.add("b", "/b").unwrap();
        // active is "a" (first add).
        assert!(reg.remove("b"));
        assert_eq!(reg.active.as_deref(), Some("a"));
    }

    #[test]
    fn remove_missing_returns_false() {
        let mut reg = VaultRegistry::default();
        reg.add("a", "/a").unwrap();
        assert!(!reg.remove("ghost"));
        assert_eq!(reg.vaults.len(), 1);
    }

    #[test]
    fn load_missing_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("vaults.toml");
        let reg = load(&path).unwrap();
        assert_eq!(reg, VaultRegistry::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested").join("vaults.toml");
        let mut reg = VaultRegistry::default();
        reg.add("work", "/n/work").unwrap();
        reg.add("home", "/n/home").unwrap();
        reg.switch("home").unwrap();
        save(&path, &reg).unwrap();
        assert!(path.exists());
        let back = load(&path).unwrap();
        assert_eq!(back, reg);
    }

    #[test]
    fn load_corrupt_file_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("vaults.toml");
        std::fs::write(&path, "this = is = broken").unwrap();
        assert!(load(&path).is_err());
    }
}
