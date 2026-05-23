//! LLM configuration loading. CAD-23.1 Phase A.
//!
//! Resolution order for API key:
//! 1. `ANTHROPIC_API_KEY` env (preferred — never serialized to disk)
//! 2. `[anthropic].api_key` in `~/.config/omninote/llm.toml`
//! 3. [`ProviderError::MissingKey`]
//!
//! Resolution order for config file:
//! 1. `OMNINOTE_LLM_CONFIG` env (test override)
//! 2. `~/.config/omninote/llm.toml`
//! 3. [`LlmConfig::default()`] (no file required for `--no-llm` flows)
//!
//! Embedding / dictation / OCR backend configs ship with later phases;
//! their fields exist now with sensible defaults so tests don't break when
//! we wire them up.

use crate::provider::ProviderError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level LLM configuration. All sections optional — `Default` produces a
/// sane Anthropic-Claude profile.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LlmConfig {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
    #[serde(default)]
    pub dictation: DictationConfig,
    #[serde(default)]
    pub ocr: OcrConfig,
}

/// Which provider to use by default and which model.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// `"anthropic"` (default), `"ollama"`, or `"grok"`.
    #[serde(default = "default_provider_name")]
    pub name: String,
    /// Model identifier, provider-specific.
    #[serde(default = "default_model")]
    pub model: String,
    /// Hard cap on response tokens.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: default_provider_name(),
            model: default_model(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_provider_name() -> String {
    "anthropic".into()
}
fn default_model() -> String {
    "claude-sonnet-4.5".into()
}
fn default_max_tokens() -> u32 {
    4096
}

/// Anthropic-specific config. API key here is a fallback — prefer env.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AnthropicConfig {
    /// Fallback API key. `None` → must come from `ANTHROPIC_API_KEY` env.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Override API base URL (testing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Embeddings backend (Phase B fills this in).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmbeddingsConfig {
    #[serde(default = "default_emb_backend")]
    pub backend: String,
    #[serde(default = "default_emb_model")]
    pub model: String,
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            backend: default_emb_backend(),
            model: default_emb_model(),
        }
    }
}

fn default_emb_backend() -> String {
    "fastembed".into()
}
fn default_emb_model() -> String {
    "BAAI/bge-small-en-v1.5".into()
}

/// Dictation backend (CAD-23.3 fills in).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DictationConfig {
    #[serde(default = "default_dict_backend")]
    pub backend: String,
    #[serde(default = "default_dict_model")]
    pub model: String,
}

impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            backend: default_dict_backend(),
            model: default_dict_model(),
        }
    }
}

fn default_dict_backend() -> String {
    "whisper-rs".into()
}
fn default_dict_model() -> String {
    "base.en".into()
}

/// OCR backend (CAD-23.4 fills in).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OcrConfig {
    #[serde(default = "default_ocr_backend")]
    pub backend: String,
    #[serde(default = "default_ocr_lang")]
    pub lang: String,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            backend: default_ocr_backend(),
            lang: default_ocr_lang(),
        }
    }
}

fn default_ocr_backend() -> String {
    "tesseract".into()
}
fn default_ocr_lang() -> String {
    "eng".into()
}

impl LlmConfig {
    /// Resolve the LLM config file path. Honors `OMNINOTE_LLM_CONFIG` env
    /// for tests; otherwise `<config-dir>/omninote/llm.toml`.
    pub fn resolve_path() -> Option<PathBuf> {
        if let Ok(p) = std::env::var("OMNINOTE_LLM_CONFIG") {
            return Some(PathBuf::from(p));
        }
        Some(dirs::config_dir()?.join("omninote").join("llm.toml"))
    }

