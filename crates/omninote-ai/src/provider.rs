use crate::config::LlmConfig;
use crate::error::{LlmError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Selects which provider backend the factory builds. Serialized lowercase in
/// `llm.toml` (`provider = "claude"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Claude,
    Grok,
    Ollama,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Grok => "grok",
            Self::Ollama => "ollama",
        }
    }
}

/// A single turn in a completion request. Roles mirror the chat conventions
/// shared across providers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Input to a completion call. Auto-tag, summary, and `ask` build one of these.
#[derive(Clone, Debug, Default)]
pub struct CompletionRequest {
    pub messages: Vec<ChatMessage>,
    /// Upper bound on generated tokens; `None` defers to the provider default.
    pub max_tokens: Option<u32>,
    /// Sampling temperature; `None` defers to the provider default.
    pub temperature: Option<f32>,
}

/// Output of a completion call.
#[derive(Clone, Debug)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
}

/// Backend-agnostic LLM interface. Implemented by per-vendor providers and
/// dispatched by `build_provider`. Async because real implementations perform
/// network I/O; the embedding path lets the RAG layer reuse the same handle.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Stable provider identifier, e.g. `"claude"`.
    fn name(&self) -> &'static str;

    /// Run a chat completion.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Produce embedding vectors for the given inputs. Providers without an
    /// embedding endpoint return `LlmError::Unsupported`.
    async fn embed(&self, _inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        Err(LlmError::Unsupported {
            provider: self.name().to_string(),
            capability: "embeddings",
        })
    }
}

/// Shared stub body: a provider that compiles and dispatches but has no
/// transport yet. Every method fails closed with `NotConfigured` so callers get
/// a typed, actionable error instead of a silent stand-in response.
macro_rules! stub_provider {
    ($ty:ident, $name:literal) => {
        pub struct $ty {
            #[allow(dead_code)]
            config: LlmConfig,
        }

        impl $ty {
            pub fn new(config: LlmConfig) -> Self {
                Self { config }
            }
        }

        #[async_trait]
        impl LlmProvider for $ty {
            fn name(&self) -> &'static str {
                $name
            }

            async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
                Err(LlmError::NotConfigured($name.to_string()))
            }
        }
    };
}

stub_provider!(ClaudeProvider, "claude");
stub_provider!(GrokProvider, "grok");
stub_provider!(OllamaProvider, "ollama");

/// Build the provider selected by `config.provider`. Returns a boxed trait
/// object so consumers hold one handle regardless of backend.
pub fn build_provider(config: LlmConfig) -> Box<dyn LlmProvider> {
    match config.provider {
        ProviderKind::Claude => Box::new(ClaudeProvider::new(config)),
        ProviderKind::Grok => Box::new(GrokProvider::new(config)),
        ProviderKind::Ollama => Box::new(OllamaProvider::new(config)),
    }
}
