# CAD-23 — Phase 4 — AI-native vault (RAG + auto-tag + dictation + OCR)

**Notion:** [https://www.notion.so/36673ac79ddb81d9b1a9f1df14a8fc9d](https://www.notion.so/36673ac79ddb81d9b1a9f1df14a8fc9d)
**Sprint:** v1.3 (2026-06-17 → 2026-07-01)
**Depende:** CAD-21 done
**Critical files:** new crate `omninote-ai/` (ou feature flag), `.omninote/llm.toml`, `.omninote/embeddings.bin`

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

- [ ] LLM provider abstraction — trait `LlmProvider` + impls Claude/Grok/Ollama
- [ ] Config `.omninote/llm.toml`
- [ ] RAG: local embeddings via `fastembed-rs` ou `ort` (ONNX) → vector store `.omninote/embeddings.bin`
- [ ] CLI `omninote ask "query"` → top-k passages com `[[wikilinks]]`
- [ ] MCP tool `vault_search_semantic`
- [ ] Auto-tag/auto-summary: CLI `omninote tag --auto FILE` + diff modal before write
- [ ] Dictation: `omninote dictate` → mic → Whisper local (`whisper-rs`) → nova nota pré-preenchida
- [ ] OCR: `omninote ocr FILE.pdf` → tesseract ou remote API → companion `.md`
- [ ] Embeddings cache invalidado quando arquivo muda (via watcher)


## Verification

`omninote ask "escrow HMAC"` sobre `~/Documents/Obsidian Vault` retorna `[[SPEC_V2 - NdA]]` no top-3. Auto-tag round-trip preserva YAML. Dictation 30s pt-BR WER < 10%. OCR PDF scanned legível.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb81d9b1a9f1df14a8fc9d
