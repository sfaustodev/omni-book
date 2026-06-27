//! Auto-tag + summary via LLM. CAD-23.2.
//!
//! [`suggest_tags`] feeds a note to the configured [`LlmProvider`] and parses
//! a structured JSON response into a [`FrontmatterDiff`]. [`apply_diff`]
//! writes the diff back to disk, preserving every other frontmatter field.
//!
//! ## Prompt design
//!
//! The system prompt instructs Claude to respond with ONLY a JSON object:
//!
//! ```json
//! { "tags": ["lowercase-hyphenated", ...], "summary": "one sentence" }
//! ```
//!
//! [`parse_llm_response`] first attempts strict JSON parsing. If Claude
//! ignored the instruction and wrapped the JSON in prose, a tolerant pass
//! extracts the first balanced `{ ... }` block and retries.
//!
//! ## Safety
//!
//! - Tags are sanitised to `[a-z0-9-]` — anything else dropped silently.
//! - Token budget capped via `max_input_chars` (default 6000 ≈ 1500 tokens).
//! - Existing tags are preserved by default (additive merge with dedup).
//! - API key never leaks — relies on [`ProviderError::redact_key`] already
//!   wired in [`AnthropicProvider`].

use crate::provider::{CompleteOpts, LlmProvider, ProviderError};
use omninote_core::types::Note;
use omninote_core::vault::Vault;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Knobs for [`suggest_tags`].
#[derive(Clone, Debug)]
pub struct SuggestOpts {
    /// Hard cap on total tags returned (current ∪ added).
    pub max_tags: usize,
    /// Truncate the note body to this many chars before sending to the LLM.
    /// Default 6000 ≈ 1500 input tokens — enough context, cheap to call.
    pub max_input_chars: usize,
    /// `true` (default) → preserve existing tags + append new ones.
    /// `false` → replace tags entirely with LLM suggestions.
    pub merge_existing: bool,
    /// Override the provider model (else uses [`CompleteOpts::default`]).
    pub model: Option<String>,
}

impl Default for SuggestOpts {
    fn default() -> Self {
        Self {
            max_tags: 5,
            max_input_chars: 6000,
            merge_existing: true,
            model: None,
        }
    }
}

/// Structured diff returned by [`suggest_tags`]. Carries enough context for
/// CLI display, MCP JSON output, and a subsequent [`apply_diff`] call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrontmatterDiff {
    pub note_rel_path: PathBuf,
    pub current_tags: Vec<String>,
    /// Final merged list (what would be written if `apply_diff` is called).
    pub suggested_tags: Vec<String>,
    /// Just the tags being added (delta) — `suggested_tags ∖ current_tags`.
    pub added_tags: Vec<String>,
    pub current_summary: String,
    pub suggested_summary: String,
}

impl FrontmatterDiff {
    /// `true` if applying the diff would change anything on disk.
    pub fn has_changes(&self) -> bool {
        !self.added_tags.is_empty() || self.current_summary != self.suggested_summary
    }

    /// Multi-line human-readable diff for CLI display.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("note: {}\n", self.note_rel_path.display()));
        out.push_str(&format!(
            "current tags: {}\n",
            format_tag_list(&self.current_tags)
        ));
        out.push_str(&format!(
            "suggested tags: {}\n",
            format_tag_list(&self.suggested_tags)
        ));
        if !self.added_tags.is_empty() {
            out.push_str(&format!("  added: {}\n", format_tag_list(&self.added_tags)));
        }
        out.push_str(&format!(
            "current summary: {}\n",
            display_str(&self.current_summary)
        ));
        out.push_str(&format!(
            "suggested summary: {}\n",
            display_str(&self.suggested_summary)
        ));
        out.push_str(&format!("has changes: {}\n", self.has_changes()));
        out
    }
}

