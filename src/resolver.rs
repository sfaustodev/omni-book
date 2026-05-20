//! Wikilink target resolution against a vault index.
//!
//! Resolution order (CAD-20):
//! 1. Exact filename match (`[[Note]]` → `Note.md`)
//! 2. Path match (`[[folder/Note]]` → `folder/Note.md`, accepts trailing `.md` too)
//! 3. Frontmatter `aliases: [...]` lookup
//! 4. Case-insensitive filename fallback
//! 5. None — link is unresolved
//!
//! The [`VaultIndex`] is built from `Vec<Note>` and intended to be rebuilt by
//! [`crate::vault::Vault`] after every `reload_notes()` call.

use crate::types::Note;
use std::collections::HashMap;
use std::path::PathBuf;

/// Lookup tables built from a `Vec<Note>` snapshot. Cheap to clone? No — keep
/// owned by `Vault` and re-borrow.
#[derive(Default, Debug)]
pub struct VaultIndex {
    /// Exact filename (without `.md`) → note `rel_path`. Case-sensitive.
    by_filename: HashMap<String, PathBuf>,
    /// Lowercase filename (without `.md`) → note `rel_path`. Fallback bucket.
    by_filename_lower: HashMap<String, PathBuf>,
    /// `folder/Note` (without `.md`) → note `rel_path`. Case-sensitive.
    by_path: HashMap<String, PathBuf>,
    /// Lowercase `folder/note` → note `rel_path`. Fallback.
    by_path_lower: HashMap<String, PathBuf>,
    /// Lowercase alias (from frontmatter `aliases: [...]`) → note `rel_path`.
    by_alias: HashMap<String, PathBuf>,
}

impl VaultIndex {
    /// Build an index from a notes snapshot. Last-writer-wins on collisions
    /// (deterministic for stable input ordering).
    pub fn build(notes: &[Note]) -> Self {
        let mut idx = VaultIndex::default();
        for note in notes {
            let rel = note.rel_path.clone();

            // Filename without extension
            if let Some(stem) = note.rel_path.file_stem().and_then(|s| s.to_str()) {
                idx.by_filename.insert(stem.to_string(), rel.clone());
                idx.by_filename_lower
                    .insert(stem.to_ascii_lowercase(), rel.clone());
            }

            // Path form: parent/stem (forward-slash normalized, no .md)
            let path_str = rel.with_extension("").to_string_lossy().replace('\\', "/");
            idx.by_path.insert(path_str.clone(), rel.clone());
            idx.by_path_lower
                .insert(path_str.to_ascii_lowercase(), rel.clone());

            // Aliases (CAD-20: frontmatter aliases)
            for alias in &note.frontmatter.aliases {
                let trimmed = alias.trim();
                if !trimmed.is_empty() {
                    idx.by_alias
                        .insert(trimmed.to_ascii_lowercase(), rel.clone());
                }
            }
        }
        idx
    }

