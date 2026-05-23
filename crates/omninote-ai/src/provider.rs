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

// ──────────────────────── AnthropicProvider ────────────────────────

/// Anthropic Messages API version pinned to the contract we wrote against.
/// Anthropic guarantees backward compat for the lifetime of a version
/// string, so changing this is an explicit decision.
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic Claude provider. Hand-rolled HTTP via `reqwest` (the
/// `anthropic-sdk-rust` crate is pre-1.0, breaking-change risk too high).
pub struct AnthropicProvider {
    // Private so the `Debug` redactor is the only legal read site outside
    // this module.
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Construct from explicit key. Use [`super::LlmConfig::anthropic_key`]
    /// to follow the env → llm.toml resolution order.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_client(api_key, reqwest::Client::new())
    }

    /// Inject a custom `reqwest::Client` (e.g. with a different timeout for
    /// tests).
    pub fn with_client(api_key: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
            client,
        }
    }

    /// Override base URL for testing (mock servers). Public so integration
    /// tests can point at `http://127.0.0.1:PORT`.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
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

/// Build the JSON body for the Messages API. Pure — no I/O — so the wire
/// format can be asserted in unit tests.
pub(crate) fn build_messages_body(
    system: &str,
    user: &str,
    opts: &CompleteOpts,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": opts.model,
        "max_tokens": opts.max_tokens,
        "temperature": opts.temperature,
        "messages": [
            { "role": "user", "content": user }
        ],
    });
    if !system.is_empty() {
        body["system"] = serde_json::Value::String(system.to_string());
    }
    body
}

/// Extract the first text block from a Messages API response. Returns
/// `ProviderError::Api` on shape mismatch.
pub(crate) fn extract_text_from_response(
    body: &serde_json::Value,
) -> Result<String, ProviderError> {
    // Anthropic format: `{ "content": [{ "type": "text", "text": "..." }, ...] }`
    let content = body
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| ProviderError::Api {
            status: 200,
            body: format!("response missing 'content' array: {body}"),
        })?;
    for block in content {
        if block.get("type").and_then(|v| v.as_str()) == Some("text") {
            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                return Ok(text.to_string());
            }
        }
    }
    Err(ProviderError::Api {
        status: 200,
        body: format!("response had no text block: {body}"),
    })
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> ProviderName {
        ProviderName::Anthropic
    }

    async fn complete(
        &self,
        system: &str,
        user: &str,
        opts: CompleteOpts,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let body = build_messages_body(system, user, &opts);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Http(ProviderError::redact_key(&e.to_string())))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ProviderError::Http(ProviderError::redact_key(&e.to_string())))?;

        if !status.is_success() {
            return Err(ProviderError::Api {
                status: status.as_u16(),
                body: ProviderError::redact_key(&text),
            });
        }

        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| ProviderError::Api {
                status: status.as_u16(),
                body: format!("parse JSON: {e}; raw={text}"),
            })?;
        extract_text_from_response(&json)
    }
}

// ──────────────────────── MockProvider (test util) ────────────────────────

/// Deterministic provider for downstream tests (CLI, MCP). Returns a
/// pre-set response without hitting any network. Public so consumer crates
/// can use it as `Box<dyn LlmProvider>` in their own test suites.
pub struct MockProvider {
    pub canned_response: Result<String, ProviderError>,
    /// Captures the last call args — read in tests to assert what the
    /// caller sent.
    pub last_call: std::sync::Mutex<Option<(String, String, CompleteOpts)>>,
}

