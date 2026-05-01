use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Resumo, Citacao, Codigo, Exercicio, Duvida, Definicao,
}

impl NoteType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Resumo => "Resumo", Self::Citacao => "Citação",
            Self::Codigo => "Código", Self::Exercicio => "Exercício",
            Self::Duvida => "Dúvida", Self::Definicao => "Definição",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Resumo => "📄", Self::Citacao => "💬", Self::Codigo => "💻",
            Self::Exercicio => "✏", Self::Duvida => "❓", Self::Definicao => "💡",
        }
    }
    pub fn all() -> [NoteType; 6] {
        [Self::Resumo, Self::Citacao, Self::Codigo, Self::Exercicio, Self::Duvida, Self::Definicao]
    }
}

impl Default for NoteType {
    fn default() -> Self { Self::Resumo }
}

/// Frontmatter YAML que vai no topo de cada .md
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    #[serde(rename = "type", default)] pub note_type: NoteType,
    #[serde(default)] pub tags: Vec<String>,
    #[serde(default)] pub source: String,
    #[serde(default)] pub source_link: String,
    #[serde(default)] pub linked_note: Option<String>,
    #[serde(default)] pub attachments: Vec<String>,
    #[serde(default)] pub created: String,
}

/// Nota em memória — caminho no filesystem é a "verdade"
#[derive(Clone, Debug)]
pub struct Note {
    pub path: PathBuf,           // caminho absoluto do .md
    pub rel_path: PathBuf,       // caminho relativo ao vault
    pub frontmatter: Frontmatter,
    pub title: String,           // do nome do arquivo (sem .md)
    pub content: String,         // markdown sem frontmatter
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)] pub dark_mode: bool,
    #[serde(default)] pub last_active: Option<String>,
    #[serde(default)] pub recent_vaults: Vec<PathBuf>,
}
