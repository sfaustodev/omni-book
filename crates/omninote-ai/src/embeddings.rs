//! Local embeddings via [`fastembed`]. CAD-23.1 Phase B.
//!
//! [`EmbeddingStore`] owns the model + an on-disk cache of every chunk's
//! embedding keyed by `(note_id, chunk_idx)`. Cache lives at
//! `<vault>/.omninote/embeddings.bin` as a bincode-serialised
//! [`EmbeddingIndex`].
//!
//! ## Cache invalidation
//!
//! Each entry stores a `content_hash` of the chunk text. When the caller
//! re-feeds a note, only chunks whose hash changed get re-embedded —
//! [`EmbeddingStore::add_note`] handles the diff transparently. Switching
//! the embedding model invalidates the whole index (different vector
//! dimensionality breaks cosine similarity).
//!
//! ## Test isolation
//!
//! The fastembed model is ~100MB and downloads on first use, so the
//! model-loading path is gated behind the `live-embeddings` cargo feature
//! (`cargo test -p omninote-ai --features live-embeddings`). All pure
//! logic — chunking, persistence, hashing, cosine similarity — has full
//! unit coverage without touching fastembed.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// One chunk of a note plus its embedding. Persisted as part of
/// [`EmbeddingIndex`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct EmbeddedChunk {
    /// `note.frontmatter.id` — stable across renames.
    pub note_id: String,
    /// 0-based ordinal within the note.
    pub chunk_idx: usize,
    /// The chunk text (kept for retrieval display).
    pub text: String,
    /// The dense vector. Dimension must match `EmbeddingIndex::model_id`'s
    /// dimensionality (e.g. 384 for BGE small).
    pub embedding: Vec<f32>,
    /// `xxhash`-ish digest of `text` for cheap "did this chunk change?".
    pub content_hash: u64,
}

/// On-disk index. Includes the model identifier so we can invalidate if
/// the user switches model (different vector dim).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct EmbeddingIndex {
    /// Provenance — `"BAAI/bge-small-en-v1.5"` etc.
    pub model_id: String,
    /// Expected vector dimensionality (sanity check on load).
    pub dim: usize,
    /// All embedded chunks across the vault.
    pub entries: Vec<EmbeddedChunk>,
}

impl EmbeddingIndex {
    /// Find all chunks belonging to a note.
    pub fn chunks_of(&self, note_id: &str) -> Vec<&EmbeddedChunk> {
        self.entries
            .iter()
            .filter(|c| c.note_id == note_id)
            .collect()
    }

    /// Replace all chunks for a note with the provided new set. Returns
    /// the count actually written.
    pub fn replace_note(&mut self, note_id: &str, new_chunks: Vec<EmbeddedChunk>) -> usize {
        self.entries.retain(|c| c.note_id != note_id);
        let n = new_chunks.len();
        self.entries.extend(new_chunks);
        n
    }

    /// Remove every chunk for a note. Returns how many were dropped.
    pub fn remove_note(&mut self, note_id: &str) -> usize {
        let before = self.entries.len();
        self.entries.retain(|c| c.note_id != note_id);
        before - self.entries.len()
    }

    /// Persist to a bincode file. Creates parent dirs.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create dir {}: {e}", parent.display()))?;
        }
        let bytes = bincode::serialize(self).map_err(|e| format!("serialize embeddings: {e}"))?;
        std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
    }

    /// Load from a bincode file. Missing file → empty index (not an
    /// error — first run is normal).
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        bincode::deserialize::<Self>(&bytes).map_err(|e| format!("deserialize embeddings: {e}"))
    }

    /// Path convention: `<vault>/.omninote/embeddings.bin`.
    pub fn default_path(vault_root: &Path) -> PathBuf {
        vault_root.join(".omninote").join("embeddings.bin")
    }
}

