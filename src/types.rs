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

#[derive(Clone, Debug)]
pub struct Note {
    pub path: PathBuf,
    pub rel_path: PathBuf,
    pub frontmatter: Frontmatter,
    pub title: String,
    pub content: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FontFamily {
    System,
    Monospace,
    Serif,
}

impl Default for FontFamily {
    fn default() -> Self { Self::System }
}

impl FontFamily {
    pub fn label(&self) -> &'static str {
        match self {
            Self::System => "Sistema (sans-serif)",
            Self::Monospace => "Monospace",
            Self::Serif => "Serif",
        }
    }
    pub fn all() -> [FontFamily; 3] {
        [Self::System, Self::Monospace, Self::Serif]
    }
    pub fn as_egui_family(&self) -> egui::FontFamily {
        match self {
            Self::System | Self::Serif => egui::FontFamily::Proportional,
            Self::Monospace => egui::FontFamily::Monospace,
        }
    }
}

fn default_font_size() -> f32 { 14.0 }
fn default_line_height() -> f32 { 1.4 }
fn default_letter_spacing() -> f32 { 0.0 }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)] pub dark_mode: bool,
    #[serde(default)] pub last_active: Option<String>,
    #[serde(default)] pub recent_vaults: Vec<PathBuf>,
    #[serde(default)] pub font_family: FontFamily,
    #[serde(default = "default_font_size")] pub font_size: f32,
    #[serde(default = "default_line_height")] pub line_height: f32,
    #[serde(default = "default_letter_spacing")] pub letter_spacing: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dark_mode: false,
            last_active: None,
            recent_vaults: Vec::new(),
            font_family: FontFamily::default(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            letter_spacing: default_letter_spacing(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeleteNote(String),
    DeleteFolder(PathBuf),
}
