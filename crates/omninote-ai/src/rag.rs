//! RAG retrieval over the vault. CAD-23.1 Phase C.
//!
//! [`Rag`] couples a vault-wide [`EmbeddingIndex`] with a backend
//! [`Embedder`] and exposes three operations:
//!
//! - [`Rag::build_index_from_notes`] — full rebuild from a slice of notes
//! - [`Rag::upsert_note`] — incremental refresh for one note (watcher hook)
//! - [`Rag::retrieve`] — cosine top-k search returning [`RagHit`]s
//!
//! Each `RagHit` carries the source note id, chunk text, and cosine score.
//! The CLI verb `omninote ask` formats hits with `[[wikilink]]` citations
//! by mapping `note_id` back through `Vault::notes` (Phase E).

use crate::embeddings::{
    chunk_note, cosine, embed_note, hash_chunk, EmbeddedChunk, Embedder, EmbeddingIndex,
};
use omninote_core::types::Note;

/// Default chunk size in chars (~500 tokens for prose). Configurable via
/// [`RagOpts::max_chunk_chars`].
pub const DEFAULT_MAX_CHUNK_CHARS: usize = 2000;

/// Tunable knobs for [`Rag`].
#[derive(Clone, Debug)]
pub struct RagOpts {
    /// Max chars per chunk fed to the embedder. Default 2000 ≈ 500 tokens.
    pub max_chunk_chars: usize,
    /// If `true`, [`Rag::upsert_note`] skips re-embedding unchanged chunks
    /// (compared by `content_hash`). Default `true`.
    pub skip_unchanged_chunks: bool,
}

impl Default for RagOpts {
    fn default() -> Self {
        Self {
            max_chunk_chars: DEFAULT_MAX_CHUNK_CHARS,
            skip_unchanged_chunks: true,
        }
    }
}

/// One retrieval hit. Score is cosine similarity in `[-1, 1]` (higher
/// is more similar).
#[derive(Clone, Debug, PartialEq)]
pub struct RagHit {
    pub note_id: String,
    pub chunk_idx: usize,
    pub chunk_text: String,
    pub score: f32,
}

/// The RAG facade. Owns the embedder + the on-disk index.
pub struct Rag<E: Embedder> {
    pub index: EmbeddingIndex,
    pub embedder: E,
    pub opts: RagOpts,
}

impl<E: Embedder> Rag<E> {
    /// Construct with an empty index. Caller should subsequently call
    /// [`Self::build_index_from_notes`] or load via
    /// [`EmbeddingIndex::load`] + [`Self::with_index`].
    pub fn new(embedder: E) -> Self {
        let mut index = EmbeddingIndex::default();
        index.model_id = embedder.model_id().to_string();
        index.dim = embedder.dim();
        Self {
            index,
            embedder,
            opts: RagOpts::default(),
        }
    }

    /// Construct around an existing index (e.g. loaded from
    /// `.omninote/embeddings.bin`). If the embedder's `model_id` doesn't
    /// match the index, the index is reset — different embedding models
    /// produce incompatible vectors.
    pub fn with_index(embedder: E, mut index: EmbeddingIndex) -> Self {
        if index.model_id != embedder.model_id() || index.dim != embedder.dim() {
            index = EmbeddingIndex {
                model_id: embedder.model_id().to_string(),
                dim: embedder.dim(),
                entries: Vec::new(),
            };
        }
        Self {
            index,
            embedder,
            opts: RagOpts::default(),
        }
    }

    /// Use the given options.
    pub fn with_opts(mut self, opts: RagOpts) -> Self {
        self.opts = opts;
        self
    }

    /// Embed every note in `notes` from scratch, replacing the in-memory
    /// index. Returns the number of chunks embedded.
    pub fn build_index_from_notes(&mut self, notes: &[Note]) -> Result<usize, String> {
        // Reset.
        self.index.entries.clear();
        self.index.model_id = self.embedder.model_id().to_string();
        self.index.dim = self.embedder.dim();

        let mut total = 0;
        for note in notes {
            let chunks = embed_note(
                &self.embedder,
                &note.frontmatter.id,
                &note.content,
                self.opts.max_chunk_chars,
            )?;
            total += chunks.len();
            self.index.entries.extend(chunks);
        }
        Ok(total)
    }

