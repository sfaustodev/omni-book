use omninote_core::wikilinks::*;

#[test]
fn section_under_heading_keeps_unclosed_backtick_fence_until_eof() {
    let content = "## Alvo\nantes\n```\n# nao fecha\n[[NaoLink]]";

    let section = section_under_heading(content, "Alvo").unwrap();

    assert_eq!(section, "antes\n```\n# nao fecha\n[[NaoLink]]");
}

#[test]
fn section_under_heading_does_not_close_backtick_fence_with_tilde_marker() {
    let content = "## Alvo\nantes\n```\n~~~\n## falso limite\n```\ndepois\n## Fim\nfora";

    let section = section_under_heading(content, "Alvo").unwrap();

    assert!(section.contains("~~~"));
    assert!(section.contains("## falso limite"));
    assert!(section.contains("depois"));
    assert!(!section.contains("fora"));
}

#[test]
fn section_under_heading_rejects_seven_hash_atx_lines() {
    let content = "####### Alvo\nnao entra\n## Real\nbody";

    assert!(section_under_heading(content, "Alvo").is_none());
    assert_eq!(section_under_heading(content, "Real").unwrap(), "body");
}

#[test]
fn section_under_heading_does_not_treat_seven_hash_line_as_boundary() {
    let content = "## Alvo\nantes\n####### nao limite\ncontinua\n## Fim\nfora";

    let section = section_under_heading(content, "Alvo").unwrap();

    assert_eq!(section, "antes\n####### nao limite\ncontinua");
}

#[test]
fn section_under_heading_requires_space_after_hashes() {
    let content = "##Alvo\nnao entra\n## Real\nok";

    assert!(section_under_heading(content, "Alvo").is_none());
    assert_eq!(section_under_heading(content, "Real").unwrap(), "ok");
}

#[test]
fn section_under_heading_returns_empty_body_for_heading_at_eof() {
    assert_eq!(section_under_heading("# Alvo", "Alvo").unwrap(), "");
}

#[test]
fn section_under_heading_duplicate_heading_uses_first_match() {
    let content = "## Alvo\nprimeiro\n## Alvo\nsegundo";

    let section = section_under_heading(content, "Alvo").unwrap();

    assert_eq!(section, "primeiro");
}

#[test]
fn section_under_heading_handles_cjk_and_emoji_body() {
    let content = "# 章節\n東京のメモ\n下一行🙂\n# 次\n外";

    let section = section_under_heading(content, "章節").unwrap();

    assert_eq!(section, "東京のメモ\n下一行🙂");
}

#[test]
fn section_under_heading_empty_input_returns_none() {
    assert!(section_under_heading("", "Alvo").is_none());
}

#[test]
fn section_under_heading_large_input_stays_linear_enough_for_tests() {
    let repeated = "### 子\nlinha com texto e [[Link]]\n".repeat(20_000);
    let content = format!("## Alvo\n{repeated}## Fim\nfora");

    let section = section_under_heading(&content, "Alvo").unwrap();

    assert!(section.starts_with("### 子\nlinha com texto"));
    assert!(section.ends_with("linha com texto e [[Link]]"));
    assert_eq!(section.matches("### 子").count(), 20_000);
    assert!(!section.contains("fora"));
}

#[test]
fn extract_spans_handles_adjacent_links_without_merging() {
    let content = "pre[[A]][[B]]post";

    let spans = extract_spans(content);

    assert_eq!(spans.len(), 2);
    assert_eq!(&content[spans[0].1.clone()], "[[A]]");
    assert_eq!(&content[spans[1].1.clone()], "[[B]]");
    assert_eq!(spans[0].1.end, spans[1].1.start);
    assert!(matches!(&spans[0].0, Wikilink::Note(n) if n.target == "A"));
    assert!(matches!(&spans[1].0, Wikilink::Note(n) if n.target == "B"));
}

#[test]
fn extract_spans_skips_inline_code_links() {
    let content = "antes `[[code]]` depois [[real]]";

    let spans = extract_spans(content);

    assert_eq!(spans.len(), 1);
    assert_eq!(&content[spans[0].1.clone()], "[[real]]");
    assert!(matches!(&spans[0].0, Wikilink::Note(n) if n.target == "real"));
}

#[test]
fn extract_spans_ignores_unclosed_embed_at_eof() {
    let content = "antes ![[img.png";

    assert!(extract_spans(content).is_empty());
}

#[test]
fn extract_spans_returns_utf8_safe_byte_ranges() {
    let content = "你看[[東京]]🙂![[图.png|图]] fim";

    let spans = extract_spans(content);

    assert_eq!(spans.len(), 2);
    assert_eq!(&content[spans[0].1.clone()], "[[東京]]");
    assert_eq!(&content[spans[1].1.clone()], "![[图.png|图]]");
    assert!(matches!(&spans[0].0, Wikilink::Note(n) if n.target == "東京"));
    assert!(
        matches!(&spans[1].0, Wikilink::Image(e) if e.path == "图.png" && e.alias.as_deref() == Some("图"))
    );
}