    /// Resolve a wikilink target to a vault-relative path, following the rules
    /// in the module docstring. Returns `None` if no rule matches.
    pub fn resolve(&self, target: &str) -> Option<&PathBuf> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return None;
        }
        // Strip optional trailing `.md` so `[[Note]]` and `[[Note.md]]` behave alike.
        let normalized = trimmed.strip_suffix(".md").unwrap_or(trimmed);

        // 1. Exact filename
        if let Some(p) = self.by_filename.get(normalized) {
            return Some(p);
        }
        // 2. Path match (forward-slash normalized)
        let path_form = normalized.replace('\\', "/");
        if let Some(p) = self.by_path.get(&path_form) {
            return Some(p);
        }
        // 3. Frontmatter aliases (case-insensitive)
        if let Some(p) = self.by_alias.get(&normalized.to_ascii_lowercase()) {
            return Some(p);
        }
        // 4. Case-insensitive filename
        if let Some(p) = self.by_filename_lower.get(&normalized.to_ascii_lowercase()) {
            return Some(p);
        }
        // 4b. Case-insensitive path
        if let Some(p) = self.by_path_lower.get(&path_form.to_ascii_lowercase()) {
            return Some(p);
        }
        None
    }

    /// True if the index contains a note that matches `target` under any rule.
    /// Convenience wrapper for `resolve(...).is_some()`.
    /// Consumed by CLI/MCP verbs in CAD-21 Phase 2.
    #[allow(dead_code)]
    pub fn contains(&self, target: &str) -> bool {
        self.resolve(target).is_some()
    }

    /// Returns the list of (target, source_rel_path) for every wikilink in
    /// `notes` that this index cannot resolve. Used by the CLI/MCP
    /// `link unresolved` verb (CAD-21 Phase 2).
    #[allow(dead_code)]
    pub fn unresolved_links(&self, notes: &[Note]) -> Vec<UnresolvedLink> {
        use crate::wikilinks::extract;
        let mut out = Vec::new();
        for note in notes {
            for w in extract(&note.content) {
                if !w.is_note() {
                    continue;
                }
                let target = w.target_str().to_string();
                if self.resolve(&target).is_none() {
                    out.push(UnresolvedLink {
                        target,
                        source: note.rel_path.clone(),
                    });
                }
            }
        }
        out
    }

    /// Index statistics for diagnostics / `vault info` CLI verb (CAD-21 Phase 2).
    #[allow(dead_code)]
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            files: self.by_filename.len(),
            paths: self.by_path.len(),
            aliases: self.by_alias.len(),
        }
    }
}

/// Diagnostic counts. Returned by [`VaultIndex::stats`]. CAD-21 Phase 2.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexStats {
    pub files: usize,
    pub paths: usize,
    pub aliases: usize,
}