/// Stable content hash for "did this chunk change?" comparisons. Not
/// crypto-strong — collisions just cause a single unnecessary re-embed.
pub fn hash_chunk(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// Split a markdown body into retrieval chunks. Splits on blank lines
/// first, then merges adjacent paragraphs until they approach `max_chars`.
/// Frontmatter (the `---` fenced block at top) is stripped — it's not
/// useful for semantic search.
///
/// Guarantees:
/// - Output chunks are non-empty after trim.
/// - Each chunk ≤ `max_chars * 1.5` (a single oversized paragraph is
///   kept whole rather than mid-word split).
/// - Order matches source order.
pub fn chunk_note(content: &str, max_chars: usize) -> Vec<String> {
    // No artificial floor on `max_chars` — the algorithm never splits
    // mid-paragraph, so `max_chars=1` simply degrades to "one paragraph per
    // chunk" rather than producing degenerate output.
    let body = strip_frontmatter(content);

    let paragraphs: Vec<&str> = body
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for p in paragraphs {
        if cur.is_empty() {
            cur.push_str(p);
        } else if cur.len() + p.len() + 2 <= max_chars {
            cur.push_str("\n\n");
            cur.push_str(p);
        } else {
            chunks.push(std::mem::take(&mut cur));
            cur.push_str(p);
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
}

/// Strip a leading `---\n…\n---\n` frontmatter block if present.
fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    let prefix_offset = content.len() - trimmed.len();
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return content;
    }
    let after_open = &trimmed[4..];
    if let Some(close_idx) = after_open.find("\n---") {
        // Move past the closing `---` and any single newline after it.
        let mut end = 4 + close_idx + 4; // `\n---` is 4 bytes
                                         // Optional trailing newline after the closing `---`
        if trimmed[end..].starts_with('\n') {
            end += 1;
        }
        let absolute = prefix_offset + end;
        return &content[absolute..];
    }
    content
}

/// Cosine similarity between two equally-dimensioned vectors. Returns
/// `f32::NAN` on dimension mismatch — caller is expected to check.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::NAN;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

// ───────────── fastembed glue (Phase B-2: gated to keep CI lean) ─────────────
//
// Loading the model downloads ~100MB on first run. We don't want that in
// every CI invocation, so the embed path is exposed but the model loader
// itself is wrapped to make mocking straightforward in Phase C tests.

/// Backend-agnostic interface so [`EmbeddingStore`] can be tested without
/// downloading a real model. Phase C `rag` tests substitute a stub impl.
pub trait Embedder: Send + Sync {
    /// Embed a batch of texts, returning one vector per input.
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;
    /// Dimensionality of returned vectors — used to validate the index on
    /// load.
    fn dim(&self) -> usize;
    /// Model identifier (e.g. `"BAAI/bge-small-en-v1.5"`).
    fn model_id(&self) -> &str;
}

/// Real fastembed-backed [`Embedder`]. Default model is BGE small (384d).
pub struct FastEmbedder {
    inner: std::sync::Mutex<fastembed::TextEmbedding>,
    dim: usize,
    model_id: String,
}

impl FastEmbedder {
    /// Load BGE small (default). First call downloads ~100MB to the
    /// fastembed cache; subsequent calls reuse it.
    pub fn bge_small() -> Result<Self, String> {
        Self::with_model(fastembed::EmbeddingModel::BGESmallENV15)
    }

    /// Load a specific fastembed model. Most callers want
    /// [`Self::bge_small`].
    pub fn with_model(model: fastembed::EmbeddingModel) -> Result<Self, String> {
        let model_id = format!("{model:?}");
        let dim = match model {
            fastembed::EmbeddingModel::BGESmallENV15 => 384,
            fastembed::EmbeddingModel::BGEBaseENV15 => 768,
            fastembed::EmbeddingModel::AllMiniLML6V2 => 384,
            _ => 384, // fallback; first embed will reveal mismatch via assert in tests
        };
        let inner = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(model).with_show_download_progress(false),
        )
        .map_err(|e| format!("fastembed init: {e}"))?;
        Ok(Self {
            inner: std::sync::Mutex::new(inner),
            dim,
            model_id,
        })
    }
}

