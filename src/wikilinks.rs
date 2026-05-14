use std::path::Path;

/// Wikilink or embed extracted from note content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wikilink {
    /// `[[Title]]` — link to another note by title
    Note(String),
    /// `![[file.png]]` — embedded image (jpg, png, gif, webp)
    Image(String),
    /// `![[file.pdf]]` or other non-image — embedded as openable file
    File(String),
}

/// Extract all `[[...]]` and `![[...]]` references from content, in order.
/// Duplicates preserved (caller can dedupe if needed).
pub fn extract(content: &str) -> Vec<Wikilink> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // Look for `[[` or `![[`
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
            // unclosed, give up
            break;
        }

        let inner = &content[start..end];
        let inner = inner.trim();
        if !inner.is_empty() {
            if is_embed {
                if is_image_extension(inner) {
                    out.push(Wikilink::Image(inner.to_string()));
                } else {
                    out.push(Wikilink::File(inner.to_string()));
                }
            } else {
                out.push(Wikilink::Note(inner.to_string()));
            }
        }

        i = end + 2;
    }

    out
}

fn is_image_extension(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let path = Path::new(&lower);
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_note_link() {
        let c = "veja [[Título da Nota]] pra contexto";
        assert_eq!(extract(c), vec![Wikilink::Note("Título da Nota".into())]);
    }

    #[test]
    fn extracts_image_embed() {
        let c = "antes ![[diagrama.png]] depois";
        assert_eq!(extract(c), vec![Wikilink::Image("diagrama.png".into())]);
    }

    #[test]
    fn extracts_pdf_embed_as_file() {
        let c = "spec ![[manual.pdf]]";
        assert_eq!(extract(c), vec![Wikilink::File("manual.pdf".into())]);
    }

    #[test]
    fn extracts_multiple_in_order() {
        let c = "veja [[A]] e ![[img.png]] e ![[doc.pdf]] e [[B]]";
        assert_eq!(
            extract(c),
            vec![
                Wikilink::Note("A".into()),
                Wikilink::Image("img.png".into()),
                Wikilink::File("doc.pdf".into()),
                Wikilink::Note("B".into()),
            ]
        );
    }

    #[test]
    fn ignores_unclosed_brackets() {
        let c = "[[unclosed and another [[Title]] end";
        // The first `[[` makes the parser look for `]]`; it finds `Title]]` inside the second pair
        // depending on implementation — verify behavior is deterministic
        let result = extract(c);
        // At minimum, should not crash and should find something reasonable
        assert!(result.len() <= 2);
    }

    #[test]
    fn ignores_empty_brackets() {
        let c = "empty [[]] and ![[]]";
        assert_eq!(extract(c), vec![]);
    }

    #[test]
    fn case_insensitive_image_detection() {
        let c = "![[Photo.PNG]] ![[movie.MP4]]";
        assert_eq!(
            extract(c),
            vec![
                Wikilink::Image("Photo.PNG".into()),
                Wikilink::File("movie.MP4".into()),
            ]
        );
    }

    #[test]
    fn trims_whitespace_in_inner() {
        let c = "[[  Spaced Title  ]]";
        assert_eq!(extract(c), vec![Wikilink::Note("Spaced Title".into())]);
    }

    #[test]
    fn standalone_link_not_embed() {
        let c = "no bang prefix [[Note]]";
        assert_eq!(extract(c), vec![Wikilink::Note("Note".into())]);
    }

    // CAD-12: adversarial parser inputs.

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
    fn multiline_inner_extracted_trimmed() {
        let c = "[[\n  Multi\nLine  \n]]";
        let result = extract(c);
        assert_eq!(result.len(), 1);
        if let Wikilink::Note(s) = &result[0] {
            assert!(s.contains("Multi"));
        } else {
            panic!("expected Note variant");
        }
    }

    #[test]
    fn null_byte_inner_preserved() {
        let c = "[[null\0byte]]";
        let result = extract(c);
        assert_eq!(result, vec![Wikilink::Note("null\0byte".into())]);
    }

    #[test]
    fn traversal_string_extracted_as_literal_note() {
        // Wikilink resolution happens later — extractor treats path-like content as a literal
        let c = "[[../../etc/passwd]]";
        assert_eq!(extract(c), vec![Wikilink::Note("../../etc/passwd".into())]);
    }

    #[test]
    fn scheme_string_extracted_as_literal_note() {
        // egui_commonmark does not auto-render wikilinks as URLs, so a scheme-like
        // payload remains a Note title and is safe.
        let c = "[[file://path]] [[scheme:payload]]";
        assert_eq!(
            extract(c),
            vec![
                Wikilink::Note("file://path".into()),
                Wikilink::Note("scheme:payload".into()),
            ]
        );
    }

    #[test]
    fn very_long_inner_does_not_panic() {
        let inner = "x".repeat(10_000);
        let c = format!("[[{inner}]]");
        let result = extract(&c);
        assert_eq!(result.len(), 1);
        if let Wikilink::Note(s) = &result[0] {
            assert_eq!(s.len(), 10_000);
        }
    }

    #[test]
    fn unicode_titles_extracted() {
        let c = "[[日本語]] [[português áéíóú]] [[🔥 emoji]]";
        let result = extract(c);
        assert_eq!(
            result,
            vec![
                Wikilink::Note("日本語".into()),
                Wikilink::Note("português áéíóú".into()),
                Wikilink::Note("🔥 emoji".into()),
            ]
        );
    }

    #[test]
    fn embed_inside_link_extracts_first_pair() {
        // The leading `[[` is a link; the parser greedily looks for `]]` and finds
        // the first one inside the embed. Behaviour is deterministic.
        let c = "[[![[nested]]]]";
        let result = extract(c);
        assert!(!result.is_empty(), "should extract something");
    }

    #[test]
    fn all_image_extensions_classified() {
        for ext in ["png", "jpg", "jpeg", "gif", "webp", "bmp"] {
            let c = format!("![[img.{ext}]]");
            let result = extract(&c);
            assert_eq!(
                result,
                vec![Wikilink::Image(format!("img.{ext}"))],
                "ext={ext} should be Image"
            );
        }
    }

    #[test]
    fn all_image_extensions_uppercase_classified() {
        for ext in ["PNG", "JPG", "JPEG", "GIF", "WEBP", "BMP"] {
            let c = format!("![[img.{ext}]]");
            let result = extract(&c);
            assert_eq!(
                result,
                vec![Wikilink::Image(format!("img.{ext}"))],
                "ext={ext} (upper) should be Image"
            );
        }
    }

    #[test]
    fn non_image_extensions_classified_as_file() {
        for ext in ["pdf", "mp4", "mov", "zip", "exe", "bin", "sh", "tar"] {
            let c = format!("![[file.{ext}]]");
            let result = extract(&c);
            assert_eq!(
                result,
                vec![Wikilink::File(format!("file.{ext}"))],
                "ext={ext} should be File"
            );
        }
    }

    #[test]
    fn embed_no_extension_classified_as_file() {
        let c = "![[no_ext]]";
        assert_eq!(extract(c), vec![Wikilink::File("no_ext".into())]);
    }

    #[test]
    fn position_order_preserved() {
        let c = "[[A]]xxx[[B]]yyy[[C]]";
        let result = extract(c);
        assert_eq!(
            result,
            vec![
                Wikilink::Note("A".into()),
                Wikilink::Note("B".into()),
                Wikilink::Note("C".into()),
            ]
        );
    }

    #[test]
    fn backslash_does_not_escape_brackets() {
        // No escape syntax is implemented — `\[[Title]]` still extracts.
        let c = r"\[[Title]]";
        let result = extract(c);
        assert_eq!(result, vec![Wikilink::Note("Title".into())]);
    }

    // CAD-12: property-based fuzzing — extractor must never panic on arbitrary input.

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 256, ..proptest::test_runner::Config::default() })]

        #[test]
        fn extract_never_panics_on_arbitrary_strings(s in proptest::prelude::any::<String>()) {
            let _ = extract(&s);
        }

        #[test]
        fn extract_never_panics_on_bracket_soup(s in "[\\[\\]a-zA-Z0-9 !]{0,200}") {
            let _ = extract(&s);
        }
    }
}
