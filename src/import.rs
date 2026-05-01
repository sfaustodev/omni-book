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
