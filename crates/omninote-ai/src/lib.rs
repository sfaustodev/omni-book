//! OmniNote AI layer — LLM provider abstraction and per-vault AI config.
//!
//! Scaffold for CAD-23 (AI-native vault). Defines the `LlmProvider` trait that
//! auto-tag, RAG/`ask`, and dictation/OCR build on, the `llm.toml` config
//! schema, and compile-ready stub providers (Claude/Grok/Ollama) that fail
//! closed with typed errors. No network calls and no heavy ML dependencies live
//! here; those land once a provider is chosen.
//!
//! Depends on `omninote-core` for vault types; never duplicates vault logic.

pub mod config;
pub mod error;
pub mod provider;

pub use config::{LlmConfig, LLM_CONFIG_FILE};
pub use error::{LlmError, Result};
pub use provider::{
    build_provider, ChatMessage, CompletionRequest, CompletionResponse, LlmProvider, ProviderKind,
    Role,
};

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_TOML: &str = r#"
provider = "ollama"
model = "llama3.1"
api_key_env = "OLLAMA_API_KEY"
base_url = "http://localhost:11434"
embed_model = "nomic-embed-text"
"#;

    #[test]
    fn parses_full_config() {
        let cfg = LlmConfig::from_toml_str(FULL_TOML).expect("valid toml");
        assert_eq!(cfg.provider, ProviderKind::Ollama);
        assert_eq!(cfg.model, "llama3.1");
        assert_eq!(cfg.api_key_env, "OLLAMA_API_KEY");
        assert_eq!(cfg.base_url.as_deref(), Some("http://localhost:11434"));
        assert_eq!(cfg.embed_model, "nomic-embed-text");
    }

    #[test]
    fn defaults_fill_missing_fields() {
        // Empty document => every field falls back to its default.
        let cfg = LlmConfig::from_toml_str("").expect("empty toml is valid");
        let default = LlmConfig::default();
        assert_eq!(cfg.provider, default.provider);
        assert_eq!(cfg.provider, ProviderKind::Claude);
        assert_eq!(cfg.model, default.model);
        assert_eq!(cfg.api_key_env, "ANTHROPIC_API_KEY");
        assert!(cfg.base_url.is_none());
        assert_eq!(cfg.embed_model, default.embed_model);
    }

    #[test]
    fn round_trips_through_toml() {
        let original = LlmConfig {
            provider: ProviderKind::Grok,
            model: "grok-beta".to_string(),
            api_key_env: "XAI_API_KEY".to_string(),
            base_url: Some("https://api.x.ai/v1".to_string()),
            embed_model: "custom-embed".to_string(),
        };
        let serialized = original.to_toml_string().expect("serialize");
        let parsed = LlmConfig::from_toml_str(&serialized).expect("reparse");
        assert_eq!(parsed.provider, original.provider);
        assert_eq!(parsed.model, original.model);
        assert_eq!(parsed.api_key_env, original.api_key_env);
        assert_eq!(parsed.base_url, original.base_url);
        assert_eq!(parsed.embed_model, original.embed_model);
    }

    #[test]
    fn none_base_url_is_omitted_from_serialization() {
        let cfg = LlmConfig::default();
        let serialized = cfg.to_toml_string().expect("serialize");
        assert!(!serialized.contains("base_url"));
    }

    #[test]
    fn errors_on_malformed_toml() {
        // Unterminated string => TOML syntax error.
        let err = LlmConfig::from_toml_str("provider = \"claude").unwrap_err();
        let _ = err; // a parse error is sufficient
    }

    #[test]
    fn errors_on_unknown_provider_variant() {
        let err = LlmConfig::from_toml_str("provider = \"gpt4\"").unwrap_err();
        let _ = err;
    }

    #[test]
    fn load_for_vault_returns_default_when_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = LlmConfig::load_for_vault(dir.path()).expect("missing file => default");
        assert_eq!(cfg.provider, ProviderKind::Claude);
    }

    #[test]
    fn load_reads_from_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_dir = dir.path().join(".omninote");
        std::fs::create_dir_all(&cfg_dir).expect("mkdir");
        let path = LlmConfig::path_for_vault(dir.path());
        std::fs::write(&path, "provider = \"grok\"\nmodel = \"grok-2\"\n").expect("write");

        let cfg = LlmConfig::load_for_vault(dir.path()).expect("load");
        assert_eq!(cfg.provider, ProviderKind::Grok);
        assert_eq!(cfg.model, "grok-2");
        // Unspecified field still defaults.
        assert_eq!(cfg.api_key_env, "ANTHROPIC_API_KEY");
    }

    #[test]
    fn load_surfaces_parse_error_for_malformed_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_dir = dir.path().join(".omninote");
        std::fs::create_dir_all(&cfg_dir).expect("mkdir");
        let path = LlmConfig::path_for_vault(dir.path());
        std::fs::write(&path, "this is = = not toml").expect("write");

        let err = LlmConfig::load(&path).unwrap_err();
        assert!(matches!(err, LlmError::ConfigParse { .. }));
    }

    #[test]
    fn path_for_vault_uses_omninote_dir() {
        let path = LlmConfig::path_for_vault(std::path::Path::new("/vault"));
        assert!(path.ends_with(".omninote/llm.toml"));
    }

    #[test]
    fn resolve_api_key_errors_when_env_unset() {
        let cfg = LlmConfig {
            api_key_env: "OMNINOTE_DEFINITELY_UNSET_KEY_XYZ".to_string(),
            ..LlmConfig::default()
        };
        let err = cfg.resolve_api_key().unwrap_err();
        assert!(matches!(err, LlmError::MissingApiKey(_)));
    }

    #[test]
    fn factory_dispatches_to_matching_stub() {
        for (kind, expected) in [
            (ProviderKind::Claude, "claude"),
            (ProviderKind::Grok, "grok"),
            (ProviderKind::Ollama, "ollama"),
        ] {
            let cfg = LlmConfig {
                provider: kind,
                ..LlmConfig::default()
            };
            let provider = build_provider(cfg);
            assert_eq!(provider.name(), expected);
        }
    }

    #[test]
    fn stub_completion_fails_closed() {
        let provider = build_provider(LlmConfig::default());
        let req = CompletionRequest {
            messages: vec![ChatMessage::user("hi")],
            ..CompletionRequest::default()
        };
        let result = futures_block_on(provider.complete(req));
        assert!(matches!(result, Err(LlmError::NotConfigured(_))));
    }

    #[test]
    fn stub_embed_is_unsupported() {
        let provider = build_provider(LlmConfig::default());
        let result = futures_block_on(provider.embed(&["x".to_string()]));
        assert!(matches!(result, Err(LlmError::Unsupported { .. })));
    }

    /// Minimal executor so async trait methods can be driven without pulling a
    /// runtime dependency into this scaffold crate. Polls a pinned future to
    /// completion; the stub futures never yield `Pending`.
    fn futures_block_on<F: std::future::Future>(fut: F) -> F::Output {
        use std::pin::pin;
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

        fn noop(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut fut = pin!(fut);
        loop {
            if let Poll::Ready(out) = fut.as_mut().poll(&mut cx) {
                return out;
            }
        }
    }
}