    /// Refresh a single note. Returns the count of chunks embedded (may
    /// be 0 if all chunks were skip-unchanged hits).
    pub fn upsert_note(&mut self, note_id: &str, content: &str) -> Result<usize, String> {
        let new_chunks = chunk_note(content, self.opts.max_chunk_chars);
        if new_chunks.is_empty() {
            // Note became empty — drop any existing chunks.
            self.index.remove_note(note_id);
            return Ok(0);
        }

        if !self.opts.skip_unchanged_chunks {
            // Cheap path: re-embed everything.
            let chunks = embed_note(&self.embedder, note_id, content, self.opts.max_chunk_chars)?;
            let n = chunks.len();
            self.index.replace_note(note_id, chunks);
            return Ok(n);
        }

        // Diff path: embed only the chunks whose hash changed.
        let old: std::collections::HashMap<usize, &EmbeddedChunk> = self
            .index
            .entries
            .iter()
            .filter(|c| c.note_id == note_id)
            .map(|c| (c.chunk_idx, c))
            .collect();

        let mut to_embed: Vec<(usize, String)> = Vec::new();
        let mut kept: Vec<EmbeddedChunk> = Vec::new();
        for (idx, text) in new_chunks.into_iter().enumerate() {
            let h = hash_chunk(&text);
            match old.get(&idx) {
                Some(prev) if prev.content_hash == h => {
                    let mut c = (*prev).clone();
                    // text identical, embedding identical — but text might
                    // technically differ at edges (trim); keep authoritative.
                    c.text = text;
                    kept.push(c);
                }
                _ => to_embed.push((idx, text)),
            }
        }

        let embedded_count = to_embed.len();
        if !to_embed.is_empty() {
            let texts: Vec<String> = to_embed.iter().map(|(_, t)| t.clone()).collect();
            let vecs = self.embedder.embed_batch(&texts)?;
            if vecs.len() != texts.len() {
                return Err(format!(
                    "embedder returned {} vectors for {} chunks",
                    vecs.len(),
                    texts.len()
                ));
            }
            for ((idx, text), embedding) in to_embed.into_iter().zip(vecs) {
                kept.push(EmbeddedChunk {
                    note_id: note_id.to_string(),
                    chunk_idx: idx,
                    content_hash: hash_chunk(&text),
                    text,
                    embedding,
                });
            }
        }

        // Sort by chunk_idx so retrieve() returns hits in deterministic order
        // when scores tie.
        kept.sort_by_key(|c| c.chunk_idx);
        self.index.replace_note(note_id, kept);
        Ok(embedded_count)
    }

    /// Drop a note's chunks (e.g. file deleted). Returns count dropped.
    pub fn forget_note(&mut self, note_id: &str) -> usize {
        self.index.remove_note(note_id)
    }

