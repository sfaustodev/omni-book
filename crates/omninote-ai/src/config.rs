use crate::error::{LlmError, Result};
use crate::provider::ProviderKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Config file name inside the vault's `.omninote/` directory.
pub const LLM_CONFIG_FILE: &str = "llm.toml";

fn default_model() -> String {
    "claude-3-5-sonnet-latest".to_string()
}

fn default_api_key_env() -> String {
    "ANTHROPIC_API_KEY".to_string()
}

fn default_embed_model() -> String {
    "bge-small-en-v1.5".to_string()
}

/// Schema for `<vault>/.omninote/llm.toml`.
///
/// Holds no secrets: `api_key_env` is the *name* of an environment variable to
/// read the key from at call time, never the key itself.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Which provider backend to dispatch to.
    #[serde(default)]
    pub provider: ProviderKind,

    /// Completion model identifier passed to the provider.
    #[serde(default = "default_model")]
    pub model: String,

    /// Name of the environment variable holding the API key. The key value is
    /// resolved lazily; it is intentionally absent from this struct.
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Override for the provider HTTP endpoint. `None` uses the provider default
    /// (e.g. the local Ollama daemon, or the vendor's public API host).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Embedding model identifier used by the RAG layer (deferred).
    #[serde(default = "default_embed_model")]
    pub embed_model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::default(),
            model: default_model(),
            api_key_env: default_api_key_env(),
            base_url: None,
            embed_model: default_embed_model(),
        }
    }
}

impl LlmConfig {
    /// Path to `llm.toml` for a given vault root.
    pub fn path_for_vault(vault_root: &Path) -> PathBuf {
        vault_root.join(".omninote").join(LLM_CONFIG_FILE)
    }

    /// Load and parse config from an explicit file path.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|source| LlmError::ConfigRead {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_toml_str(&raw).map_err(|source| LlmError::ConfigParse {
            path: path.display().to_string(),
            source,
        })
    }

    /// Load config for a vault, falling back to defaults when `llm.toml` is absent.
    /// A present-but-malformed file is an error; a missing file is not.
    pub fn load_for_vault(vault_root: &Path) -> Result<Self> {
        let path = Self::path_for_vault(vault_root);
        if path.exists() {
            Self::load(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Parse from a TOML string without path context (used by `load` and tests).
    pub fn from_toml_str(s: &str) -> std::result::Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Serialize to a pretty TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Resolve the API key from the configured environment variable. Errors if
    /// the variable is unset or empty so callers fail closed rather than sending
    /// an empty credential.
    pub fn resolve_api_key(&self) -> Result<String> {
        match std::env::var(&self.api_key_env) {
            Ok(v) if !v.is_empty() => Ok(v),
            _ => Err(LlmError::MissingApiKey(self.api_key_env.clone())),
        }
    }
}
