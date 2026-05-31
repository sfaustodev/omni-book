use omninote_ai::config::LlmConfig;
use omninote_ai::error::LlmError;
use omninote_ai::provider::{build_provider, ChatMessage, CompletionRequest, ProviderKind};
use std::fs;
use tempfile::tempdir;

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

#[test]
fn test_resolve_api_key_whitespace_and_missing() {
    let cfg = LlmConfig {
        api_key_env: "TEST_AGY_API_KEY".to_string(),
        ..Default::default()
    };

    // 1. Missing env var
    std::env::remove_var("TEST_AGY_API_KEY");
    assert!(matches!(
        cfg.resolve_api_key(),
        Err(LlmError::MissingApiKey(ref env)) if env == "TEST_AGY_API_KEY"
    ));

    // 2. Whitespace-only env var
    std::env::set_var("TEST_AGY_API_KEY", "   \n  ");
    assert!(matches!(
        cfg.resolve_api_key(),
        Err(LlmError::MissingApiKey(ref env)) if env == "TEST_AGY_API_KEY"
    ));

    // 3. Correct value
    std::env::set_var("TEST_AGY_API_KEY", "secret-key-123");
    let key = cfg.resolve_api_key().unwrap();
    assert_eq!(key, "secret-key-123");
    std::env::remove_var("TEST_AGY_API_KEY");
}

#[test]
fn test_load_for_vault_when_config_is_directory() {
    let tmp = tempdir().unwrap();
    let omninote_dir = tmp.path().join(".omninote");
    fs::create_dir_all(&omninote_dir).unwrap();

    // Create a directory named llm.toml instead of a file
    let config_dir_path = omninote_dir.join("llm.toml");
    fs::create_dir_all(&config_dir_path).unwrap();

    // Reading a directory as a file must return ConfigRead error, not silently load default!
    let res = LlmConfig::load_for_vault(tmp.path());
    assert!(res.is_err(), "Should error when config path is a directory");
    assert!(matches!(res.unwrap_err(), LlmError::ConfigRead { .. }));
}

#[test]
fn test_stub_providers_name_and_not_configured() {
    for kind in [
        ProviderKind::Claude,
        ProviderKind::Grok,
        ProviderKind::Ollama,
    ] {
        let cfg = LlmConfig {
            provider: kind,
            ..Default::default()
        };
        let provider = build_provider(cfg);

        assert_eq!(provider.name(), kind.as_str());

        // Stub provider should fail closed with NotConfigured
        let req = CompletionRequest {
            messages: vec![ChatMessage::user("hello")],
            ..Default::default()
        };

        // Let's block on the future using our custom block_on
        let res = futures_block_on(provider.complete(req));
        assert!(
            matches!(res, Err(LlmError::NotConfigured(ref name)) if name == kind.as_str()),
            "Expected NotConfigured error for stub provider: {:?}",
            kind
        );
    }
}
