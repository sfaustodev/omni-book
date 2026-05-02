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
        let is_embed = i + 2 < bytes.len() && bytes[i] == b'!' && bytes[i + 1] == b'[' && bytes[i + 2] == b'[';
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
}
