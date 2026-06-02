//! Obsidian-compatible wikilink parser.
//!
//! Grammar (CAD-20):
//! - `[[target]]`                 → note link by name/path
//! - `[[target|alias]]`           → note link with display alias
//! - `[[target#heading]]`         → note link with heading anchor
//! - `[[target#^block-id]]`       → note link with block anchor
//! - `[[folder/path/note]]`       → path-based note link
//! - `![[image.png]]`             → image embed (png/jpg/jpeg/gif/webp/bmp)
//! - `![[file.pdf]]`              → non-image file embed
//! - `![[Note]]` / `![[Note.md]]` → embed of another note's content
//! - `![[Note#Heading]]`          → embed of a heading section
//! - `![[path|alt]]`              → embed with alias/alt-text
//!
//! Resolution from [`NoteRef::target`] to an actual `Note` happens in
//! [`crate::resolver`]. This module only does syntactic extraction.

use std::path::Path;

/// A `#anchor` or `#^block-id` reference inside a wikilink target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// `#Heading` — jump to heading anchor (may include nested `#`, e.g. `H1#H2`)
    Heading(String),
    /// `#^block-id` — jump to block anchor (a line marked with `^id`)
    Block(String),
}

/// Reference to another note (with optional alias and anchor).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRef {
    /// Raw target text (filename, `folder/path/note`, or `path/note.md`).
    pub target: String,
    /// Display label override from `[[target|alias]]`.
    pub alias: Option<String>,
    /// `#Heading` or `#^block-id` suffix.
    pub anchor: Option<Anchor>,
}

/// Reference to an embedded asset (image or file) on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbedRef {
    /// Filename or relative path inside `_attachments/`.
    pub path: String,
    /// Alt-text / display alias from `![[path|alt]]`.
    pub alias: Option<String>,
}

/// Wikilink or embed extracted from note content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wikilink {
    /// `[[Title]]`, `[[Title|alias]]`, `[[Title#Heading]]`, `[[folder/path]]`
    Note(NoteRef),
    /// `![[Note]]` or `![[Note#Heading]]` — embed another note's content
    NoteEmbed(NoteRef),
    /// `![[image.png]]` — embedded image (png/jpg/jpeg/gif/webp/bmp)
    Image(EmbedRef),
    /// `![[file.pdf]]` — non-image, non-note file embed
    File(EmbedRef),
}

impl Wikilink {
    /// Convenience: get the raw target string for any variant.
    /// Consumed by `resolver::VaultIndex::unresolved_links` and by the CLI
    /// `link unresolved` verb (wired in CAD-21 Phase 2).
    #[allow(dead_code)]
    pub fn target_str(&self) -> &str {
        match self {
            Self::Note(r) | Self::NoteEmbed(r) => &r.target,
            Self::Image(r) | Self::File(r) => &r.path,
        }
    }

    /// True if this is a note reference (link or embed), false if it points to an asset.
    /// Consumed by the CLI/MCP `link unresolved` verb (wired in CAD-21 Phase 2).
    #[allow(dead_code)]
    pub fn is_note(&self) -> bool {
        matches!(self, Self::Note(_) | Self::NoteEmbed(_))
    }
}

