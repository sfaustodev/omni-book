//! Full-text search across vault notes. CAD-21 Phase B.
//!
//! Simple substring matching for v1. Future (out of scope phase B):
//! fuzzy matching, stemming, scoring, BM25. Phase 4 (CAD-23) will add
//! semantic search via embeddings.

use crate::types::Note;

/// A single match: which note, which line, the matching snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub rel_path: std::path::PathBuf,
    pub title: String,
    pub line_no: usize,
    pub snippet: String,
}

/// Search options.
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchOpts {
    /// True = case-sensitive. Default false (case-insensitive).
    pub case_sensitive: bool,
    /// Max hits to return. None = unbounded.
    pub limit: Option<usize>,
}

/// Search `notes` for occurrences of `query`. Returns hits ordered by note
/// path, then line number. Empty query returns empty result.
pub fn search(notes: &[Note], query: &str, opts: SearchOpts) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = if opts.case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    };
    let mut hits = Vec::new();
    let limit = opts.limit.unwrap_or(usize::MAX);

    'notes: for note in notes {
        for (line_no, line) in note.content.lines().enumerate() {
            let haystack = if opts.case_sensitive {
                line.to_string()
            } else {
                line.to_ascii_lowercase()
            };
            if haystack.contains(&needle) {
                hits.push(SearchHit {
                    rel_path: note.rel_path.clone(),
                    title: note.title.clone(),
                    line_no: line_no + 1,
                    snippet: line.trim().to_string(),
                });
                if hits.len() >= limit {
                    break 'notes;
                }
            }
        }
    }

    hits
}

/// Search note titles only (faster path for picker/quick-switcher UX).
/// Returns matches ordered by note path. Case-insensitive by default.
pub fn search_titles(notes: &[Note], query: &str, opts: SearchOpts) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let needle = if opts.case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    };
    let limit = opts.limit.unwrap_or(usize::MAX);
    let mut hits = Vec::new();
    for note in notes {
        let title_h = if opts.case_sensitive {
            note.title.clone()
        } else {
            note.title.to_ascii_lowercase()
        };
        if title_h.contains(&needle) {
            hits.push(SearchHit {
                rel_path: note.rel_path.clone(),
                title: note.title.clone(),
                line_no: 0,
                snippet: note.title.clone(),
            });
            if hits.len() >= limit {
                break;
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Frontmatter, Note, NoteType};
    use std::path::PathBuf;

    fn mk_note(rel: &str, content: &str) -> Note {
        Note {
            path: PathBuf::from("/tmp/vault").join(rel),
            rel_path: PathBuf::from(rel),
            frontmatter: Frontmatter {
                id: rel.into(),
                note_type: NoteType::Resumo,
                tags: vec![],
                source: String::new(),
                source_link: String::new(),
                linked_note: None,
                attachments: vec![],
                created: String::new(),
                aliases: vec![],
                summary: String::new(),
                extra: Default::default(),
            },
            title: PathBuf::from(rel)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into(),
            content: content.into(),
        }
    }

    #[test]
    fn empty_query_returns_empty() {
        let notes = vec![mk_note("Foo.md", "any text")];
        assert!(search(&notes, "", SearchOpts::default()).is_empty());
        assert!(search_titles(&notes, "", SearchOpts::default()).is_empty());
    }

    #[test]
    fn finds_substring_case_insensitive_by_default() {
        let notes = vec![mk_note("Foo.md", "Hello World\nGoodbye")];
        let hits = search(&notes, "hello", SearchOpts::default());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_no, 1);
        assert_eq!(hits[0].snippet, "Hello World");
    }

    #[test]
    fn case_sensitive_respects_opt() {
        let notes = vec![mk_note("Foo.md", "Hello\nhello")];
        let hits = search(
            &notes,
            "Hello",
            SearchOpts {
                case_sensitive: true,
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_no, 1);
    }

    #[test]
    fn multiple_hits_per_note_each_line() {
        let notes = vec![mk_note("Foo.md", "foo\nfoo bar\nbaz foo")];
        let hits = search(&notes, "foo", SearchOpts::default());
        assert_eq!(hits.len(), 3);
        assert_eq!(hits[0].line_no, 1);
        assert_eq!(hits[1].line_no, 2);
        assert_eq!(hits[2].line_no, 3);
    }

    #[test]
    fn search_across_multiple_notes() {
        let notes = vec![
            mk_note("A.md", "alpha here"),
            mk_note("B.md", "beta and alpha"),
            mk_note("C.md", "no match"),
        ];
        let hits = search(&notes, "alpha", SearchOpts::default());
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn limit_truncates_results() {
        let notes = vec![mk_note("Foo.md", "x\nx\nx\nx\nx")];
        let hits = search(
            &notes,
            "x",
            SearchOpts {
                limit: Some(2),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn snippet_is_trimmed() {
        let notes = vec![mk_note("Foo.md", "    indented match here    ")];
        let hits = search(&notes, "match", SearchOpts::default());
        assert_eq!(hits[0].snippet, "indented match here");
    }

    #[test]
    fn titles_match_by_filename_stem() {
        let notes = vec![
            mk_note("SPEC_LGPD.md", "body irrelevant"),
            mk_note("SPEC_QR.md", "body irrelevant"),
        ];
        let hits = search_titles(&notes, "qr", SearchOpts::default());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "SPEC_QR");
    }

    #[test]
    fn search_ignores_empty_notes_gracefully() {
        let notes = vec![mk_note("Empty.md", "")];
        assert!(search(&notes, "anything", SearchOpts::default()).is_empty());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn search_never_panics(query in proptest::prelude::any::<String>(), body in proptest::prelude::any::<String>()) {
            let notes = vec![mk_note("Foo.md", &body)];
            let _ = search(&notes, &query, SearchOpts::default());
            let _ = search_titles(&notes, &query, SearchOpts::default());
        }
    }
}
