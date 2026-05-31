//! Adversarial coverage for the AI config + provider scaffold (triad gate).
//! Focus: parse robustness, default fallthrough, and fail-closed key resolution.

use omninote_ai::{build_provider, LlmConfig, LlmError, ProviderKind};

#[test]
fn unknown_provider_value_is_rejected() {
    let err = LlmConfig::from_toml_str("provider = \"gemini\"\n");
    assert!(
        err.is_err(),
        "an unmodelled provider must not silently parse"
    );
}

#[test]
fn partial_config_fills_remaining_defaults() {
    let cfg = LlmConfig::from_toml_str("provider = \"grok\"\n").expect("valid partial toml");
    assert_eq!(cfg.provider, ProviderKind::Grok);
    // Every unset field falls back to its default rather than failing the parse.
    assert_eq!(cfg.model, "claude-3-5-sonnet-latest");
    assert_eq!(cfg.api_key_env, "ANTHROPIC_API_KEY");
    assert_eq!(cfg.embed_model, "bge-small-en-v1.5");
    assert!(cfg.base_url.is_none());
}

#[test]
fn empty_toml_is_all_defaults() {
    let cfg = LlmConfig::from_toml_str("").expect("empty is valid");
    assert_eq!(cfg.provider, ProviderKind::Claude);
    assert_eq!(cfg.model, "claude-3-5-sonnet-latest");
}

#[test]
fn malformed_toml_errors_not_panics() {
    assert!(LlmConfig::from_toml_str("provider = ").is_err());
    assert!(LlmConfig::from_toml_str("= \"x\"").is_err());
    assert!(LlmConfig::from_toml_str("[[[").is_err());
}

#[test]
fn to_toml_never_emits_a_secret_field() {
    // The key is resolved from the environment, never stored — serialization
    // must not invent an `api_key` field that could end up on disk.
    let cfg = LlmConfig::default();
    let text = cfg.to_toml_string().expect("serializes");
    assert!(!text.to_lowercase().contains("api_key ="));
    assert!(!text.contains("\nkey ="));
    // The env-var *name* is fine to persist.
    assert!(text.contains("api_key_env"));
}

#[test]
fn base_url_round_trips_and_omits_when_absent() {
    let with = LlmConfig::from_toml_str("base_url = \"http://localhost:11434\"\n").unwrap();
    assert_eq!(with.base_url.as_deref(), Some("http://localhost:11434"));
    let text = LlmConfig::default().to_toml_string().unwrap();
    assert!(!text.contains("base_url"), "None base_url must be omitted");
}

#[test]
fn resolve_api_key_unset_fails_closed() {
    let cfg = cfg_with_env("OMNINOTE_TRIAD_UNSET_KEY_XYZ");
    std::env::remove_var(&cfg.api_key_env);
    match cfg.resolve_api_key() {
        Err(LlmError::MissingApiKey(name)) => assert_eq!(name, cfg.api_key_env),
        other => panic!("unset var must fail closed, got {other:?}"),
    }
}

#[test]
fn resolve_api_key_empty_fails_closed() {
    let cfg = cfg_with_env("OMNINOTE_TRIAD_EMPTY_KEY_XYZ");
    std::env::set_var(&cfg.api_key_env, "");
    assert!(matches!(
        cfg.resolve_api_key(),
        Err(LlmError::MissingApiKey(_))
    ));
    std::env::remove_var(&cfg.api_key_env);
}

#[test]
fn resolve_api_key_whitespace_only_fails_closed() {
    // Regression: a blank-but-not-empty value must not be sent as a credential.
    let cfg = cfg_with_env("OMNINOTE_TRIAD_BLANK_KEY_XYZ");
    std::env::set_var(&cfg.api_key_env, "   \t ");
    assert!(matches!(
        cfg.resolve_api_key(),
        Err(LlmError::MissingApiKey(_))
    ));
    std::env::remove_var(&cfg.api_key_env);
}

#[test]
fn resolve_api_key_present_returns_value_verbatim() {
    let cfg = cfg_with_env("OMNINOTE_TRIAD_REAL_KEY_XYZ");
    std::env::set_var(&cfg.api_key_env, "sk-ant-secret-123");
    assert_eq!(cfg.resolve_api_key().unwrap(), "sk-ant-secret-123");
    std::env::remove_var(&cfg.api_key_env);
}

#[test]
fn stub_providers_fail_closed_not_silent() {
    // Every backend is a stub until transport lands: complete() must surface a
    // typed NotConfigured, never a fabricated empty response.
    for kind in ["claude", "grok", "ollama"] {
        let cfg = LlmConfig::from_toml_str(&format!("provider = \"{kind}\"\n")).unwrap();
        let provider = build_provider(cfg);
        assert_eq!(provider.name(), kind);
        let res = block_on(provider.complete(Default::default()));
        assert!(
            matches!(res, Err(LlmError::NotConfigured(_))),
            "{kind} stub must fail closed"
        );
    }
}

/// Build a config that reads its key from a named env var (struct-update keeps
/// clippy's field-reassign lint quiet and the call sites terse).
fn cfg_with_env(api_key_env: &str) -> LlmConfig {
    LlmConfig {
        api_key_env: api_key_env.to_string(),
        ..LlmConfig::default()
    }
}

/// Minimal std-only executor — avoids a tokio dev-dependency for futures that
/// resolve on first poll (the stubs return synchronously, no real await point).
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};

    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Arc::new(NoopWaker).into();
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