/// Extract all `[[...]]` and `![[...]]` references from `content`, in order.
/// Duplicates preserved (caller can dedupe if needed). Never panics on arbitrary input.
///
/// CAD-20: skips matches inside fenced code blocks (```...```) and inline code spans
/// (`...`). This prevents false positives like the TOML `[[package]]` array-of-tables
/// header bleeding into the wikilink set when notes contain Cargo.toml diff snippets.
pub fn extract(content: &str) -> Vec<Wikilink> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut in_fence = false;
    let mut at_line_start = true;
    let mut in_inline_code = false;

    while i < bytes.len() {
        // Toggle fenced code block on ``` at start of a line.
        if at_line_start
            && i + 2 < bytes.len()
            && bytes[i] == b'`'
            && bytes[i + 1] == b'`'
            && bytes[i + 2] == b'`'
        {
            in_fence = !in_fence;
            i += 3;
            at_line_start = false;
            continue;
        }
        // Toggle inline code span on single backtick (only outside fenced blocks).
        if !in_fence && bytes[i] == b'`' {
            in_inline_code = !in_inline_code;
            i += 1;
            at_line_start = false;
            continue;
        }
        if bytes[i] == b'\n' {
            at_line_start = true;
            // Inline code spans don't cross newlines per CommonMark.
            in_inline_code = false;
            i += 1;
            continue;
        }
        if !bytes[i].is_ascii_whitespace() {
            at_line_start = false;
        }

        // Skip wikilink detection inside any code region.
        if in_fence || in_inline_code {
            i += 1;
            continue;
        }

        let is_embed =
            i + 2 < bytes.len() && bytes[i] == b'!' && bytes[i + 1] == b'[' && bytes[i + 2] == b'[';
        let is_link = !is_embed && i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[';

        if !is_link && !is_embed {
            i += 1;
            continue;
        }

        let start = if is_embed { i + 3 } else { i + 2 };
        let mut end = start;
        while end + 1 < bytes.len() && !(bytes[end] == b']' && bytes[end + 1] == b']') {
            end += 1;
        }
        if end + 1 >= bytes.len() {
            break; // unclosed
        }

        let inner = content[start..end].trim();
        if !inner.is_empty() {
            if is_embed {
                out.push(classify_embed(inner));
            } else {
                out.push(parse_note_link(inner));
            }
        }
        i = end + 2;
    }

    out
}

/// Like [`extract`], but also returns each link's byte span in `content`
/// (covering the full `[[…]]` / `![[…]]` match including brackets). Used by the
/// GUI inline renderer (CAD-25 Slice 3) to splice clickable widgets into the
/// surrounding markdown text. Same code-region skipping as `extract`.
pub fn extract_spans(content: &str) -> Vec<(Wikilink, std::ops::Range<usize>)> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut in_fence = false;
    let mut at_line_start = true;
    let mut in_inline_code = false;

    while i < bytes.len() {
        if at_line_start
            && i + 2 < bytes.len()
            && bytes[i] == b'`'
            && bytes[i + 1] == b'`'
            && bytes[i + 2] == b'`'
        {
            in_fence = !in_fence;
            i += 3;
            at_line_start = false;
            continue;
        }
        if !in_fence && bytes[i] == b'`' {
            in_inline_code = !in_inline_code;
            i += 1;
            at_line_start = false;
            continue;
        }
        if bytes[i] == b'\n' {
            at_line_start = true;
            in_inline_code = false;
            i += 1;
            continue;
        }
        if !bytes[i].is_ascii_whitespace() {
            at_line_start = false;
        }
        if in_fence || in_inline_code {
            i += 1;
            continue;
        }

        let is_embed =
            i + 2 < bytes.len() && bytes[i] == b'!' && bytes[i + 1] == b'[' && bytes[i + 2] == b'[';
        let is_link = !is_embed && i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[';
        if !is_link && !is_embed {
            i += 1;
            continue;
        }

        let match_start = i;
        let start = if is_embed { i + 3 } else { i + 2 };
        let mut end = start;
        while end + 1 < bytes.len() && !(bytes[end] == b']' && bytes[end + 1] == b']') {
            end += 1;
        }
        if end + 1 >= bytes.len() {
            break;
        }

        let inner = content[start..end].trim();
        if !inner.is_empty() {
            let link = if is_embed {
                classify_embed(inner)
            } else {
                parse_note_link(inner)
            };
            out.push((link, match_start..end + 2));
        }
        i = end + 2;
    }

    out
}