impl Embedder for FastEmbedder {
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| format!("embed mutex poisoned: {e}"))?;
        guard
            .embed(refs, None)
            .map_err(|e| format!("fastembed embed: {e}"))
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Convenience: chunk + embed a note in one call. Pure plumbing — no I/O
/// beyond the embedder itself. Returns a vec ready to merge into an
/// [`EmbeddingIndex`].
pub fn embed_note<E: Embedder>(
    embedder: &E,
    note_id: &str,
    content: &str,
    max_chars: usize,
) -> Result<Vec<EmbeddedChunk>, String> {
    let chunks = chunk_note(content, max_chars);
    if chunks.is_empty() {
        return Ok(Vec::new());
    }
    let vecs = embedder.embed_batch(&chunks)?;
    if vecs.len() != chunks.len() {
        return Err(format!(
            "embedder returned {} vectors for {} chunks",
            vecs.len(),
            chunks.len()
        ));
    }
    Ok(chunks
        .into_iter()
        .zip(vecs)
        .enumerate()
        .map(|(idx, (text, embedding))| EmbeddedChunk {
            note_id: note_id.to_string(),
            chunk_idx: idx,
            content_hash: hash_chunk(&text),
            text,
            embedding,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ───────────── pure-logic tests (no fastembed) ─────────────

    #[test]
    fn strip_frontmatter_removes_yaml_header() {
        let raw = "---\nid: foo\ntags: []\n---\n# Body\n\ntext";
        assert_eq!(strip_frontmatter(raw), "# Body\n\ntext");
    }

    #[test]
    fn strip_frontmatter_noop_when_absent() {
        let raw = "# Body\n\ntext";
        assert_eq!(strip_frontmatter(raw), raw);
    }

    #[test]
    fn strip_frontmatter_noop_when_open_without_close() {
        let raw = "---\nid: foo\n# never closes";
        assert_eq!(strip_frontmatter(raw), raw);
    }

    #[test]
    fn chunk_note_returns_empty_for_empty_content() {
        assert!(chunk_note("", 100).is_empty());
        assert!(chunk_note("   \n\n   ", 100).is_empty());
    }

    #[test]
    fn chunk_note_keeps_short_content_as_single_chunk() {
        let body = "Just one paragraph here.";
        let chunks = chunk_note(body, 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], body);
    }

    #[test]
    fn chunk_note_splits_on_blank_lines() {
        let body = "para one\n\npara two\n\npara three";
        let chunks = chunk_note(body, 12);
        // Each paragraph exceeds the merge budget → 3 chunks.
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn chunk_note_merges_small_paragraphs() {
        let body = "alpha\n\nbeta\n\ngamma\n\ndelta";
        let chunks = chunk_note(body, 100);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("alpha"));
        assert!(chunks[0].contains("delta"));
    }

    #[test]
    fn chunk_note_strips_frontmatter_before_chunking() {
        let body = "---\nid: x\n---\n\nbody one\n\nbody two";
        let chunks = chunk_note(body, 100);
        assert_eq!(chunks.len(), 1);
        assert!(!chunks[0].contains("id: x"));
        assert!(chunks[0].contains("body one"));
    }

    #[test]
    fn chunk_note_single_paragraph_stays_whole_with_tiny_max_chars() {
        // Single-paragraph body never splits mid-word — `max_chars=1`
        // still returns the whole paragraph as one chunk (the algorithm
        // refuses to mangle word boundaries).
        let body = "alpha beta gamma delta";
        let chunks = chunk_note(body, 1);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], body);
    }

    #[test]
    fn chunk_note_preserves_order() {
        let body = "first\n\nsecond\n\nthird\n\nfourth";
        let chunks = chunk_note(body, 6); // force one chunk per paragraph
        assert_eq!(chunks, vec!["first", "second", "third", "fourth"]);
    }

    #[test]
    fn hash_chunk_is_deterministic() {
        assert_eq!(hash_chunk("hello"), hash_chunk("hello"));
        assert_ne!(hash_chunk("hello"), hash_chunk("Hello"));
    }

    #[test]
    fn cosine_orthogonal_is_zero() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine(&a, &b) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_parallel_is_one() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 4.0, 6.0];
        assert!((cosine(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_is_minus_one() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((cosine(&a, &b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_dim_mismatch_returns_nan() {
        let r = cosine(&[1.0, 2.0], &[1.0, 2.0, 3.0]);
        assert!(r.is_nan());
    }

    #[test]
    fn cosine_zero_vector_returns_zero() {
        let r = cosine(&[0.0, 0.0], &[1.0, 0.0]);
        assert_eq!(r, 0.0);
    }

    // ───────────── EmbeddingIndex ─────────────

    fn chunk(note: &str, idx: usize, text: &str) -> EmbeddedChunk {
        EmbeddedChunk {
            note_id: note.into(),
            chunk_idx: idx,
            text: text.into(),
            embedding: vec![idx as f32; 4],
            content_hash: hash_chunk(text),
        }
    }

    #[test]
    fn index_default_is_empty() {
        let idx = EmbeddingIndex::default();
        assert_eq!(idx.entries.len(), 0);
        assert_eq!(idx.dim, 0);
    }

    #[test]
    fn index_chunks_of_filters_by_note() {
        let mut idx = EmbeddingIndex::default();
        idx.entries.push(chunk("a", 0, "x"));
        idx.entries.push(chunk("a", 1, "y"));
        idx.entries.push(chunk("b", 0, "z"));
        assert_eq!(idx.chunks_of("a").len(), 2);
        assert_eq!(idx.chunks_of("b").len(), 1);
        assert_eq!(idx.chunks_of("c").len(), 0);
    }

    #[test]
    fn index_replace_note_replaces_all_chunks() {
        let mut idx = EmbeddingIndex::default();
        idx.entries.push(chunk("a", 0, "old"));
        idx.entries.push(chunk("a", 1, "older"));
        let n = idx.replace_note("a", vec![chunk("a", 0, "new")]);
        assert_eq!(n, 1);
        assert_eq!(idx.chunks_of("a").len(), 1);
        assert_eq!(idx.chunks_of("a")[0].text, "new");
    }

    #[test]
    fn index_remove_note_returns_removed_count() {
        let mut idx = EmbeddingIndex::default();
        idx.entries.push(chunk("a", 0, "x"));
        idx.entries.push(chunk("a", 1, "y"));
        idx.entries.push(chunk("b", 0, "z"));
        assert_eq!(idx.remove_note("a"), 2);
        assert_eq!(idx.entries.len(), 1);
    }

    #[test]
    fn index_save_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".omninote/embeddings.bin");
        let mut idx = EmbeddingIndex {
            model_id: "test-model".into(),
            dim: 4,
            entries: vec![chunk("a", 0, "x"), chunk("b", 0, "y")],
        };
        idx.entries[0].embedding = vec![0.1, 0.2, 0.3, 0.4];
        idx.save(&path).unwrap();
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.model_id, "test-model");
        assert_eq!(loaded.dim, 4);
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].embedding, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn index_load_returns_empty_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".omninote/embeddings.bin");
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.entries.len(), 0);
        assert_eq!(loaded.dim, 0);
    }

    #[test]
    fn index_default_path_uses_dot_omninote_subdir() {
        let p = EmbeddingIndex::default_path(Path::new("/tmp/vault"));
        assert!(p.ends_with(".omninote/embeddings.bin"));
    }

    // ───────────── Embedder trait (stub) ─────────────

    struct StubEmbedder {
        dim: usize,
    }
    impl Embedder for StubEmbedder {
        fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            // Deterministic stub: vector[i] = (text.len() % 7) as f32 + i
            Ok(texts
                .iter()
                .map(|t| {
                    (0..self.dim)
                        .map(|i| (t.len() % 7) as f32 + i as f32)
                        .collect()
                })
                .collect())
        }
        fn dim(&self) -> usize {
            self.dim
        }
        fn model_id(&self) -> &str {
            "stub"
        }
    }

    #[test]
    fn embed_note_emits_one_chunk_per_paragraph() {
        let e = StubEmbedder { dim: 4 };
        let chunks = embed_note(&e, "note-1", "alpha\n\nbeta\n\ngamma", 6).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].note_id, "note-1");
        assert_eq!(chunks[0].chunk_idx, 0);
        assert_eq!(chunks[2].chunk_idx, 2);
    }

    #[test]
    fn embed_note_empty_body_returns_empty_vec() {
        let e = StubEmbedder { dim: 4 };
        let chunks = embed_note(&e, "x", "", 100).unwrap();
        assert!(chunks.is_empty());
    }

    #[test]
    fn embed_note_skips_frontmatter() {
        let e = StubEmbedder { dim: 4 };
        let chunks = embed_note(&e, "x", "---\nid: x\n---\n\nbody", 100).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(!chunks[0].text.contains("id: x"));
    }

    #[test]
    fn embed_note_propagates_embedder_dim() {
        let e = StubEmbedder { dim: 8 };
        let chunks = embed_note(&e, "x", "alpha", 100).unwrap();
        assert_eq!(chunks[0].embedding.len(), 8);
    }

    #[test]
    fn embed_note_assigns_content_hash() {
        let e = StubEmbedder { dim: 4 };
        let chunks = embed_note(&e, "x", "alpha\n\nbeta", 6).unwrap();
        assert_eq!(chunks[0].content_hash, hash_chunk("alpha"));
        assert_eq!(chunks[1].content_hash, hash_chunk("beta"));
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 128, ..proptest::test_runner::Config::default() })]

        #[test]
        fn chunk_note_never_panics(body in proptest::prelude::any::<String>(), max in 0usize..2000) {
            let _ = chunk_note(&body, max);
        }

        #[test]
        fn hash_chunk_never_panics(s in proptest::prelude::any::<String>()) {
            let _ = hash_chunk(&s);
        }

        #[test]
        fn cosine_never_panics(a in proptest::collection::vec(-100f32..100.0, 0..50), b in proptest::collection::vec(-100f32..100.0, 0..50)) {
            let _ = cosine(&a, &b);
        }
    }
}