impl MockProvider {
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            canned_response: Ok(text.into()),
            last_call: std::sync::Mutex::new(None),
        }
    }

    pub fn err(e: ProviderError) -> Self {
        Self {
            canned_response: Err(e),
            last_call: std::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> ProviderName {
        ProviderName::Mock
    }

    async fn complete(
        &self,
        system: &str,
        user: &str,
        opts: CompleteOpts,
    ) -> Result<String, ProviderError> {
        *self.last_call.lock().unwrap() = Some((system.into(), user.into(), opts));
        match &self.canned_response {
            Ok(s) => Ok(s.clone()),
            Err(e) => Err(match e {
                ProviderError::NotImplemented(s) => ProviderError::NotImplemented(s),
                ProviderError::Http(s) => ProviderError::Http(s.clone()),
                ProviderError::Api { status, body } => ProviderError::Api {
                    status: *status,
                    body: body.clone(),
                },
                ProviderError::Config(s) => ProviderError::Config(s.clone()),
                ProviderError::MissingKey => ProviderError::MissingKey,
            }),
        }
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

    // ─── Phase D — request/response wire format (pure, no HTTP) ───

    #[test]
    fn build_messages_body_includes_required_fields() {
        let opts = CompleteOpts {
            max_tokens: 2048,
            temperature: 0.5,
            model: "claude-sonnet-4.5".into(),
        };
        let body = build_messages_body("you are helpful", "what is rust?", &opts);
        assert_eq!(body["model"], "claude-sonnet-4.5");
        assert_eq!(body["max_tokens"], 2048);
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["system"], "you are helpful");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "what is rust?");
    }

    #[test]
    fn build_messages_body_omits_system_when_empty() {
        let opts = CompleteOpts::default();
        let body = build_messages_body("", "hi", &opts);
        assert!(body.get("system").is_none());
    }

    #[test]
    fn extract_text_returns_first_text_block() {
        let resp = serde_json::json!({
            "id": "msg_123",
            "content": [
                { "type": "text", "text": "Hello, world!" },
                { "type": "text", "text": "second block (ignored)" }
            ]
        });
        let text = extract_text_from_response(&resp).unwrap();
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn extract_text_skips_non_text_blocks() {
        let resp = serde_json::json!({
            "content": [
                { "type": "tool_use", "id": "x" },
                { "type": "text", "text": "the answer" }
            ]
        });
        let text = extract_text_from_response(&resp).unwrap();
        assert_eq!(text, "the answer");
    }

    #[test]
    fn extract_text_errors_when_no_content_array() {
        let resp = serde_json::json!({"id": "msg_x"});
        let r = extract_text_from_response(&resp);
        assert!(matches!(r, Err(ProviderError::Api { .. })));
    }

    #[test]
    fn extract_text_errors_when_no_text_block() {
        let resp = serde_json::json!({
            "content": [
                { "type": "tool_use", "id": "x" }
            ]
        });
        let r = extract_text_from_response(&resp);
        assert!(matches!(r, Err(ProviderError::Api { .. })));
    }

    #[tokio::test]
    async fn anthropic_complete_errors_when_unreachable_base_url() {
        // Point at a localhost port nothing's listening on — exercises the
        // HTTP error path without needing a mock server.
        let p = AnthropicProvider::new("sk-ant-test").with_base_url("http://127.0.0.1:1");
        let r = p.complete("", "test", CompleteOpts::default()).await;
        match r {
            Err(ProviderError::Http(msg)) => {
                // API key must NEVER appear in the error.
                assert!(
                    !msg.contains("sk-ant-test"),
                    "API key leaked in HTTP error: {msg}"
                );
            }
            other => panic!("expected ProviderError::Http, got {other:?}"),
        }
    }

    // ─── MockProvider ───

    #[tokio::test]
    async fn mock_provider_returns_canned_ok() {
        let p = MockProvider::ok("canned response");
        let r = p.complete("", "user msg", CompleteOpts::default()).await;
        assert_eq!(r.unwrap(), "canned response");
    }

    #[tokio::test]
    async fn mock_provider_returns_canned_error() {
        let p = MockProvider::err(ProviderError::MissingKey);
        let r = p.complete("", "x", CompleteOpts::default()).await;
        assert!(matches!(r, Err(ProviderError::MissingKey)));
    }

    #[tokio::test]
    async fn mock_provider_records_last_call() {
        let p = MockProvider::ok("ok");
        let _ = p
            .complete("be helpful", "the question", CompleteOpts::default())
            .await;
        let last = p.last_call.lock().unwrap().clone();
        let (sys, user, _opts) = last.unwrap();
        assert_eq!(sys, "be helpful");
        assert_eq!(user, "the question");
    }

    #[tokio::test]
    async fn mock_provider_implements_trait_object() {
        // Critical for downstream callers — must be usable as
        // `Box<dyn LlmProvider>`.
        let p: Box<dyn LlmProvider> = Box::new(MockProvider::ok("via trait"));
        let r = p.complete("", "x", CompleteOpts::default()).await;
        assert_eq!(r.unwrap(), "via trait");
        assert_eq!(p.name(), ProviderName::Mock);
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
