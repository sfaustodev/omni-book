use std::fs;
use std::path::Path;

/// Importa um chat exportado do Claude.
/// Aceita .json (estrutura: {messages: [{role, content}]}) ou .md.
pub fn import_claude_chat(src: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(src).map_err(|e| e.to_string())?;
    if src.extension().and_then(|s| s.to_str()) == Some("json") {
        return parse_claude_json(&raw);
    }
    Ok(raw)
}

fn parse_claude_json(json: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = String::new();
    if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
        out.push_str(&format!("# {}\n\n", name));
    }
    let messages = v
        .get("chat_messages")
        .or_else(|| v.get("messages"))
        .and_then(|m| m.as_array())
        .ok_or("formato de chat não reconhecido")?;
    for msg in messages {
        let role = msg
            .get("sender")
            .or_else(|| msg.get("role"))
            .and_then(|r| r.as_str())
            .unwrap_or("?");
        let label = match role {
            "human" | "user" => "**Você:**",
            "assistant" => "**Claude:**",
            _ => "**?:**",
        };
        let content = if let Some(s) = msg.get("text").and_then(|t| t.as_str()) {
            s.to_string()
        } else if let Some(arr) = msg.get("content").and_then(|c| c.as_array()) {
            arr.iter()
                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            msg.get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        };
        out.push_str(&format!("{}\n\n{}\n\n---\n\n", label, content));
    }
    Ok(out)
}

pub fn import_claude_artifact(src: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(src).map_err(|e| e.to_string())?;
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("txt");
    let lang = match ext {
        "tsx" | "jsx" => "tsx",
        "ts" => "typescript",
        "js" => "javascript",
        "py" => "python",
        "rs" => "rust",
        "html" => "html",
        "md" => return Ok(raw),
        _ => ext,
    };
    let title = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("artefato");
    Ok(format!("# {}\n\n```{}\n{}\n```\n", title, lang, raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(content: &str, ext: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(ext).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn claude_chat_json_produces_markdown() {
        let json = r#"{"name":"Conversa","chat_messages":[{"sender":"human","text":"Olá"},{"sender":"assistant","text":"Oi!"}]}"#;
        let f = tmp_file(json, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(out.contains("# Conversa"));
        assert!(out.contains("**Você:**"));
        assert!(out.contains("**Claude:**"));
        assert!(out.contains("Olá"));
    }

    #[test]
    fn claude_chat_missing_messages_errors() {
        let f = tmp_file(r#"{"name":"Vazio"}"#, ".json");
        assert!(import_claude_chat(f.path()).is_err());
    }

    #[test]
    fn md_file_passes_through() {
        let md = "# Título\n\nConteúdo.";
        let f = tmp_file(md, ".md");
        assert_eq!(import_claude_chat(f.path()).unwrap(), md);
    }

    #[test]
    fn artifact_rust_wraps_in_fenced_block() {
        let code = "fn main() {}";
        let f = tmp_file(code, ".rs");
        let out = import_claude_artifact(f.path()).unwrap();
        assert!(out.contains("```rust"));
        assert!(out.contains("fn main()"));
    }

    #[test]
    fn artifact_md_passes_through() {
        let md = "# Art\n\nConteúdo.";
        let f = tmp_file(md, ".md");
        assert_eq!(import_claude_artifact(f.path()).unwrap(), md);
    }

    // CAD-12: adversarial JSON + multimodal + extension matrix.

    #[test]
    fn malformed_json_errors_without_panic() {
        for raw in ["{", "}", "[[", "{\"a\":}", "\0\0\0", "not json at all"] {
            let f = tmp_file(raw, ".json");
            let res = import_claude_chat(f.path());
            assert!(res.is_err(), "expected err for {raw:?}");
        }
    }

    #[test]
    fn deeply_nested_json_does_not_panic() {
        // serde_json has internal recursion limits; ensure no stack overflow / panic
        let depth = 200;
        let mut raw = String::new();
        raw.push_str(&"[".repeat(depth));
        raw.push('1');
        raw.push_str(&"]".repeat(depth));
        let f = tmp_file(&raw, ".json");
        let _ = import_claude_chat(f.path());
    }

    #[test]
    fn json_missing_messages_errors() {
        let f = tmp_file(r#"{"name":"X"}"#, ".json");
        let res = import_claude_chat(f.path());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("formato de chat não reconhecido"));
    }

    #[test]
    fn json_unknown_role_label_falls_back() {
        let raw = r#"{"chat_messages":[{"sender":"alien","text":"hi"}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(out.contains("**?:**"));
        assert!(out.contains("hi"));
    }

    #[test]
    fn json_multimodal_content_array_concats_text_only() {
        let raw = r#"{"chat_messages":[{"sender":"assistant","content":[{"text":"a"},{"image":"x"},{"text":"b"}]}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(out.contains("a\nb"), "got {out:?}");
    }

    #[test]
    fn json_content_string_form_used() {
        let raw = r#"{"messages":[{"role":"user","content":"plain string"}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(out.contains("plain string"));
        assert!(out.contains("**Você:**"));
    }

    #[test]
    fn json_content_missing_yields_empty_block() {
        let raw = r#"{"chat_messages":[{"sender":"human"}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        // Block exists with **Você:** label even with no content
        assert!(out.contains("**Você:**"));
    }

    #[test]
    fn json_no_name_omits_h1_heading() {
        let raw = r#"{"chat_messages":[{"sender":"human","text":"oi"}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(!out.starts_with("# "), "got {out:?}");
    }

    #[test]
    fn json_text_with_separator_collision_passes_through() {
        // The output uses `\\n---\\n\\n` between blocks. If user text contains `---` itself,
        // current impl does not escape — verify it just passes through (documented gap).
        let raw = r#"{"chat_messages":[{"sender":"human","text":"start\n---\nend"}]}"#;
        let f = tmp_file(raw, ".json");
        let out = import_claude_chat(f.path()).unwrap();
        assert!(out.contains("start"));
        assert!(out.contains("end"));
    }

    #[test]
    fn import_chat_missing_file_errors() {
        let res = import_claude_chat(std::path::Path::new("/tmp/does_not_exist_omninote_xyz.json"));
        assert!(res.is_err());
    }

    #[test]
    fn import_chat_zero_byte_file_errors() {
        let f = tmp_file("", ".json");
        let res = import_claude_chat(f.path());
        assert!(res.is_err());
    }

    #[test]
    fn artifact_extension_matrix_classification() {
        for (ext, expected_lang) in [
            ("tsx", "tsx"),
            ("jsx", "tsx"),
            ("ts", "typescript"),
            ("js", "javascript"),
            ("py", "python"),
            ("rs", "rust"),
            ("html", "html"),
        ] {
            let f = tmp_file("src", &format!(".{ext}"));
            let out = import_claude_artifact(f.path()).unwrap();
            let fence = format!("```{expected_lang}");
            assert!(
                out.contains(&fence),
                "ext={ext}: missing fence {fence:?} in {out:?}"
            );
        }
    }

    #[test]
    fn artifact_unknown_extension_uses_extension_as_lang() {
        let f = tmp_file("data", ".xyz");
        let out = import_claude_artifact(f.path()).unwrap();
        assert!(out.contains("```xyz"));
    }

    #[test]
    fn artifact_with_triple_backticks_in_source_passes_through() {
        // Documents collision gap — input with ``` is not escaped before being wrapped
        // in the output fence. Markdown viewer may or may not render correctly.
        let f = tmp_file("```inner```", ".rs");
        let out = import_claude_artifact(f.path()).unwrap();
        assert!(out.contains("```inner```"));
    }
}