/// A wikilink that doesn't resolve to any note in the vault.
/// CAD-21 Phase 2 surfaces this via `omninote link unresolved`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedLink {
    pub target: String,
    pub source: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Frontmatter, Note, NoteType};
    use std::path::{Path, PathBuf};

    fn mk_note(rel_path: &str, content: &str, aliases: Vec<&str>) -> Note {
        let rel = PathBuf::from(rel_path);
        let title = rel
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        Note {
            path: Path::new("/tmp/vault").join(&rel),
            rel_path: rel,
            frontmatter: Frontmatter {
                id: format!("n_{title}"),
                note_type: NoteType::Resumo,
                tags: vec![],
                source: String::new(),
                source_link: String::new(),
                linked_note: None,
                attachments: vec![],
                created: String::new(),
                aliases: aliases.into_iter().map(|s| s.to_string()).collect(),
            },
            title,
            content: content.to_string(),
        }
    }

    #[test]
    fn resolves_exact_filename() {
        let notes = vec![mk_note("Foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert_eq!(idx.resolve("Foo"), Some(&PathBuf::from("Foo.md")));
    }

    #[test]
    fn resolves_with_trailing_md() {
        let notes = vec![mk_note("Foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert_eq!(idx.resolve("Foo.md"), Some(&PathBuf::from("Foo.md")));
    }

    #[test]
    fn resolves_path_match() {
        let notes = vec![mk_note("folder/sub/Note.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert_eq!(
            idx.resolve("folder/sub/Note"),
            Some(&PathBuf::from("folder/sub/Note.md"))
        );
    }

    #[test]
    fn resolves_path_with_backslashes() {
        // Windows-style separator in the target string is normalized.
        let notes = vec![mk_note("folder/Note.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert_eq!(
            idx.resolve("folder\\Note"),
            Some(&PathBuf::from("folder/Note.md"))
        );
    }

    #[test]
    fn resolves_frontmatter_alias_case_insensitive() {
        let notes = vec![mk_note("SPEC.md", "", vec!["NdA", "Spec da Namorada"])];
        let idx = VaultIndex::build(&notes);
        assert_eq!(idx.resolve("nda"), Some(&PathBuf::from("SPEC.md")));
        assert_eq!(
            idx.resolve("spec da namorada"),
            Some(&PathBuf::from("SPEC.md"))
        );
    }

    #[test]
    fn resolves_case_insensitive_filename_fallback() {
        let notes = vec![mk_note("MyNote.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        // Exact case missing — fallback to insensitive match.
        assert_eq!(idx.resolve("mynote"), Some(&PathBuf::from("MyNote.md")));
        assert_eq!(idx.resolve("MYNOTE"), Some(&PathBuf::from("MyNote.md")));
    }

    #[test]
    fn returns_none_for_unknown() {
        let notes = vec![mk_note("Foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert!(idx.resolve("Bar").is_none());
    }

    #[test]
    fn returns_none_for_empty_target() {
        let notes = vec![mk_note("Foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert!(idx.resolve("").is_none());
        assert!(idx.resolve("   ").is_none());
    }

    #[test]
    fn priority_exact_over_alias() {
        // If "Foo" is both a filename AND an alias of another note, exact filename wins.
        let notes = vec![
            mk_note("Foo.md", "", vec![]),
            mk_note("Bar.md", "", vec!["Foo"]),
        ];
        let idx = VaultIndex::build(&notes);
        assert_eq!(idx.resolve("Foo"), Some(&PathBuf::from("Foo.md")));
    }

    #[test]
    fn priority_exact_over_case_insensitive() {
        // "Foo" is exact, "foo" is also a different file → exact case wins.
        let notes = vec![mk_note("Foo.md", "", vec![]), mk_note("foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        // exact match for "foo" returns foo.md (not the lowercased Foo)
        assert_eq!(idx.resolve("foo"), Some(&PathBuf::from("foo.md")));
        assert_eq!(idx.resolve("Foo"), Some(&PathBuf::from("Foo.md")));
    }

    #[test]
    fn contains_mirrors_resolve() {
        let notes = vec![mk_note("Foo.md", "", vec![])];
        let idx = VaultIndex::build(&notes);
        assert!(idx.contains("Foo"));
        assert!(!idx.contains("Bar"));
    }

    #[test]
    fn unresolved_links_reports_missing_targets() {
        let notes = vec![
            mk_note("Foo.md", "links to [[Bar]] and [[Baz]]", vec![]),
            mk_note("Bar.md", "", vec![]),
        ];
        let idx = VaultIndex::build(&notes);
        let unresolved = idx.unresolved_links(&notes);
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].target, "Baz");
        assert_eq!(unresolved[0].source, PathBuf::from("Foo.md"));
    }

    #[test]
    fn unresolved_ignores_embed_assets() {
        // Images and files should NOT be reported as unresolved notes.
        let notes = vec![mk_note(
            "Foo.md",
            "![[img.png]] ![[manual.pdf]] [[Bar]]",
            vec![],
        )];
        let idx = VaultIndex::build(&notes);
        let unresolved = idx.unresolved_links(&notes);
        // Only [[Bar]] should be unresolved (img.png and manual.pdf are asset embeds, not note refs).
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].target, "Bar");
    }

    #[test]
    fn unresolved_includes_note_embeds() {
        // ![[OtherNote]] is a NoteEmbed and must be tracked.
        let notes = vec![mk_note("Foo.md", "![[NoSuchNote]]", vec![])];
        let idx = VaultIndex::build(&notes);
        let unresolved = idx.unresolved_links(&notes);
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].target, "NoSuchNote");
    }

    #[test]
    fn stats_counts_correctly() {
        let notes = vec![
            mk_note("a/Foo.md", "", vec!["A1", "A2"]),
            mk_note("b/Bar.md", "", vec!["B1"]),
        ];
        let idx = VaultIndex::build(&notes);
        let stats = idx.stats();
        assert_eq!(stats.files, 2);
        assert_eq!(stats.paths, 2);
        assert_eq!(stats.aliases, 3);
    }

    #[test]
    fn empty_aliases_skipped() {
        // Whitespace-only aliases must not pollute the index.
        let notes = vec![mk_note("Foo.md", "", vec!["", "  ", "Real"])];
        let idx = VaultIndex::build(&notes);
        let stats = idx.stats();
        assert_eq!(stats.aliases, 1);
        assert!(idx.resolve("Real").is_some());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn resolve_never_panics_on_arbitrary_target(target in proptest::prelude::any::<String>()) {
            let notes = vec![mk_note("Foo.md", "", vec!["Bar"])];
            let idx = VaultIndex::build(&notes);
            let _ = idx.resolve(&target);
        }
    }
}
