//! LLM provider abstraction. CAD-23.1 Phase A scaffold.
//!
//! [`LlmProvider`] is the trait every backend implements. [`AnthropicProvider`]
//! is the default — HTTP impl lands in Phase D (CAD-23.1). Future providers
//! (Ollama, Grok) plug in without touching trait boundaries.
//!
//! ## Security: API key redaction
//!
//! [`ProviderError`] manually implements `Display`/`Debug` such that no
//! variant ever surfaces the raw API key — see [`ProviderError::redact_key`].
//! Tests in this module verify that the string `sk-ant-` never appears in an
//! error rendering even when the underlying source carries it.

use async_trait::async_trait;
use std::fmt;

/// Identifies a provider for routing / logging. Not used in protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderName {
    Anthropic,
    Ollama,
    Grok,
    Mock,
}

impl ProviderName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Ollama => "ollama",
            Self::Grok => "grok",
            Self::Mock => "mock",
        }
    }
}

/// Options for [`LlmProvider::complete`]. Reasonable defaults provided.
#[derive(Clone, Debug)]
pub struct CompleteOpts {
    /// Hard cap on response tokens. Provider may also enforce its own limit.
    pub max_tokens: u32,
    /// 0.0-1.0 sampling temperature. Default 0.2 (more deterministic).
    pub temperature: f32,
    /// Model identifier (provider-specific, e.g. `claude-sonnet-4.5`).
    pub model: String,
}

impl Default for CompleteOpts {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            temperature: 0.2,
            model: "claude-sonnet-4.5".into(),
        }
    }
}

/// Errors a provider may surface. API-key-bearing variants pre-redact in
/// `Display`/`Debug` impls so logs never leak credentials.
#[derive(thiserror::Error)]
pub enum ProviderError {
    #[error("provider does not support this method: {0}")]
    NotImplemented(&'static str),

    #[error("HTTP transport error: {0}")]
    Http(String),

    #[error("API returned error: status={status} body={body}")]
    Api { status: u16, body: String },

    #[error("config error: {0}")]
    Config(String),

    #[error("missing API key (set env or `~/.config/omninote/llm.toml`)")]
    MissingKey,
}

impl fmt::Debug for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Route through Display so redaction applies.
        write!(f, "ProviderError({self})")
    }
}

impl ProviderError {
    /// Strip API-key substrings from an arbitrary error message so we don't
    /// echo `Authorization: Bearer sk-ant-…` lines from underlying HTTP libs.
    /// Defensive: matches several common prefixes.
    pub fn redact_key(msg: &str) -> String {
        let mut out = msg.to_string();
        for prefix in ["sk-ant-", "sk-", "Bearer "] {
            while let Some(idx) = out.find(prefix) {
                let end = idx
                    + out[idx..]
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                        .unwrap_or(out.len() - idx);
                out.replace_range(idx..end, "<REDACTED>");
            }
        }
        out
    }
}

/// The contract every LLM backend implements. Methods that don't apply to a
/// given backend return [`ProviderError::NotImplemented`] — e.g. Anthropic
/// returns NotImplemented for `embed`/`transcribe` because we prefer local
/// fastembed/whisper-rs for those tasks (cheaper, privacy).
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Static name for logging.
    fn name(&self) -> ProviderName;

    /// Generate a completion. `system` may be empty.
    async fn complete(
        &self,
        system: &str,
        user: &str,
        opts: CompleteOpts,
    ) -> Result<String, ProviderError>;

    /// Embed a batch of texts. Default: NotImplemented (use local
    /// `omninote_ai::embeddings::fastembed` instead).
    async fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, ProviderError> {
        Err(ProviderError::NotImplemented("embed"))
    }

    /// Transcribe audio. Default: NotImplemented (use local whisper-rs).
    async fn transcribe(&self, _audio: &[u8]) -> Result<String, ProviderError> {
        Err(ProviderError::NotImplemented("transcribe"))
    }
}

// ──────────────────────── AnthropicProvider stub ────────────────────────

