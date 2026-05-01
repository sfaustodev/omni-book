use std::path::Path;
use std::fs;

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
    let messages = v.get("chat_messages").or_else(|| v.get("messages"))
        .and_then(|m| m.as_array())
        .ok_or("formato de chat não reconhecido")?;
    for msg in messages {
        let role = msg.get("sender").or_else(|| msg.get("role"))
            .and_then(|r| r.as_str()).unwrap_or("?");
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
                .collect::<Vec<_>>().join("\n")
        } else {
            msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string()
        };
        out.push_str(&format!("{}\n\n{}\n\n---\n\n", label, content));
    }
    Ok(out)
}

pub fn import_claude_artifact(src: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(src).map_err(|e| e.to_string())?;
    let ext = src.extension().and_then(|s| s.to_str()).unwrap_or("txt");
    let lang = match ext {
        "tsx" | "jsx" => "tsx", "ts" => "typescript", "js" => "javascript",
        "py" => "python", "rs" => "rust", "html" => "html",
        "md" => return Ok(raw),
        _ => ext,
    };
    let title = src.file_stem().and_then(|s| s.to_str()).unwrap_or("artefato");
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
}
