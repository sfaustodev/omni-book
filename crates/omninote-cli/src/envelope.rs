//! Structured `--json` output envelope shared by every CLI verb.
//!
//! Shape:
//! - success: `{"ok": true, "data": <payload>, "meta": <obj>?}`
//! - failure: `{"ok": false, "error": "<message>"}`
//!
//! `data`/`meta`/`error` are omitted when absent (`skip_serializing_if`), so
//! a bare success collapses to `{"ok": true}`. Generic over the payload type
//! so callers serialize strongly-typed values instead of ad-hoc `json!`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Machine-readable result wrapper. `T` is the success payload type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub ok: bool,
    // `Option` fields deserialize to `None` when absent without `serde(default)`,
    // which would otherwise force a spurious `T: Default` bound on the payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> Envelope<T> {
    /// Success with a payload and no meta.
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            meta: None,
            error: None,
        }
    }

    /// Success with a payload plus a `meta` object (e.g. counts, echoed query).
    pub fn ok_meta(data: T, meta: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            meta: Some(meta),
            error: None,
        }
    }

    /// Failure carrying a human-readable message. `data`/`meta` stay empty.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            meta: None,
            error: Some(msg.into()),
        }
    }
}

impl<T: Serialize> Envelope<T> {
    /// Serialize to a single-line JSON string. Infallible for the types this
    /// CLI feeds in (plain structs / serde_json::Value), but propagates any
    /// serializer error rather than panicking.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize and print to stdout on its own line.
    pub fn print(&self) -> Result<(), serde_json::Error> {
        println!("{}", self.to_json()?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_omits_meta_and_error() {
        let env = Envelope::ok(json!({"path": "/v"}));
        let s = env.to_json().unwrap();
        assert_eq!(s, r#"{"ok":true,"data":{"path":"/v"}}"#);
    }

    #[test]
    fn ok_meta_includes_meta() {
        let env = Envelope::ok_meta(json!([1, 2]), json!({"count": 2}));
        let s = env.to_json().unwrap();
        assert!(s.contains(r#""ok":true"#));
        assert!(s.contains(r#""data":[1,2]"#));
        assert!(s.contains(r#""meta":{"count":2}"#));
        assert!(!s.contains("error"));
    }

    #[test]
    fn error_omits_data_and_meta() {
        let env: Envelope<Value> = Envelope::error("boom");
        let s = env.to_json().unwrap();
        assert_eq!(s, r#"{"ok":false,"error":"boom"}"#);
    }

    #[test]
    fn round_trip_success() {
        let original = Envelope::ok_meta(json!({"x": 1}), json!({"count": 1}));
        let s = original.to_json().unwrap();
        let back: Envelope<Value> = serde_json::from_str(&s).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn round_trip_error() {
        let original: Envelope<Value> = Envelope::error("not found");
        let s = original.to_json().unwrap();
        let back: Envelope<Value> = serde_json::from_str(&s).unwrap();
        assert_eq!(back, original);
        assert!(!back.ok);
        assert_eq!(back.error.as_deref(), Some("not found"));
    }

    #[test]
    fn round_trip_typed_payload() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Payload {
            name: String,
            n: u32,
        }
        let original = Envelope::ok(Payload {
            name: "a".into(),
            n: 7,
        });
        let s = original.to_json().unwrap();
        let back: Envelope<Payload> = serde_json::from_str(&s).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn deserializes_bare_ok_without_optional_fields() {
        let back: Envelope<Value> = serde_json::from_str(r#"{"ok":true}"#).unwrap();
        assert!(back.ok);
        assert!(back.data.is_none());
        assert!(back.meta.is_none());
        assert!(back.error.is_none());
    }
}