/// Inline tag extraction (`#tag` in body content, not frontmatter).
/// Returns tags in order of appearance. Skips `#` inside code blocks and wikilinks.
///
/// Rules (CAD-20):
/// - Tag starts at `#` preceded by whitespace, start-of-line, or punctuation (not letter/digit).
/// - Tag body = `[A-Za-z0-9_/-]+` (allows nested tags like `#proj/sub`).
/// - Wikilink anchors (`[[Note#Heading]]`) and block refs (`#^block-id`) are **not** tags.
/// - Heading lines (`# Title` with a space) are **not** tags.
///
/// Wired into the sidebar tag chip filter in CAD-25 Fase B (UI Design v2).
#[allow(dead_code)]
pub fn extract_inline_tags(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut in_code_block = false;
    let mut at_line_start = true;
    let mut inside_wikilink_brackets: i32 = 0;

    while i < bytes.len() {
        // Track fenced code blocks (```)
        if at_line_start
            && i + 2 < bytes.len()
            && bytes[i] == b'`'
            && bytes[i + 1] == b'`'
            && bytes[i + 2] == b'`'
        {
            in_code_block = !in_code_block;
            i += 3;
            at_line_start = false;
            continue;
        }
        // Track wikilink brackets so `[[Note#Heading]]` doesn't yield "#Heading" as a tag.
        if i + 1 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'[' {
            inside_wikilink_brackets += 1;
            i += 2;
            at_line_start = false;
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b']' && bytes[i + 1] == b']' {
            inside_wikilink_brackets = (inside_wikilink_brackets - 1).max(0);
            i += 2;
            at_line_start = false;
            continue;
        }

        let c = bytes[i];
        if c == b'\n' {
            at_line_start = true;
            i += 1;
            continue;
        }
        if !in_code_block && inside_wikilink_brackets == 0 && c == b'#' {
            // Reject heading lines: `#` followed by ` ` at line start.
            if at_line_start && i + 1 < bytes.len() && bytes[i + 1] == b' ' {
                at_line_start = false;
                i += 1;
                continue;
            }
            // Previous char must be whitespace or start-of-content.
            let prev_ok = i == 0
                || matches!(
                    bytes[i - 1],
                    b' ' | b'\t' | b'\n' | b'(' | b'[' | b'{' | b',' | b';' | b'!' | b'?'
                );
            // Block-ref `#^` is not a tag.
            let is_block_ref = i + 1 < bytes.len() && bytes[i + 1] == b'^';
            if prev_ok && !is_block_ref {
                let mut j = i + 1;
                while j < bytes.len() {
                    let b = bytes[j];
                    if b.is_ascii_alphanumeric() || b == b'_' || b == b'/' || b == b'-' {
                        j += 1;
                    } else {
                        break;
                    }
                }
                if j > i + 1 {
                    let tag = content[i + 1..j].to_string();
                    out.push(tag);
                }
                i = j;
                at_line_start = false;
                continue;
            }
        }
        if !c.is_ascii_whitespace() {
            at_line_start = false;
        }
        i += 1;
    }

    out
}

/// Extract the body under an ATX heading whose text matches `heading`
/// (case-insensitive, trimmed), up to the next heading of the same or shallower
/// depth. Used by `![[Note#Heading]]` embeds (CAD-25) to show just that section.
/// Returns `None` if the heading isn't found. Skips fenced code so a `#` inside
/// a code block isn't treated as a heading boundary.
pub fn section_under_heading(content: &str, heading: &str) -> Option<String> {
    let want = heading.trim();
    let mut in_fence: Option<char> = None;
    let mut collecting: Option<usize> = None; // depth of the matched heading
    let mut out: Vec<&str> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        // Fence tracking (same kind-aware logic as the outline parser).
        match in_fence {
            None if trimmed.starts_with("```") => {
                in_fence = Some('`');
                if collecting.is_some() {
                    out.push(line);
                }
                continue;
            }
            None if trimmed.starts_with("~~~") => {
                in_fence = Some('~');
                if collecting.is_some() {
                    out.push(line);
                }
                continue;
            }
            Some('`') if trimmed.starts_with("```") => {
                in_fence = None;
                if collecting.is_some() {
                    out.push(line);
                }
                continue;
            }
            Some('~') if trimmed.starts_with("~~~") => {
                in_fence = None;
                if collecting.is_some() {
                    out.push(line);
                }
                continue;
            }
            Some(_) => {
                if collecting.is_some() {
                    out.push(line);
                }
                continue;
            }
            None => {}
        }

        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        let is_heading = (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ');

        if let Some(depth) = collecting {
            // A heading of same-or-shallower depth ends the section.
            if is_heading && hashes <= depth {
                break;
            }
            out.push(line);
        } else if is_heading && trimmed[hashes..].trim().eq_ignore_ascii_case(want) {
            collecting = Some(hashes);
        }
    }

    collecting?;
    Some(out.join("\n").trim().to_string())
}

// ---------- internals ----------

fn classify_embed(inner: &str) -> Wikilink {
    let (target, alias, anchor) = parse_inner(inner);
    if is_image_extension(&target) {
        Wikilink::Image(EmbedRef {
            path: target,
            alias,
        })
    } else if is_note_target(&target) {
        Wikilink::NoteEmbed(NoteRef {
            target,
            alias,
            anchor,
        })
    } else {
        // Non-image, non-note → opaque file
        Wikilink::File(EmbedRef {
            path: target,
            alias,
        })
    }
}

fn parse_note_link(inner: &str) -> Wikilink {
    let (target, alias, anchor) = parse_inner(inner);
    Wikilink::Note(NoteRef {
        target,
        alias,
        anchor,
    })
}

/// Splits inner text on `|` (alias) then `#` (anchor).
///
/// Returns `(target, alias, anchor)`. `#^block-id` becomes `Anchor::Block`,
/// everything else after `#` becomes `Anchor::Heading`.
fn parse_inner(inner: &str) -> (String, Option<String>, Option<Anchor>) {
    let (no_alias, alias) = match inner.split_once('|') {
        Some((l, r)) => {
            let a = r.trim();
            (
                l.trim(),
                if a.is_empty() {
                    None
                } else {
                    Some(a.to_string())
                },
            )
        }
        None => (inner.trim(), None),
    };
    let (target, anchor) = match no_alias.split_once('#') {
        Some((t, a)) => {
            let a_trim = a.trim();
            if a_trim.is_empty() {
                (t.trim().to_string(), None)
            } else if let Some(block_id) = a_trim.strip_prefix('^') {
                let id = block_id.trim().to_string();
                if id.is_empty() {
                    (t.trim().to_string(), None)
                } else {
                    (t.trim().to_string(), Some(Anchor::Block(id)))
                }
            } else {
                (
                    t.trim().to_string(),
                    Some(Anchor::Heading(a_trim.to_string())),
                )
            }
        }
        None => (no_alias.to_string(), None),
    };
    (target, alias, anchor)
}

fn is_image_extension(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let path = Path::new(&lower);
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

/// True if a name should be treated as a note (for embed classification).
/// `.md` extension or no extension at all → note. Anything else (with extension) → file.
fn is_note_target(name: &str) -> bool {
    let path = Path::new(name);
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("md"),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- basic note links -----

    #[test]
    fn extracts_simple_note_link() {
        let c = "veja [[Título da Nota]] pra contexto";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        assert!(
            matches!(&r[0], Wikilink::Note(n) if n.target == "Título da Nota" && n.alias.is_none() && n.anchor.is_none())
        );
    }

    #[test]
    fn extracts_alias_link() {
        let c = "veja [[SPEC_V2 - NdA|spec da NdA]] aqui";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "SPEC_V2 - NdA");
            assert_eq!(n.alias.as_deref(), Some("spec da NdA"));
            assert!(n.anchor.is_none());
        } else {
            panic!("expected Note");
        }
    }

    #[test]
    fn extracts_heading_anchor() {
        let c = "veja [[Note#Section 8.2]] aqui";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert_eq!(n.anchor, Some(Anchor::Heading("Section 8.2".to_string())));
        } else {
            panic!("expected Note");
        }
    }

    #[test]
    fn extracts_block_anchor() {
        let c = "veja [[Note#^abc123]] aqui";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert_eq!(n.anchor, Some(Anchor::Block("abc123".to_string())));
        } else {
            panic!("expected Note");
        }
    }

    #[test]
    fn extracts_path_target() {
        let c = "veja [[Projects/CFO/specs/SPEC_LGPD]] aqui";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Projects/CFO/specs/SPEC_LGPD");
        } else {
            panic!("expected Note");
        }
    }

    #[test]
    fn extracts_path_with_alias_and_anchor() {
        let c = "[[folder/Note#Heading|nick]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "folder/Note");
            assert_eq!(n.alias.as_deref(), Some("nick"));
            assert_eq!(n.anchor, Some(Anchor::Heading("Heading".to_string())));
        } else {
            panic!("expected Note");
        }
    }

    // ----- embeds -----

    #[test]
    fn extracts_image_embed() {
        let c = "antes ![[diagrama.png]] depois";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        if let Wikilink::Image(e) = &r[0] {
            assert_eq!(e.path, "diagrama.png");
        } else {
            panic!("expected Image");
        }
    }

    #[test]
    fn extracts_image_embed_with_alias() {
        let c = "![[foto.jpg|alt text aqui]]";
        let r = extract(c);
        if let Wikilink::Image(e) = &r[0] {
            assert_eq!(e.path, "foto.jpg");
            assert_eq!(e.alias.as_deref(), Some("alt text aqui"));
        } else {
            panic!("expected Image");
        }
    }

    #[test]
    fn extracts_pdf_embed_as_file() {
        let c = "spec ![[manual.pdf]]";
        let r = extract(c);
        if let Wikilink::File(e) = &r[0] {
            assert_eq!(e.path, "manual.pdf");
        } else {
            panic!("expected File");
        }
    }

    #[test]
    fn embed_no_extension_is_note_embed() {
        let c = "![[Some Note]]";
        let r = extract(c);
        if let Wikilink::NoteEmbed(n) = &r[0] {
            assert_eq!(n.target, "Some Note");
        } else {
            panic!("expected NoteEmbed, got {:?}", r);
        }
    }

    #[test]
    fn embed_md_extension_is_note_embed() {
        let c = "![[Some Note.md]]";
        let r = extract(c);
        if let Wikilink::NoteEmbed(n) = &r[0] {
            assert_eq!(n.target, "Some Note.md");
        } else {
            panic!("expected NoteEmbed");
        }
    }

    #[test]
    fn embed_note_with_heading_anchor() {
        let c = "![[Note#Section]]";
        let r = extract(c);
        if let Wikilink::NoteEmbed(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert_eq!(n.anchor, Some(Anchor::Heading("Section".to_string())));
        } else {
            panic!("expected NoteEmbed");
        }
    }

    #[test]
    fn all_image_extensions_classified() {
        for ext in ["png", "jpg", "jpeg", "gif", "webp", "bmp"] {
            let c = format!("![[img.{ext}]]");
            let r = extract(&c);
            assert!(
                matches!(r[0], Wikilink::Image(_)),
                "ext={ext} should be Image"
            );
        }
    }

    #[test]
    fn all_image_extensions_uppercase_classified() {
        for ext in ["PNG", "JPG", "JPEG", "GIF", "WEBP", "BMP"] {
            let c = format!("![[img.{ext}]]");
            let r = extract(&c);
            assert!(
                matches!(r[0], Wikilink::Image(_)),
                "ext={ext} (upper) should be Image"
            );
        }
    }

    #[test]
    fn non_image_non_md_extensions_classified_as_file() {
        for ext in ["pdf", "mp4", "mov", "zip", "exe", "bin", "sh", "tar"] {
            let c = format!("![[file.{ext}]]");
            let r = extract(&c);
            assert!(
                matches!(r[0], Wikilink::File(_)),
                "ext={ext} should be File"
            );
        }
    }

    // ----- multiple + order -----

    #[test]
    fn extracts_multiple_in_order() {
        let c = "veja [[A|a]] e ![[img.png]] e ![[doc.pdf]] e [[B#Sec]]";
        let r = extract(c);
        assert_eq!(r.len(), 4);
        assert!(
            matches!(&r[0], Wikilink::Note(n) if n.target == "A" && n.alias.as_deref() == Some("a"))
        );
        assert!(matches!(&r[1], Wikilink::Image(e) if e.path == "img.png"));
        assert!(matches!(&r[2], Wikilink::File(e) if e.path == "doc.pdf"));
        assert!(
            matches!(&r[3], Wikilink::Note(n) if n.target == "B" && matches!(n.anchor, Some(Anchor::Heading(ref h)) if h == "Sec"))
        );
    }

    #[test]
    fn position_order_preserved() {
        let c = "[[A]]xxx[[B]]yyy[[C]]";
        let r = extract(c);
        assert_eq!(r.len(), 3);
        assert!(r.iter().all(|w| matches!(w, Wikilink::Note(_))));
    }

    // ----- edge cases / hardening -----

    #[test]
    fn ignores_unclosed_brackets() {
        let c = "[[unclosed and another [[Title]] end";
        let r = extract(c);
        assert!(r.len() <= 2, "should not crash, got {} links", r.len());
    }

    #[test]
    fn ignores_empty_brackets() {
        let c = "empty [[]] and ![[]]";
        assert_eq!(extract(c), vec![]);
    }

    #[test]
    fn trims_whitespace_in_inner() {
        let c = "[[  Spaced Title  ]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Spaced Title");
        } else {
            panic!();
        }
    }

    #[test]
    fn empty_alias_treated_as_none() {
        let c = "[[Note|]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert!(n.alias.is_none());
        } else {
            panic!();
        }
    }

    #[test]
    fn empty_anchor_treated_as_none() {
        let c = "[[Note#]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert!(n.anchor.is_none());
        } else {
            panic!();
        }
    }

    #[test]
    fn empty_block_id_treated_as_none() {
        let c = "[[Note#^]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert!(n.anchor.is_none());
        } else {
            panic!();
        }
    }

    #[test]
    fn anchor_with_nested_hashes() {
        // Obsidian supports `Note#H1#H2#H3` for sub-headings.
        let c = "[[Note#H1#H2#H3]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Note");
            assert_eq!(n.anchor, Some(Anchor::Heading("H1#H2#H3".to_string())));
        } else {
            panic!();
        }
    }

    #[test]
    fn unicode_titles_extracted() {
        let c = "[[日本語]] [[português áéíóú]] [[🔥 emoji]]";
        let r = extract(c);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn traversal_string_extracted_as_literal() {
        let c = "[[../../etc/passwd]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "../../etc/passwd");
        } else {
            panic!();
        }
    }

    #[test]
    fn scheme_string_extracted_as_literal() {
        let c = "[[file://path]] [[scheme:payload]]";
        let r = extract(c);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn very_long_inner_does_not_panic() {
        let inner = "x".repeat(10_000);
        let c = format!("[[{inner}]]");
        let r = extract(&c);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn many_open_brackets_does_not_panic() {
        let c = "[[".repeat(50);
        let _ = extract(&c);
    }

    #[test]
    fn many_close_brackets_does_not_panic() {
        let c = "]]".repeat(50);
        let _ = extract(&c);
    }

    #[test]
    fn null_byte_inner_preserved() {
        let c = "[[null\0byte]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "null\0byte");
        } else {
            panic!();
        }
    }

    #[test]
    fn multiline_inner_extracted_trimmed() {
        let c = "[[\n  Multi\nLine  \n]]";
        let r = extract(c);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn backslash_does_not_escape_brackets() {
        // No escape syntax — `\[[Title]]` still extracts.
        let c = r"\[[Title]]";
        let r = extract(c);
        if let Wikilink::Note(n) = &r[0] {
            assert_eq!(n.target, "Title");
        } else {
            panic!();
        }
    }

    #[test]
    fn standalone_link_not_embed() {
        let c = "no bang prefix [[Note]]";
        let r = extract(c);
        assert!(matches!(&r[0], Wikilink::Note(_)));
    }

    // ----- code fence / inline code skipping (CAD-20 smoke regression fix) -----

    #[test]
    fn ignores_wikilink_inside_fenced_code() {
        // TOML `[[package]]` header inside a Cargo.toml diff must NOT extract.
        let c = "before [[Real]]\n```toml\n[[package]]\nname = \"foo\"\n```\nafter [[AlsoReal]]";
        let r = extract(c);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].target_str(), "Real");
        assert_eq!(r[1].target_str(), "AlsoReal");
    }

    #[test]
    fn ignores_wikilink_inside_inline_code() {
        let c = "use `[[NotALink]]` not `[[OtherFake]]` but [[Real]]";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].target_str(), "Real");
    }

    #[test]
    fn nested_fences_toggle_correctly() {
        let c = "[[A]]\n```\n[[FakeA]]\n```\n[[B]]\n```rust\n[[FakeB]]\n```\n[[C]]";
        let r = extract(c);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].target_str(), "A");
        assert_eq!(r[1].target_str(), "B");
        assert_eq!(r[2].target_str(), "C");
    }

    #[test]
    fn inline_code_does_not_cross_newline() {
        // Unclosed backtick on a line shouldn't suppress wikilinks on next line.
        let c = "this `is unclosed\n[[Real]]";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].target_str(), "Real");
    }

    #[test]
    fn indented_fence_suppresses_wikilink() {
        // CommonMark allows fenced code blocks indented up to 3 spaces.
        // Our parser is CommonMark-compatible: whitespace before ``` still triggers fence.
        let c = "  ```\n[[InsideFence]]\n  ```\n[[OutsideFence]]";
        let r = extract(c);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].target_str(), "OutsideFence");
    }

    // ----- target_str / is_note helpers -----

    #[test]
    fn target_str_works_for_all_variants() {
        let r = extract("[[A]] ![[b.png]] ![[c.pdf]] ![[D]]");
        assert_eq!(r[0].target_str(), "A");
        assert_eq!(r[1].target_str(), "b.png");
        assert_eq!(r[2].target_str(), "c.pdf");
        assert_eq!(r[3].target_str(), "D");
    }

    #[test]
    fn is_note_classifies_correctly() {
        let r = extract("[[A]] ![[b.png]] ![[c.pdf]] ![[D]]");
        assert!(r[0].is_note()); // [[A]]
        assert!(!r[1].is_note()); // image
        assert!(!r[2].is_note()); // file
        assert!(r[3].is_note()); // ![[D]] = NoteEmbed
    }

    // ----- inline tag extraction -----

    #[test]
    fn extracts_inline_tag_simple() {
        let c = "este é um #rust note";
        assert_eq!(extract_inline_tags(c), vec!["rust"]);
    }

    #[test]
    fn extracts_nested_tag() {
        let c = "trabalho em #projeto/cfo hoje";
        assert_eq!(extract_inline_tags(c), vec!["projeto/cfo"]);
    }

    #[test]
    fn extracts_multiple_tags() {
        let c = "#rust e #obsidian e #productivity";
        assert_eq!(
            extract_inline_tags(c),
            vec!["rust", "obsidian", "productivity"]
        );
    }

    #[test]
    fn ignores_heading_line() {
        let c = "# Título\n#real-tag aqui";
        assert_eq!(extract_inline_tags(c), vec!["real-tag"]);
    }

    #[test]
    fn ignores_block_ref_in_tag_parser() {
        let c = "veja [[Note#^abc]] aqui #real";
        assert_eq!(extract_inline_tags(c), vec!["real"]);
    }

    #[test]
    fn ignores_heading_anchor_in_wikilink() {
        let c = "veja [[Note#Heading]] #real";
        assert_eq!(extract_inline_tags(c), vec!["real"]);
    }

    #[test]
    fn ignores_hash_in_code_fence() {
        let c = "antes #before\n```\n#not-a-tag\n```\n#after";
        assert_eq!(extract_inline_tags(c), vec!["before", "after"]);
    }

    #[test]
    fn section_under_heading_extrai_ate_proximo_heading() {
        let md = "# Topo\nintro\n## Sprint\nlinha A\nlinha B\n## Outro\nlinha C\n";
        let s = section_under_heading(md, "Sprint").unwrap();
        assert_eq!(s, "linha A\nlinha B");
    }

    #[test]
    fn section_under_heading_case_insensitive_e_nested() {
        // Heading aninhado (### dentro da seção ##) faz parte da seção, mas um
        // ## (mesma profundidade) encerra.
        let md = "## Alvo\ntexto\n### Sub\nmais\n## Fim\nfora\n";
        let s = section_under_heading(md, "alvo").unwrap();
        assert!(s.contains("texto"));
        assert!(s.contains("### Sub"));
        assert!(s.contains("mais"));
        assert!(!s.contains("fora"));
    }

    #[test]
    fn section_under_heading_ausente_retorna_none() {
        assert!(section_under_heading("# A\nx\n", "Inexistente").is_none());
    }

    #[test]
    fn section_under_heading_ignora_hash_em_code_fence() {
        let md = "## Alvo\nantes\n```\n## nao e heading\n```\ndepois\n## Fim\nfora\n";
        let s = section_under_heading(md, "Alvo").unwrap();
        assert!(s.contains("antes"));
        assert!(s.contains("## nao e heading")); // dentro do fence, faz parte
        assert!(s.contains("depois"));
        assert!(!s.contains("fora"));
    }

    #[test]
    fn extract_spans_recorta_o_match_completo() {
        let c = "vê [[Nota A]] e ![[img.png]] aqui";
        let spans = extract_spans(c);
        assert_eq!(spans.len(), 2);
        // O span do 1º deve recortar exatamente "[[Nota A]]".
        assert_eq!(&c[spans[0].1.clone()], "[[Nota A]]");
        assert!(matches!(spans[0].0, Wikilink::Note(_)));
        // O 2º recorta "![[img.png]]".
        assert_eq!(&c[spans[1].1.clone()], "![[img.png]]");
        assert!(matches!(spans[1].0, Wikilink::Image(_)));
    }

    #[test]
    fn extract_spans_pula_code_e_casa_extract() {
        let c = "`[[no code]]` mas [[real]] sim";
        let spans = extract_spans(c);
        assert_eq!(spans.len(), 1);
        assert_eq!(&c[spans[0].1.clone()], "[[real]]");
        // Mesma contagem que o extract (consistência entre as duas fns).
        assert_eq!(spans.len(), extract(c).len());
    }

    #[test]
    fn ignores_hash_in_middle_of_word() {
        let c = "issue#123 e color #FF0000";
        // issue#123 has `#` after letter → not a tag. #FF0000 starts after space → IS a tag.
        let r = extract_inline_tags(c);
        assert!(!r.iter().any(|t| t == "123"));
        assert!(r.contains(&"FF0000".to_string()));
    }

    #[test]
    fn tag_after_punctuation_extracted() {
        let c = "(parens #tag1), [bracket #tag2]";
        let r = extract_inline_tags(c);
        assert!(r.contains(&"tag1".to_string()));
        assert!(r.contains(&"tag2".to_string()));
    }

    // ----- property-based fuzz -----

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 256, ..proptest::test_runner::Config::default() })]

        #[test]
        fn extract_never_panics_on_arbitrary_strings(s in proptest::prelude::any::<String>()) {
            let _ = extract(&s);
        }

        #[test]
        fn extract_never_panics_on_bracket_soup(s in r"[\[\]a-zA-Z0-9 !|#\^]{0,200}") {
            let _ = extract(&s);
        }

        #[test]
        fn extract_inline_tags_never_panics(s in proptest::prelude::any::<String>()) {
            let _ = extract_inline_tags(&s);
        }

        #[test]
        fn extract_inline_tags_never_panics_on_tag_soup(s in r"[#a-zA-Z0-9_/\- \n`\[\]]{0,200}") {
            let _ = extract_inline_tags(&s);
        }
    }
}
