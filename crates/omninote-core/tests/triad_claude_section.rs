//! Adversarial coverage for the Slice 3 markdown parsers (triad gate, Claude —
//! Codex's run didn't produce a file). Targets section_under_heading + extract_spans.

use omninote_core::wikilinks::{extract_spans, section_under_heading, Wikilink};

#[test]
fn section_unclosed_fence_does_not_panic_and_includes_rest() {
    // A fence opened but never closed: everything after stays "in fence" but the
    // function must not panic and must return the collected lines.
    let md = "## Alvo\nantes\n```\nsem fechar\nmais codigo\n";
    let s = section_under_heading(md, "Alvo").unwrap();
    assert!(s.contains("antes"));
    assert!(s.contains("sem fechar"));
}

#[test]
fn section_heading_7plus_hashes_is_not_a_heading() {
    // ATX caps at 6 `#`; `#######` is not a heading boundary.
    let md = "## Alvo\ntexto\n####### nao conta\nfim\n";
    let s = section_under_heading(md, "Alvo").unwrap();
    assert!(s.contains("####### nao conta"));
    assert!(s.contains("fim"));
}

#[test]
fn section_heading_at_eof_no_body() {
    let md = "# Topo\nx\n## Vazio\n";
    let s = section_under_heading(md, "Vazio").unwrap();
    assert_eq!(s, "");
}

#[test]
fn section_duplicate_heading_takes_first() {
    let md = "## Dup\nprimeiro\n## Outro\nmeio\n## Dup\nsegundo\n";
    let s = section_under_heading(md, "Dup").unwrap();
    assert_eq!(s, "primeiro");
}

#[test]
fn section_multibyte_heading_and_body_no_panic() {
    let md = "## Café ☕\nlinha com açaí e 日本語\n## Fim\nfora\n";
    let s = section_under_heading(md, "café ☕").unwrap();
    assert!(s.contains("açaí"));
    assert!(s.contains("日本語"));
    assert!(!s.contains("fora"));
}

#[test]
fn section_huge_input_is_linear() {
    // 50k lines — must complete quickly (linear scan, no quadratic blowup).
    let mut md = String::from("## Alvo\n");
    for i in 0..50_000 {
        md.push_str(&format!("linha {i}\n"));
    }
    md.push_str("## Fim\nfora\n");
    let s = section_under_heading(&md, "Alvo").unwrap();
    assert!(s.starts_with("linha 0"));
    assert!(s.contains("linha 49999"));
    assert!(!s.contains("fora"));
}

#[test]
fn extract_spans_adjacent_links() {
    let c = "[[A]][[B]]";
    let spans = extract_spans(c);
    assert_eq!(spans.len(), 2);
    assert_eq!(&c[spans[0].1.clone()], "[[A]]");
    assert_eq!(&c[spans[1].1.clone()], "[[B]]");
}

#[test]
fn extract_spans_unclosed_embed_no_panic() {
    // Unclosed `![[` must not panic; it just yields no completed span.
    let spans = extract_spans("texto ![[sem fechar");
    assert!(
        spans.is_empty()
            || spans
                .iter()
                .all(|(_, r)| r.end <= "texto ![[sem fechar".len())
    );
}

#[test]
fn extract_spans_multibyte_offsets_are_valid() {
    // The returned ranges must slice on char boundaries even with leading
    // multibyte text — otherwise indexing `&content[range]` would panic.
    let c = "açaí 日本 [[Nota]] fim";
    let spans = extract_spans(c);
    assert_eq!(spans.len(), 1);
    // This slice would panic if the offset weren't a char boundary:
    assert_eq!(&c[spans[0].1.clone()], "[[Nota]]");
    assert!(matches!(spans[0].0, Wikilink::Note(_)));
}

#[test]
fn extract_spans_ignores_links_in_inline_code() {
    let c = "real [[Sim]] e `[[Nao]]`";
    let spans = extract_spans(c);
    assert_eq!(spans.len(), 1);
    assert_eq!(&c[spans[0].1.clone()], "[[Sim]]");
}