    /// Cosine top-k. Empty query → empty result. `top_k=0` → empty.
    pub fn retrieve(&self, query: &str, top_k: usize) -> Result<Vec<RagHit>, String> {
        let q = query.trim();
        if q.is_empty() || top_k == 0 || self.index.entries.is_empty() {
            return Ok(Vec::new());
        }
        let qvec = self.embedder.embed_batch(&[q.to_string()])?;
        let qvec = qvec
            .into_iter()
            .next()
            .ok_or_else(|| "embedder returned zero vectors for query".to_string())?;
        if qvec.len() != self.index.dim {
            return Err(format!(
                "query embedding dim {} does not match index dim {}",
                qvec.len(),
                self.index.dim
            ));
        }

        // Score every entry, then partial-sort.
        let mut scored: Vec<(f32, &EmbeddedChunk)> = self
            .index
            .entries
            .iter()
            .map(|c| (cosine(&qvec, &c.embedding), c))
            .filter(|(s, _)| !s.is_nan())
            .collect();

        // Descending by score; tiebreak by note_id+chunk_idx for determinism.
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.note_id.cmp(&b.1.note_id))
                .then_with(|| a.1.chunk_idx.cmp(&b.1.chunk_idx))
        });

        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, c)| RagHit {
                note_id: c.note_id.clone(),
                chunk_idx: c.chunk_idx,
                chunk_text: c.text.clone(),
                score,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omninote_core::types::{Frontmatter, NoteType};
    use std::path::PathBuf;

    /// Deterministic stub embedder. Maps each character to a "bucket" of
    /// dimensions, producing meaningfully-different vectors for different
    /// content while staying entirely synthetic (no model download).
    struct StubEmbedder {
        dim: usize,
    }
    impl Embedder for StubEmbedder {
        fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            Ok(texts.iter().map(|t| stub_embed(t, self.dim)).collect())
        }
        fn dim(&self) -> usize {
            self.dim
        }
        fn model_id(&self) -> &str {
            "stub-v1"
        }
    }

    fn stub_embed(text: &str, dim: usize) -> Vec<f32> {
        // Distribute characters across `dim` buckets and normalise to unit
        // length so cosine similarity is well-defined.
        let mut v = vec![0f32; dim];
        for (i, b) in text.bytes().enumerate() {
            v[i % dim] += (b as f32) * 0.01;
        }
        let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            for x in v.iter_mut() {
                *x /= n;
            }
        }
        v
    }

    fn mk_note(id: &str, title: &str, body: &str) -> Note {
        Note {
            path: PathBuf::from(format!("/tmp/{title}.md")),
            rel_path: PathBuf::from(format!("{title}.md")),
            frontmatter: Frontmatter {
                id: id.into(),
                note_type: NoteType::Resumo,
                tags: vec![],
                source: String::new(),
                source_link: String::new(),
                linked_note: None,
                attachments: vec![],
                created: String::new(),
                aliases: vec![],
            },
            title: title.into(),
            content: body.into(),
        }
    }

    // ──────────────── construction ────────────────

    #[test]
    fn new_starts_with_empty_index_tagged_with_model_metadata() {
        let r = Rag::new(StubEmbedder { dim: 8 });
        assert_eq!(r.index.entries.len(), 0);
        assert_eq!(r.index.model_id, "stub-v1");
        assert_eq!(r.index.dim, 8);
    }

    #[test]
    fn with_index_keeps_compatible_index() {
        let mut idx = EmbeddingIndex {
            model_id: "stub-v1".into(),
            dim: 8,
            entries: vec![EmbeddedChunk {
                note_id: "n1".into(),
                chunk_idx: 0,
                text: "x".into(),
                embedding: vec![0.1; 8],
                content_hash: 0,
            }],
        };
        let backup_entries = idx.entries.clone();
        let r = Rag::with_index(StubEmbedder { dim: 8 }, idx.clone());
        assert_eq!(r.index.entries.len(), 1);
        assert_eq!(r.index.entries, backup_entries);
    }

    #[test]
    fn with_index_resets_index_when_model_changed() {
        let stale = EmbeddingIndex {
            model_id: "old-model".into(),
            dim: 8,
            entries: vec![EmbeddedChunk {
                note_id: "n1".into(),
                chunk_idx: 0,
                text: "x".into(),
                embedding: vec![0.1; 8],
                content_hash: 0,
            }],
        };
        let r = Rag::with_index(StubEmbedder { dim: 8 }, stale);
        assert_eq!(r.index.entries.len(), 0, "stale index must be reset");
        assert_eq!(r.index.model_id, "stub-v1");
    }

    #[test]
    fn with_index_resets_index_when_dim_changed() {
        let stale = EmbeddingIndex {
            model_id: "stub-v1".into(),
            dim: 4,
            entries: vec![EmbeddedChunk {
                note_id: "n1".into(),
                chunk_idx: 0,
                text: "x".into(),
                embedding: vec![0.1; 4],
                content_hash: 0,
            }],
        };
        let r = Rag::with_index(StubEmbedder { dim: 8 }, stale);
        assert_eq!(r.index.entries.len(), 0);
        assert_eq!(r.index.dim, 8);
    }

    // ──────────────── build_index_from_notes ────────────────

    #[test]
    fn build_index_embeds_every_note() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        let notes = vec![
            mk_note("n1", "Alpha", "first para\n\nsecond para"),
            mk_note("n2", "Beta", "single para"),
        ];
        let n = r
            .build_index_from_notes(&notes)
            .expect("build should succeed");
        // n1 might split into 2 chunks if merge budget exceeded; with default
        // 2000 chars budget, they merge into 1. So n1=1, n2=1.
        assert_eq!(n, 2);
        assert_eq!(r.index.entries.len(), 2);
    }

    #[test]
    fn build_index_skips_empty_notes() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        let notes = vec![mk_note("n1", "Empty", ""), mk_note("n2", "Has", "body")];
        let n = r.build_index_from_notes(&notes).unwrap();
        assert_eq!(n, 1);
        assert_eq!(r.index.entries[0].note_id, "n2");
    }

    #[test]
    fn build_index_clears_previous_entries() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.build_index_from_notes(&[mk_note("old", "Old", "stale")])
            .unwrap();
        r.build_index_from_notes(&[mk_note("new", "New", "fresh")])
            .unwrap();
        assert_eq!(r.index.entries.len(), 1);
        assert_eq!(r.index.entries[0].note_id, "new");
    }

    // ──────────────── upsert_note ────────────────

    #[test]
    fn upsert_note_adds_new_note() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        let n = r.upsert_note("n1", "first\n\nsecond").unwrap();
        assert_eq!(n, 1); // both paragraphs merge into one chunk with default budget
        assert_eq!(r.index.chunks_of("n1").len(), 1);
    }

    #[test]
    fn upsert_note_skips_unchanged_chunks() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "unchanged body").unwrap();
        let embedded_count = r.upsert_note("n1", "unchanged body").unwrap();
        assert_eq!(embedded_count, 0, "identical content must skip re-embed");
    }

    #[test]
    fn upsert_note_replaces_when_content_changed() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "version one").unwrap();
        let v1_embedding = r.index.chunks_of("n1")[0].embedding.clone();
        r.upsert_note("n1", "version two completely different")
            .unwrap();
        let v2_embedding = r.index.chunks_of("n1")[0].embedding.clone();
        assert_ne!(v1_embedding, v2_embedding);
    }

    #[test]
    fn upsert_note_clears_when_content_becomes_empty() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "body").unwrap();
        assert_eq!(r.index.chunks_of("n1").len(), 1);
        r.upsert_note("n1", "").unwrap();
        assert_eq!(r.index.chunks_of("n1").len(), 0);
    }

    #[test]
    fn upsert_note_disabled_skip_path_reembeds_everything() {
        let mut r = Rag::new(StubEmbedder { dim: 8 }).with_opts(RagOpts {
            max_chunk_chars: 2000,
            skip_unchanged_chunks: false,
        });
        r.upsert_note("n1", "alpha").unwrap();
        let n = r.upsert_note("n1", "alpha").unwrap();
        assert_eq!(n, 1, "skip disabled means we re-embed everything");
    }

    // ──────────────── forget_note ────────────────

    #[test]
    fn forget_note_drops_all_chunks_for_id() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("keep", "keep me").unwrap();
        r.upsert_note("drop", "drop me").unwrap();
        let n = r.forget_note("drop");
        assert_eq!(n, 1);
        assert_eq!(r.index.chunks_of("drop").len(), 0);
        assert_eq!(r.index.chunks_of("keep").len(), 1);
    }

    // ──────────────── retrieve ────────────────

    #[test]
    fn retrieve_empty_query_returns_empty() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "anything").unwrap();
        assert!(r.retrieve("", 5).unwrap().is_empty());
        assert!(r.retrieve("  ", 5).unwrap().is_empty());
    }

    #[test]
    fn retrieve_top_k_zero_returns_empty() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "body").unwrap();
        assert!(r.retrieve("anything", 0).unwrap().is_empty());
    }

    #[test]
    fn retrieve_empty_index_returns_empty() {
        let r = Rag::new(StubEmbedder { dim: 8 });
        assert!(r.retrieve("query", 5).unwrap().is_empty());
    }

    #[test]
    fn retrieve_finds_relevant_note() {
        let mut r = Rag::new(StubEmbedder { dim: 32 });
        r.upsert_note("noise", "completely unrelated mathematics topic")
            .unwrap();
        r.upsert_note(
            "target",
            "the quick brown fox jumps over the lazy dog repeatedly",
        )
        .unwrap();
        r.upsert_note("filler", "another paragraph with different words")
            .unwrap();
        let hits = r.retrieve("quick brown fox", 3).unwrap();
        assert!(!hits.is_empty());
        // Stub embedder isn't semantically smart but should still rank the
        // target highest because its char distribution overlaps most with
        // the query.
        assert_eq!(
            hits[0].note_id, "target",
            "target should be the top hit, got: {:?}",
            hits
        );
    }

    #[test]
    fn retrieve_respects_top_k() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        for i in 0..5 {
            r.upsert_note(&format!("n{i}"), &format!("body {i}"))
                .unwrap();
        }
        assert_eq!(r.retrieve("body", 3).unwrap().len(), 3);
        assert_eq!(r.retrieve("body", 100).unwrap().len(), 5);
    }

    #[test]
    fn retrieve_returns_chunk_text() {
        let mut r = Rag::new(StubEmbedder { dim: 8 });
        r.upsert_note("n1", "exact retrievable text").unwrap();
        let hits = r.retrieve("retrievable", 1).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].chunk_text.contains("retrievable"));
    }

    #[test]
    fn retrieve_orders_descending_by_score() {
        let mut r = Rag::new(StubEmbedder { dim: 16 });
        r.upsert_note("a", "alpha alpha alpha").unwrap();
        r.upsert_note("b", "beta beta beta").unwrap();
        r.upsert_note("c", "gamma gamma gamma").unwrap();
        let hits = r.retrieve("alpha", 3).unwrap();
        assert_eq!(hits.len(), 3);
        for w in hits.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "hits must be ordered desc: {:?}",
                hits
            );
        }
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config { cases: 64, ..proptest::test_runner::Config::default() })]

        #[test]
        fn retrieve_never_panics(query in proptest::prelude::any::<String>(), top_k in 0usize..20) {
            let mut r = Rag::new(StubEmbedder { dim: 8 });
            r.upsert_note("n1", "anything").ok();
            let _ = r.retrieve(&query, top_k);
        }

        #[test]
        fn upsert_never_panics(body in proptest::prelude::any::<String>()) {
            let mut r = Rag::new(StubEmbedder { dim: 8 });
            let _ = r.upsert_note("n", &body);
        }
    }
}
