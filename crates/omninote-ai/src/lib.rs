//! AI features for OmniNote — RAG search, auto-tag, dictation, OCR. CAD-23.
//!
//! Local-first stack: `fastembed` (embeddings), `whisper-rs` (dictation),
//! `leptess` (OCR). LLM provider abstracted behind [`LlmProvider`] trait —
//! [`AnthropicProvider`] is the default impl; Ollama/Grok deferred to v2.
//!
//! Modules ship incrementally:
//! - **CAD-23.1** — `config`, `provider` (this scaffold), `embeddings`, `rag`
//! - **CAD-23.2** — `auto_tag`
//! - **CAD-23.3** — `dictation`
//! - **CAD-23.4** — `ocr`
//!
//! Consumers: `omninote-cli` (verbs `ask`, `tag`, `dictate`, `ocr`),
//! `omninote-mcp` (tools `vault_ask`, `note_auto_tag`, …), `omninote-gui`
//! (slash menu + dictation hotkey + AI chat panel).

pub mod config;
pub mod embeddings;
pub mod provider;
pub mod rag;

pub use config::{LlmConfig, ProviderConfig};
pub use embeddings::{
    chunk_note, cosine, embed_note, EmbeddedChunk, Embedder, EmbeddingIndex, FastEmbedder,
};
pub use provider::{AnthropicProvider, CompleteOpts, LlmProvider, ProviderError, ProviderName};
pub use rag::{Rag, RagHit, RagOpts, DEFAULT_MAX_CHUNK_CHARS};