    /// Load from a specific path. Missing file → `default()`. Malformed TOML
    /// → `ProviderError::Config`.
    pub fn load_from(path: &Path) -> Result<Self, ProviderError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ProviderError::Config(format!("read {}: {e}", path.display())))?;
        toml::from_str(&raw)
            .map_err(|e| ProviderError::Config(format!("parse {}: {e}", path.display())))
    }

    /// Convenience: resolve path + load. Default if no config path can be
    /// determined (rare — Windows in single-user mode).
    pub fn load() -> Result<Self, ProviderError> {
        match Self::resolve_path() {
            Some(p) => Self::load_from(&p),
            None => Ok(Self::default()),
        }
    }

    /// Resolve the Anthropic API key (env wins, config falls back, else
    /// `MissingKey`).
    pub fn anthropic_key(&self) -> Result<String, ProviderError> {
        if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
            if !k.trim().is_empty() {
                return Ok(k);
            }
        }
        self.anthropic
            .api_key
            .clone()
            .filter(|k| !k.trim().is_empty())
            .ok_or(ProviderError::MissingKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    // Env vars are process-wide shared state and cargo runs tests in
    // parallel by default. This mutex serializes the env-touching tests so
    // they don't stomp on each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn isolated_env<F: FnOnce()>(f: F) {
        // Hold the lock for the entire window — set, run, restore.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let saved_cfg = std::env::var("OMNINOTE_LLM_CONFIG").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("OMNINOTE_LLM_CONFIG");
        f();
        match saved_key {
            Some(v) => std::env::set_var("ANTHROPIC_API_KEY", v),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match saved_cfg {
            Some(v) => std::env::set_var("OMNINOTE_LLM_CONFIG", v),
            None => std::env::remove_var("OMNINOTE_LLM_CONFIG"),
        }
    }

    #[test]
    fn default_config_has_sensible_anthropic_profile() {
        let cfg = LlmConfig::default();
        assert_eq!(cfg.provider.name, "anthropic");
        assert!(cfg.provider.model.starts_with("claude-"));
        assert!(cfg.provider.max_tokens >= 1024);
        assert_eq!(cfg.embeddings.backend, "fastembed");
        assert_eq!(cfg.dictation.backend, "whisper-rs");
        assert_eq!(cfg.ocr.backend, "tesseract");
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope.toml");
        let cfg = LlmConfig::load_from(&missing).unwrap();
        assert_eq!(cfg.provider.name, "anthropic");
    }

    #[test]
    fn load_parses_minimal_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("llm.toml");
        std::fs::write(
            &path,
            r#"
[provider]
name = "anthropic"
model = "claude-opus-4.7"
max_tokens = 8192
"#,
        )
        .unwrap();
        let cfg = LlmConfig::load_from(&path).unwrap();
        assert_eq!(cfg.provider.model, "claude-opus-4.7");
        assert_eq!(cfg.provider.max_tokens, 8192);
    }

    #[test]
    fn load_parses_full_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("llm.toml");
        std::fs::write(
            &path,
            r#"
[provider]
name = "anthropic"
model = "claude-sonnet-4.5"
max_tokens = 4096

[anthropic]
api_key = "sk-ant-abc"

[embeddings]
backend = "fastembed"
model = "BAAI/bge-base-en-v1.5"

[dictation]
backend = "whisper-rs"
model = "small.en"

[ocr]
backend = "tesseract"
lang = "eng+por"
"#,
        )
        .unwrap();
        let cfg = LlmConfig::load_from(&path).unwrap();
        assert_eq!(cfg.anthropic.api_key.as_deref(), Some("sk-ant-abc"));
        assert_eq!(cfg.embeddings.model, "BAAI/bge-base-en-v1.5");
        assert_eq!(cfg.dictation.model, "small.en");
        assert_eq!(cfg.ocr.lang, "eng+por");
    }

    #[test]
    fn load_errors_on_malformed_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.toml");
        std::fs::write(&path, "this is not = valid toml [[[[").unwrap();
        let err = LlmConfig::load_from(&path).unwrap_err();
        assert!(matches!(err, ProviderError::Config(_)));
    }

    #[test]
    fn anthropic_key_prefers_env_over_config() {
        isolated_env(|| {
            std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-from-env");
            let mut cfg = LlmConfig::default();
            cfg.anthropic.api_key = Some("sk-ant-from-toml".into());
            assert_eq!(cfg.anthropic_key().unwrap(), "sk-ant-from-env");
        });
    }

    #[test]
    fn anthropic_key_falls_back_to_config() {
        isolated_env(|| {
            let mut cfg = LlmConfig::default();
            cfg.anthropic.api_key = Some("sk-ant-from-toml".into());
            assert_eq!(cfg.anthropic_key().unwrap(), "sk-ant-from-toml");
        });
    }

    #[test]
    fn anthropic_key_errs_when_both_absent() {
        isolated_env(|| {
            let cfg = LlmConfig::default();
            let err = cfg.anthropic_key().unwrap_err();
            assert!(matches!(err, ProviderError::MissingKey));
        });
    }

    #[test]
    fn anthropic_key_treats_empty_env_as_missing() {
        isolated_env(|| {
            std::env::set_var("ANTHROPIC_API_KEY", "   ");
            let cfg = LlmConfig::default();
            let err = cfg.anthropic_key().unwrap_err();
            assert!(matches!(err, ProviderError::MissingKey));
        });
    }

    #[test]
    fn anthropic_key_treats_empty_config_as_missing() {
        isolated_env(|| {
            let mut cfg = LlmConfig::default();
            cfg.anthropic.api_key = Some("  ".into());
            let err = cfg.anthropic_key().unwrap_err();
            assert!(matches!(err, ProviderError::MissingKey));
        });
    }

    #[test]
    fn resolve_path_honors_env_override() {
        isolated_env(|| {
            std::env::set_var("OMNINOTE_LLM_CONFIG", "/tmp/custom.toml");
            assert_eq!(
                LlmConfig::resolve_path(),
                Some(PathBuf::from("/tmp/custom.toml"))
            );
        });
    }

    #[test]
    fn resolve_path_falls_back_to_config_dir() {
        isolated_env(|| {
            let p = LlmConfig::resolve_path().unwrap();
            assert!(p.ends_with("omninote/llm.toml"));
        });
    }
}
