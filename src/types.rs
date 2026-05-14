use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum NoteType {
    #[default]
    Resumo,
    Citacao,
    Codigo,
    Exercicio,
    Duvida,
    Definicao,
}

impl NoteType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Resumo => "Resumo",
            Self::Citacao => "Citação",
            Self::Codigo => "Código",
            Self::Exercicio => "Exercício",
            Self::Duvida => "Dúvida",
            Self::Definicao => "Definição",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Resumo => "📄",
            Self::Citacao => "💬",
            Self::Codigo => "💻",
            Self::Exercicio => "✏",
            Self::Duvida => "❓",
            Self::Definicao => "💡",
        }
    }
    pub fn all() -> [NoteType; 6] {
        [
            Self::Resumo,
            Self::Citacao,
            Self::Codigo,
            Self::Exercicio,
            Self::Duvida,
            Self::Definicao,
        ]
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    #[serde(rename = "type", default)]
    pub note_type: NoteType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_link: String,
    #[serde(default)]
    pub linked_note: Option<String>,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default)]
    pub created: String,
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
#[derive(Default)]
pub enum FontFamily {
    #[default]
    System,
    Monospace,
    Serif,
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

fn default_font_size() -> f32 {
    14.0
}
fn default_line_height() -> f32 {
    1.4
}
fn default_letter_spacing() -> f32 {
    0.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub dark_mode: bool,
    #[serde(default)]
    pub last_active: Option<String>,
    #[serde(default)]
    pub recent_vaults: Vec<PathBuf>,
    #[serde(default)]
    pub font_family: FontFamily,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default = "default_letter_spacing")]
    pub letter_spacing: f32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_type_yaml_round_trip_all_variants() {
        for nt in NoteType::all() {
            let yaml = serde_yaml::to_string(&nt).unwrap();
            let back: NoteType = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back, nt);
        }
    }

    #[test]
    fn note_type_label_and_icon_non_empty() {
        for nt in NoteType::all() {
            assert!(!nt.label().is_empty(), "label empty for {nt:?}");
            assert!(!nt.icon().is_empty(), "icon empty for {nt:?}");
        }
    }

    #[test]
    fn note_type_default_is_resumo() {
        assert_eq!(NoteType::default(), NoteType::Resumo);
    }

    #[test]
    fn font_family_yaml_round_trip_all_variants() {
        for ff in FontFamily::all() {
            let yaml = serde_yaml::to_string(&ff).unwrap();
            let back: FontFamily = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(back, ff);
        }
    }

    #[test]
    fn font_family_egui_mapping_consistent() {
        assert_eq!(
            FontFamily::System.as_egui_family(),
            egui::FontFamily::Proportional
        );
        assert_eq!(
            FontFamily::Serif.as_egui_family(),
            egui::FontFamily::Proportional
        );
        assert_eq!(
            FontFamily::Monospace.as_egui_family(),
            egui::FontFamily::Monospace
        );
    }

    #[test]
    fn font_family_labels_non_empty() {
        for ff in FontFamily::all() {
            assert!(!ff.label().is_empty(), "label empty for {ff:?}");
        }
    }

    #[test]
    fn app_config_default_values() {
        let c = AppConfig::default();
        assert!(!c.dark_mode);
        assert!(c.last_active.is_none());
        assert!(c.recent_vaults.is_empty());
        assert_eq!(c.font_family, FontFamily::System);
        assert!((c.font_size - 14.0).abs() < f32::EPSILON);
        assert!((c.line_height - 1.4).abs() < f32::EPSILON);
        assert!((c.letter_spacing - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn app_config_serde_missing_fields_use_defaults() {
        let c: AppConfig = serde_json::from_str("{}").unwrap();
        assert!(!c.dark_mode);
        assert_eq!(c.font_family, FontFamily::System);
        assert!((c.font_size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn app_config_serde_extra_fields_ignored() {
        let raw = r#"{"dark_mode":true,"font_size":20.0,"future_setting":"v9"}"#;
        let c: AppConfig = serde_json::from_str(raw).unwrap();
        assert!(c.dark_mode);
        assert!((c.font_size - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn frontmatter_default_has_empty_fields() {
        let fm = Frontmatter::default();
        assert!(fm.id.is_empty());
        assert_eq!(fm.note_type, NoteType::Resumo);
        assert!(fm.tags.is_empty());
        assert!(fm.linked_note.is_none());
        assert!(fm.attachments.is_empty());
    }

    #[test]
    fn frontmatter_yaml_round_trip_preserves_known_fields() {
        let fm = Frontmatter {
            id: "abc".into(),
            note_type: NoteType::Citacao,
            tags: vec!["one".into(), "two".into()],
            source: "book".into(),
            source_link: "https://example.com".into(),
            linked_note: Some("other-id".into()),
            attachments: vec!["img.png".into()],
            created: "2026-05-13T10:00:00Z".into(),
        };
        let yaml = serde_yaml::to_string(&fm).unwrap();
        let back: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.id, fm.id);
        assert_eq!(back.note_type, fm.note_type);
        assert_eq!(back.tags, fm.tags);
        assert_eq!(back.source, fm.source);
        assert_eq!(back.source_link, fm.source_link);
        assert_eq!(back.linked_note, fm.linked_note);
        assert_eq!(back.attachments, fm.attachments);
        assert_eq!(back.created, fm.created);
    }

    #[test]
    fn confirm_action_debug_does_not_panic() {
        let a = ConfirmAction::DeleteNote("note-id".into());
        let b = ConfirmAction::DeleteFolder(PathBuf::from("Folder"));
        let _ = format!("{a:?} {b:?}");
    }
}
