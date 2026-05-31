use thiserror::Error;

/// Errors surfaced by the AI layer: config loading and provider operations.
#[derive(Debug, Error)]
pub enum LlmError {
    /// A provider was selected but its backend is not wired yet. Returned by the
    /// stub completion/embedding paths until a real transport lands.
    #[error("provider `{0}` is not configured for network access")]
    NotConfigured(String),

    /// The config names an `api_key_env`, but that environment variable is unset
    /// or empty. The raw key is never stored in config; it is read at call time.
    #[error("environment variable `{0}` is unset or empty")]
    MissingApiKey(String),

    /// The requested capability (e.g. embeddings) is not implemented by a provider.
    #[error("provider `{provider}` does not support {capability}")]
    Unsupported {
        provider: String,
        capability: &'static str,
    },

    /// Config file could not be read from disk.
    #[error("failed to read config at {path}: {source}")]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Config file contents are not valid TOML or do not match the schema.
    #[error("failed to parse config at {path}: {source}")]
    ConfigParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    /// Config could not be serialized back to TOML.
    #[error("failed to serialize config: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),
}

/// Convenience alias for fallible AI-layer operations.
pub type Result<T> = std::result::Result<T, LlmError>;