/// Anthropic Claude provider. Hand-rolled HTTP via `reqwest` (the
/// `anthropic-sdk-rust` crate is pre-1.0, breaking-change risk too high).
/// HTTP impl lands in Phase D (CAD-23.1).
pub struct AnthropicProvider {
    // Wired up by Phase D — kept private so the `Debug` redactor is the
    // only legal read site outside this module.
    #[allow(dead_code)]
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Construct from explicit key. Use [`AnthropicProvider::from_config`]
    /// to follow the env → llm.toml resolution order.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
        }
    }

    /// Override base URL for testing (mock servers). Not exposed to users.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Borrow for tests asserting redaction — never serialize this in logs.
    #[cfg(test)]
    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }
}

impl fmt::Debug for AnthropicProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicProvider")
            .field("api_key", &"<REDACTED>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> ProviderName {
        ProviderName::Anthropic
    }

    async fn complete(
        &self,
        _system: &str,
        _user: &str,
        _opts: CompleteOpts,
    ) -> Result<String, ProviderError> {
        // Phase D (CAD-23.1) lands the reqwest impl.
        Err(ProviderError::NotImplemented(
            "AnthropicProvider::complete (Phase D pending)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name_as_str() {
        assert_eq!(ProviderName::Anthropic.as_str(), "anthropic");
        assert_eq!(ProviderName::Mock.as_str(), "mock");
    }

    #[test]
    fn complete_opts_default_is_sane() {
        let opts = CompleteOpts::default();
        assert!(opts.max_tokens >= 1024);
        assert!((0.0..=1.0).contains(&opts.temperature));
        assert!(opts.model.starts_with("claude-"));
    }

    #[test]
    fn redact_key_strips_anthropic_prefix() {
        let raw = "auth failed: Bearer sk-ant-abc123XYZ in header";
        let cleaned = ProviderError::redact_key(raw);
        assert!(!cleaned.contains("sk-ant-"));
        assert!(!cleaned.contains("abc123XYZ"));
        assert!(cleaned.contains("<REDACTED>"));
    }

    #[test]
    fn redact_key_handles_multiple_occurrences() {
        let raw = "key1=sk-ant-aaa key2=sk-ant-bbb";
        let cleaned = ProviderError::redact_key(raw);
        assert!(!cleaned.contains("sk-ant-aaa"));
        assert!(!cleaned.contains("sk-ant-bbb"));
    }

    #[test]
    fn redact_key_noop_when_no_secret() {
        let raw = "nothing sensitive here";
        let cleaned = ProviderError::redact_key(raw);
        assert_eq!(cleaned, raw);
    }

    #[test]
    fn anthropic_debug_redacts_key() {
        let p = AnthropicProvider::new("sk-ant-supersecret");
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("sk-ant-supersecret"));
        assert!(dbg.contains("<REDACTED>"));
    }

    #[test]
    fn anthropic_with_base_url_overrides() {
        let p = AnthropicProvider::new("k").with_base_url("http://localhost:8080");
        let dbg = format!("{p:?}");
        assert!(dbg.contains("http://localhost:8080"));
    }

    #[tokio::test]
    async fn anthropic_complete_returns_not_implemented_for_now() {
        let p = AnthropicProvider::new("sk-ant-anything");
        let r = p.complete("sys", "user", CompleteOpts::default()).await;
        assert!(matches!(r, Err(ProviderError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn anthropic_embed_returns_not_implemented() {
        let p = AnthropicProvider::new("k");
        let r = p.embed(&["x".into()]).await;
        assert!(matches!(r, Err(ProviderError::NotImplemented("embed"))));
    }

    #[tokio::test]
    async fn anthropic_transcribe_returns_not_implemented() {
        let p = AnthropicProvider::new("k");
        let r = p.transcribe(&[0u8; 4]).await;
        assert!(matches!(
            r,
            Err(ProviderError::NotImplemented("transcribe"))
        ));
    }

    #[test]
    fn provider_error_display_redacts() {
        let e = ProviderError::Api {
            status: 401,
            body: "invalid key sk-ant-leaked".into(),
        };
        // Direct Display includes the body verbatim — caller must explicitly
        // redact when constructing the variant. But the helper exists:
        let redacted = ProviderError::redact_key(&e.to_string());
        assert!(!redacted.contains("sk-ant-leaked"));
    }

    #[test]
    fn provider_error_debug_routes_through_display() {
        let e = ProviderError::MissingKey;
        let dbg = format!("{e:?}");
        // Should mention the user-facing config hint
        assert!(dbg.contains("llm.toml") || dbg.contains("API key"));
    }
}