fn format_tag_list(tags: &[String]) -> String {
    if tags.is_empty() {
        "(none)".into()
    } else {
        tags.iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn display_str(s: &str) -> String {
    if s.is_empty() {
        "(empty)".into()
    } else {
        s.to_string()
    }
}

/// Raw shape Claude is asked to return. Internal — re-shaped into
/// [`FrontmatterDiff`] by [`merge_diff`].
#[derive(Clone, Debug, Deserialize)]
pub struct RawSuggestion {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

/// Full flow: prompt → LLM → parse → merge. Errors propagate from the
/// provider call OR from a malformed response that survives the fallback
/// parser.
pub async fn suggest_tags<P: LlmProvider + ?Sized>(
    provider: &P,
    note: &Note,
    opts: SuggestOpts,
) -> Result<FrontmatterDiff, ProviderError> {
    let system = build_system_prompt(&opts, &note.frontmatter.tags);
    let user = build_user_prompt(note, &opts);
    let complete_opts = CompleteOpts {
        max_tokens: 1024, // tags + summary are tiny — don't waste context
        temperature: 0.2,
        model: opts.model.clone().unwrap_or(CompleteOpts::default().model),
    };
    let raw_text = provider.complete(&system, &user, complete_opts).await?;
    let raw = parse_llm_response(&raw_text)?;
    Ok(merge_diff(note, raw, &opts))
}

/// Persist a diff back to disk. Reads the note from `vault.notes` by
/// `rel_path`, mutates frontmatter (tags + summary), saves via the same
/// path as [`omninote_core::vault::Vault::save_note`] — which is the
/// already-proven YAML write path (CAD-22 coverage).
pub fn apply_diff(vault: &mut Vault, diff: &FrontmatterDiff) -> Result<PathBuf, String> {
    let idx = vault
        .notes
        .iter()
        .position(|n| n.rel_path == diff.note_rel_path)
        .ok_or_else(|| format!("note not found in vault: {}", diff.note_rel_path.display()))?;
    let note = vault.notes[idx].clone();
    let mut updated = note;
    updated.frontmatter.tags = diff.suggested_tags.clone();
    updated.frontmatter.summary = diff.suggested_summary.clone();
    vault.save_note(&updated)?;
    vault.notes[idx] = updated.clone();
    Ok(updated.path)
}

// ──────────────────────── pure helpers (unit-testable) ────────────────────────

pub(crate) fn build_system_prompt(opts: &SuggestOpts, existing_tags: &[String]) -> String {
    let mut s = String::with_capacity(512);
    s.push_str("You analyze personal notes and suggest tags + a one-line summary.\n\n");
    s.push_str("Respond ONLY with valid JSON matching this schema:\n");
    s.push_str("{ \"tags\": [\"string\", ...], \"summary\": \"string\" }\n\n");
    s.push_str("Rules:\n");
    s.push_str(&format!(
        "- Suggest at most {} tags total.\n",
        opts.max_tags
    ));
    s.push_str("- Tags are lowercase, ASCII letters/digits and hyphens only (no spaces, no punctuation).\n");
    s.push_str("- Summary is one sentence, no more than 120 characters.\n");
    if !existing_tags.is_empty() {
        s.push_str(&format!(
            "- The note already has these tags (you may reuse, drop, or extend): {}\n",
            existing_tags
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    s.push_str("- Do NOT add markdown, prose, code fences, or explanations outside the JSON.\n");
    s
}

pub(crate) fn build_user_prompt(note: &Note, opts: &SuggestOpts) -> String {
    let truncated = truncate_for_prompt(&note.content, opts.max_input_chars);
    let mut s = String::with_capacity(truncated.len() + 128);
    s.push_str("Title: ");
    s.push_str(&note.title);
    s.push_str("\n\nBody:\n");
    s.push_str(&truncated);
    if note.content.len() > truncated.len() {
        s.push_str("\n\n[note truncated for token budget]");
    }
    s
}

fn truncate_for_prompt(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }
    // Snap to the previous char boundary so we don't slice mid-multibyte.
    let mut end = max_chars;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    content[..end].to_string()
}

/// Strict-first, then tolerant JSON extraction. Returns the structured
/// suggestion or [`ProviderError::Api`] with a non-leaky description.
pub(crate) fn parse_llm_response(text: &str) -> Result<RawSuggestion, ProviderError> {
    let trimmed = text.trim();
    if let Ok(raw) = serde_json::from_str::<RawSuggestion>(trimmed) {
        return Ok(raw);
    }
    // Fallback: find the first balanced `{...}` block and retry.
    if let Some(block) = extract_first_json_block(trimmed) {
        if let Ok(raw) = serde_json::from_str::<RawSuggestion>(&block) {
            return Ok(raw);
        }
    }
    Err(ProviderError::Api {
        status: 200,
        body: format!(
            "LLM response could not be parsed as JSON {{tags, summary}}: {}",
            text.chars().take(200).collect::<String>()
        ),
    })
}

/// Tiny balanced-brace scanner. Returns the first `{...}` block (including
/// braces) at depth 0, or `None`. Ignores braces inside strings.
fn extract_first_json_block(s: &str) -> Option<String> {
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    let mut in_str = false;
    let mut escape = false;
    for (i, ch) in s.char_indices() {
        if in_str {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s_idx) = start {
                        return Some(s[s_idx..=i].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn merge_diff(note: &Note, raw: RawSuggestion, opts: &SuggestOpts) -> FrontmatterDiff {
    let sanitized: Vec<String> = raw
        .tags
        .into_iter()
        .map(|t| sanitize_tag(&t))
        .filter(|t| !t.is_empty())
        .collect();

    let current_tags = note.frontmatter.tags.clone();

    let mut merged: Vec<String> = if opts.merge_existing {
        current_tags.clone()
    } else {
        Vec::new()
    };
    for tag in sanitized {
        if !merged.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            merged.push(tag);
            if merged.len() >= opts.max_tags {
                break;
            }
        }
    }
    merged.truncate(opts.max_tags);

    let added_tags: Vec<String> = merged
        .iter()
        .filter(|t| !current_tags.iter().any(|c| c.eq_ignore_ascii_case(t)))
        .cloned()
        .collect();

    let summary = raw.summary.trim().to_string();
    let suggested_summary = if summary.len() > 200 {
        summary.chars().take(200).collect::<String>()
    } else {
        summary
    };

    FrontmatterDiff {
        note_rel_path: note.rel_path.clone(),
        current_tags,
        suggested_tags: merged,
        added_tags,
        current_summary: note.frontmatter.summary.clone(),
        suggested_summary,
    }
}

/// Reduce a candidate tag to `[a-z0-9-]`. Spaces collapse to `-`. Anything
/// else dropped. Empty result means the candidate was unusable.
fn sanitize_tag(t: &str) -> String {
    let lower = t.trim().trim_start_matches('#').to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut prev_dash = false;
    for ch in lower.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch)
        } else if ch == '-' || ch == '_' || ch.is_whitespace() {
            Some('-')
        } else {
            None
        };
        if let Some(c) = mapped {
            if c == '-' {
                if !prev_dash && !out.is_empty() {
                    out.push('-');
                    prev_dash = true;
                }
            } else {
                out.push(c);
                prev_dash = false;
            }
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockProvider;
    use omninote_core::types::{Frontmatter, NoteType};
    use std::path::PathBuf;

    fn mk_note(rel: &str, content: &str, tags: Vec<&str>) -> Note {
        Note {
            path: PathBuf::from("/tmp/vault").join(rel),
            rel_path: PathBuf::from(rel),
            frontmatter: Frontmatter {
                id: rel.into(),
                note_type: NoteType::Resumo,
                tags: tags.into_iter().map(String::from).collect(),
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

    // ───── prompts ─────

    #[test]
    fn build_system_prompt_includes_max_tags_cap() {
        let opts = SuggestOpts {
            max_tags: 7,
            ..Default::default()
        };
        let p = build_system_prompt(&opts, &[]);
        assert!(p.contains("at most 7"));
    }

    #[test]
    fn build_system_prompt_omits_existing_section_when_no_tags() {
        let p = build_system_prompt(&SuggestOpts::default(), &[]);
        assert!(!p.contains("already has these tags"));
    }

    #[test]
    fn build_system_prompt_lists_existing_tags_when_present() {
        let p = build_system_prompt(
            &SuggestOpts::default(),
            &["rust".into(), "discipline".into()],
        );
        assert!(p.contains("already has these tags"));
        assert!(p.contains("\"rust\""));
        assert!(p.contains("\"discipline\""));
    }

    #[test]
    fn build_user_prompt_includes_title_and_body() {
        let note = mk_note("Foo.md", "body content", vec![]);
        let p = build_user_prompt(&note, &SuggestOpts::default());
        assert!(p.contains("Title: Foo"));
        assert!(p.contains("body content"));
    }

    #[test]
    fn build_user_prompt_truncates_long_body() {
        let long = "a".repeat(10_000);
        let note = mk_note("L.md", &long, vec![]);
        let opts = SuggestOpts {
            max_input_chars: 100,
            ..Default::default()
        };
        let p = build_user_prompt(&note, &opts);
        assert!(p.contains("[note truncated"));
        // Body portion should be ≤ max_input_chars + title preamble length.
        assert!(p.len() < 500);
    }

    #[test]
    fn truncate_for_prompt_respects_char_boundaries() {
        let s = "café";
        // 'é' is 2 bytes — truncating mid-char would panic without snap.
        let out = truncate_for_prompt(s, 4);
        assert!(s.starts_with(&out));
    }

    // ───── JSON parser ─────

    #[test]
    fn parse_llm_response_clean_json() {
        let raw = r#"{"tags": ["rust", "cli"], "summary": "A note."}"#;
        let r = parse_llm_response(raw).unwrap();
        assert_eq!(r.tags, vec!["rust", "cli"]);
        assert_eq!(r.summary, "A note.");
    }

    #[test]
    fn parse_llm_response_extracts_from_prose_wrapped() {
        let raw = r#"Here is the JSON you requested:
        ```json
        {"tags": ["rust", "cli"], "summary": "A note."}
        ```
        Hope this helps!"#;
        let r = parse_llm_response(raw).unwrap();
        assert_eq!(r.tags, vec!["rust", "cli"]);
        assert_eq!(r.summary, "A note.");
    }

    #[test]
    fn parse_llm_response_extracts_first_block_when_multiple() {
        let raw =
            r#"{"tags": ["a"], "summary": "first"} and also {"tags": ["b"], "summary": "second"}"#;
        let r = parse_llm_response(raw).unwrap();
        assert_eq!(r.tags, vec!["a"]);
        assert_eq!(r.summary, "first");
    }

    #[test]
    fn parse_llm_response_handles_strings_with_braces() {
        let raw = r#"{"tags": ["rust"], "summary": "uses {curly braces}"}"#;
        let r = parse_llm_response(raw).unwrap();
        assert_eq!(r.summary, "uses {curly braces}");
    }

    #[test]
    fn parse_llm_response_errs_on_no_json() {
        let raw = "I refuse to answer that question.";
        let r = parse_llm_response(raw);
        assert!(matches!(r, Err(ProviderError::Api { .. })));
    }

    #[test]
    fn parse_llm_response_errs_when_block_is_malformed() {
        let raw = r#"{"tags": ["rust", missing-quote], "summary": ""}"#;
        let r = parse_llm_response(raw);
        assert!(matches!(r, Err(ProviderError::Api { .. })));
    }

    #[test]
    fn parse_llm_response_accepts_missing_fields_as_empty() {
        // Both fields have #[serde(default)] so partial JSON deserializes.
        let raw = r#"{}"#;
        let r = parse_llm_response(raw).unwrap();
        assert!(r.tags.is_empty());
        assert!(r.summary.is_empty());
    }

    // ───── sanitize_tag ─────

    #[test]
    fn sanitize_tag_lowercases_and_keeps_hyphens() {
        assert_eq!(sanitize_tag("Rust-Lang"), "rust-lang");
    }

    #[test]
    fn sanitize_tag_strips_leading_hash() {
        assert_eq!(sanitize_tag("#rust"), "rust");
    }

    #[test]
    fn sanitize_tag_replaces_spaces_with_dashes() {
        assert_eq!(sanitize_tag("hello world"), "hello-world");
    }

    #[test]
    fn sanitize_tag_collapses_multiple_separators() {
        assert_eq!(sanitize_tag("foo   bar"), "foo-bar");
        assert_eq!(sanitize_tag("foo___bar"), "foo-bar");
    }

    #[test]
    fn sanitize_tag_drops_emoji_and_punctuation() {
        assert_eq!(sanitize_tag("🦀rust!"), "rust");
    }

    #[test]
    fn sanitize_tag_returns_empty_for_pure_garbage() {
        assert_eq!(sanitize_tag("✨✨✨"), "");
    }

    // ───── merge_diff ─────

    #[test]
    fn merge_diff_preserves_existing_tags_additive_mode() {
        let note = mk_note("F.md", "", vec!["rust"]);
        let raw = RawSuggestion {
            tags: vec!["cli".into(), "discipline".into()],
            summary: "Brief.".into(),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        assert_eq!(diff.current_tags, vec!["rust"]);
        assert_eq!(diff.suggested_tags, vec!["rust", "cli", "discipline"]);
        assert_eq!(diff.added_tags, vec!["cli", "discipline"]);
    }

    #[test]
    fn merge_diff_replace_mode_drops_existing() {
        let note = mk_note("F.md", "", vec!["rust"]);
        let raw = RawSuggestion {
            tags: vec!["cli".into()],
            summary: String::new(),
        };
        let opts = SuggestOpts {
            merge_existing: false,
            ..Default::default()
        };
        let diff = merge_diff(&note, raw, &opts);
        assert_eq!(diff.suggested_tags, vec!["cli"]);
        assert_eq!(diff.added_tags, vec!["cli"]);
        // current_tags still reported for display purposes.
        assert_eq!(diff.current_tags, vec!["rust"]);
    }

    #[test]
    fn merge_diff_caps_at_max_tags() {
        let note = mk_note("F.md", "", vec!["a", "b"]);
        let raw = RawSuggestion {
            tags: (0..20).map(|i| format!("t{i}")).collect(),
            summary: String::new(),
        };
        let opts = SuggestOpts {
            max_tags: 5,
            ..Default::default()
        };
        let diff = merge_diff(&note, raw, &opts);
        assert_eq!(diff.suggested_tags.len(), 5);
    }

    #[test]
    fn merge_diff_dedupes_case_insensitive() {
        let note = mk_note("F.md", "", vec!["Rust"]);
        let raw = RawSuggestion {
            tags: vec!["rust".into(), "RUST".into(), "cli".into()],
            summary: String::new(),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        assert_eq!(diff.suggested_tags, vec!["Rust", "cli"]);
        assert_eq!(diff.added_tags, vec!["cli"]);
    }

    #[test]
    fn merge_diff_sanitizes_dangerous_input() {
        let note = mk_note("F.md", "", vec![]);
        let raw = RawSuggestion {
            tags: vec![
                "Hello World".into(),
                "🦀-rust".into(),
                "<script>".into(),
                "".into(),
            ],
            summary: String::new(),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        // hello-world, rust, script (sanitized), empty dropped
        assert!(diff.suggested_tags.contains(&"hello-world".to_string()));
        assert!(diff.suggested_tags.contains(&"rust".to_string()));
        assert!(diff.suggested_tags.contains(&"script".to_string()));
        assert!(!diff
            .suggested_tags
            .iter()
            .any(|t| t.contains('<') || t.contains('>')));
    }

    #[test]
    fn merge_diff_truncates_overlong_summary() {
        let note = mk_note("F.md", "", vec![]);
        let raw = RawSuggestion {
            tags: vec![],
            summary: "a".repeat(500),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        assert_eq!(diff.suggested_summary.len(), 200);
    }

    #[test]
    fn merge_diff_has_changes_reports_correctly() {
        let note = mk_note("F.md", "", vec!["rust"]);
        // No new tags, no summary change.
        let raw = RawSuggestion {
            tags: vec!["rust".into()],
            summary: String::new(),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        assert!(!diff.has_changes());

        // Summary changes → has_changes.
        let raw = RawSuggestion {
            tags: vec!["rust".into()],
            summary: "new summary".into(),
        };
        let diff = merge_diff(&note, raw, &SuggestOpts::default());
        assert!(diff.has_changes());
    }

    // ───── full flow with MockProvider ─────

    #[tokio::test]
    async fn suggest_tags_full_flow_with_mock() {
        let note = mk_note("Demo.md", "Some rust async code here.", vec!["existing"]);
        let canned =
            r#"{"tags": ["rust", "async", "code"], "summary": "Demo of async Rust code."}"#;
        let provider = MockProvider::ok(canned);
        let diff = suggest_tags(&provider, &note, SuggestOpts::default())
            .await
            .unwrap();
        assert_eq!(diff.current_tags, vec!["existing"]);
        assert_eq!(
            diff.suggested_tags,
            vec!["existing", "rust", "async", "code"]
        );
        assert_eq!(diff.suggested_summary, "Demo of async Rust code.");
        assert!(diff.has_changes());
    }

    #[tokio::test]
    async fn suggest_tags_passes_existing_tags_to_prompt() {
        let note = mk_note("X.md", "body", vec!["rust", "cli"]);
        let canned = r#"{"tags": [], "summary": ""}"#;
        let provider = MockProvider::ok(canned);
        let _ = suggest_tags(&provider, &note, SuggestOpts::default())
            .await
            .unwrap();
        let last = provider.last_call.lock().unwrap().clone();
        let (sys, _user, _opts) = last.unwrap();
        assert!(sys.contains("\"rust\""));
        assert!(sys.contains("\"cli\""));
    }

    #[tokio::test]
    async fn suggest_tags_propagates_provider_error() {
        let note = mk_note("X.md", "body", vec![]);
        let provider = MockProvider::err(ProviderError::MissingKey);
        let r = suggest_tags(&provider, &note, SuggestOpts::default()).await;
        assert!(matches!(r, Err(ProviderError::MissingKey)));
    }

    #[tokio::test]
    async fn suggest_tags_propagates_parse_error_when_llm_garbage() {
        let note = mk_note("X.md", "body", vec![]);
        let provider = MockProvider::ok("I'm a helpful assistant, here is no JSON for you.");
        let r = suggest_tags(&provider, &note, SuggestOpts::default()).await;
        assert!(matches!(r, Err(ProviderError::Api { .. })));
    }

    // ───── apply_diff ─────

    #[test]
    fn apply_diff_writes_frontmatter_and_preserves_other_keys() {
        // Use a real Vault so save_note/reload_notes paths are exercised.
        let tmp = tempfile::tempdir().unwrap();
        let mut vault = omninote_core::vault::Vault::open(tmp.path().to_path_buf()).unwrap();
        let mut note = vault
            .create_note(None, "ApplyMe", NoteType::Resumo)
            .unwrap();
        note.frontmatter.tags = vec!["existing".into()];
        note.frontmatter.source = "Livro Importante".into();
        vault.save_note(&note).unwrap();
        vault.reload_notes();

        let diff = FrontmatterDiff {
            note_rel_path: note.rel_path.clone(),
            current_tags: vec!["existing".into()],
            suggested_tags: vec!["existing".into(), "novo".into()],
            added_tags: vec!["novo".into()],
            current_summary: String::new(),
            suggested_summary: "Resumo aplicado.".into(),
        };

        let written = apply_diff(&mut vault, &diff).unwrap();
        assert!(written.exists());
        vault.reload_notes();
        let reloaded = vault
            .notes
            .iter()
            .find(|n| n.frontmatter.id == note.frontmatter.id)
            .unwrap();
        assert_eq!(reloaded.frontmatter.tags, vec!["existing", "novo"]);
        assert_eq!(reloaded.frontmatter.summary, "Resumo aplicado.");
        // Critical: other keys must survive the round-trip.
        assert_eq!(reloaded.frontmatter.source, "Livro Importante");
    }

    #[test]
    fn apply_diff_errs_when_note_not_in_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let mut vault = omninote_core::vault::Vault::open(tmp.path().to_path_buf()).unwrap();
        let diff = FrontmatterDiff {
            note_rel_path: PathBuf::from("nope.md"),
            current_tags: vec![],
            suggested_tags: vec![],
            added_tags: vec![],
            current_summary: String::new(),
            suggested_summary: String::new(),
        };
        let r = apply_diff(&mut vault, &diff);
        assert!(r.is_err());
    }

    // ───── extract_first_json_block edge cases ─────

    #[test]
    fn extract_first_json_block_returns_none_when_no_brace() {
        assert!(extract_first_json_block("plain text").is_none());
    }

    #[test]
    fn extract_first_json_block_handles_nested_braces() {
        let s = r#"text {"a": {"b": 1}, "c": 2} more"#;
        let block = extract_first_json_block(s).unwrap();
        assert_eq!(block, r#"{"a": {"b": 1}, "c": 2}"#);
    }

    #[test]
    fn extract_first_json_block_skips_braces_inside_strings() {
        let s = r#"{"key": "value with } and { inside"}"#;
        let block = extract_first_json_block(s).unwrap();
        assert_eq!(block, s);
    }

    #[test]
    fn extract_first_json_block_returns_none_on_unbalanced() {
        // Has `{` but no matching close.
        assert!(extract_first_json_block("{no close").is_none());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn parse_llm_response_never_panics(s in proptest::prelude::any::<String>()) {
            let _ = parse_llm_response(&s);
        }

        #[test]
        fn sanitize_tag_never_panics(s in proptest::prelude::any::<String>()) {
            let _ = sanitize_tag(&s);
        }

        #[test]
        fn extract_first_json_block_never_panics(s in proptest::prelude::any::<String>()) {
            let _ = extract_first_json_block(&s);
        }
    }
}
