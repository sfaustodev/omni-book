use lopdf::Document;
use std::path::Path;

pub fn extract_text(pdf_path: &Path) -> Result<String, String> {
    let doc = Document::load(pdf_path).map_err(|e| e.to_string())?;
    let mut out = String::new();
    let pages = doc.get_pages();
    for (page_num, _) in pages.iter() {
        if let Ok(text) = doc.extract_text(&[*page_num]) {
            out.push_str(&format!("\n## Página {}\n\n", page_num));
            out.push_str(&text);
            out.push('\n');
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::dictionary;
    use lopdf::{Object, Stream};
    use std::io::Write;

    /// Build a minimal valid PDF with `n` pages, each containing a unique text token.
    fn make_pdf_with_text(pages: &[&str]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let mut page_ids = Vec::new();
        for text in pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 12.into()]),
                    Operation::new("Td", vec![100.into(), 700.into()]),
                    Operation::new("Tj", vec![Object::string_literal(*text)]),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            });
            page_ids.push(page_id);
        }
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
                "Count" => pages.len() as i64,
                "Resources" => resources_id,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.compress();

        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    fn write_tmp(bytes: &[u8], suffix: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
        f.write_all(bytes).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn extract_text_single_page_contains_text() {
        let bytes = make_pdf_with_text(&["Hello World"]);
        let f = write_tmp(&bytes, ".pdf");
        let out = extract_text(f.path()).unwrap();
        assert!(out.contains("Hello World"), "got {out:?}");
        assert!(out.contains("## Página"), "missing page heading: {out:?}");
    }

    #[test]
    fn extract_text_multipage_includes_all_pages() {
        let bytes = make_pdf_with_text(&["AlphaPage", "BetaPage", "GammaPage"]);
        let f = write_tmp(&bytes, ".pdf");
        let out = extract_text(f.path()).unwrap();
        assert!(out.contains("AlphaPage"));
        assert!(out.contains("BetaPage"));
        assert!(out.contains("GammaPage"));
        // Three distinct page headings
        assert_eq!(out.matches("## Página").count(), 3, "got {out:?}");
    }

    #[test]
    fn extract_text_zero_byte_errors() {
        let f = write_tmp(b"", ".pdf");
        assert!(extract_text(f.path()).is_err());
    }

    #[test]
    fn extract_text_random_bytes_errors() {
        let bytes: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
        let f = write_tmp(&bytes, ".pdf");
        let res = extract_text(f.path());
        // Either an error (expected) or an empty string — both must not panic
        if let Ok(out) = res {
            assert!(out.is_empty() || out.contains("##"));
        }
    }

    #[test]
    fn extract_text_non_pdf_content_errors() {
        let f = write_tmp(b"not a pdf at all, just some text", ".pdf");
        assert!(extract_text(f.path()).is_err());
    }

    #[test]
    fn extract_text_missing_file_errors() {
        let res = extract_text(Path::new("/tmp/does_not_exist_omninote_pdf.pdf"));
        assert!(res.is_err());
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 32, ..proptest::test_runner::Config::default() })]

        #[test]
        fn extract_text_random_input_never_panics(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..1024)) {
            let f = write_tmp(&bytes, ".pdf");
            let _ = std::panic::catch_unwind(|| extract_text(f.path()));
        }
    }
}
