use lopdf::Document;
use std::path::Path;

pub fn extract_text(pdf_path: &Path) -> Result<String, String> {
    let doc = Document::load(pdf_path).map_err(|e| e.to_string())?;
    let mut out = String::new();
    let pages = doc.get_pages();
    for page_num in pages.keys() {
        if let Ok(text) = doc.extract_text(&[*page_num]) {
            out.push_str(&format!("\n## Página {}\n\n", page_num));
            out.push_str(&text);
            out.push('\n');
        }
    }
    Ok(out)
}
