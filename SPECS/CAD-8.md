# CAD-8 · 📥 Modal Importar (PDF, JSON Claude, Artefato)

**Notion:** https://app.notion.com/p/35373ac79ddb81deba62f250c09fdade
**Phase:** v0.3 · **Status:** 🌱 Backlog · **Priority:** 📌 Média · **Size:** 🐈 M (1-2 dias) · **Estimate:** 6h
**Type:** ✨ Feature · **Areas:** Modais, Importação, PDF

---

## 🎯 Objetivo
Um modal com três caminhos de importação: PDF, chat Claude (JSON) e artefato Claude.

## ✅ Critérios de aceite
- [x] **📄 PDF** → FileDialog `.pdf` → `pdf::extract_text` → nota com markdown extraído + PDF copiado pra `_attachments` + entrada em `frontmatter.attachments`
- [x] **🤖 Chat Claude (JSON)** → FileDialog `.json` → `import::import_claude_chat` → cria nota
- [x] **📦 Artefato Claude** → FileDialog → `import::import_claude_artifact` → cria nota
- [x] Erros mostrados em `error_msg` modal
- [x] Nota recém-importada vira ativa
- [x] Title da nota = filename sem extensão

## 🛠️ Especificação técnica
`rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file()`. É blocking, mas tudo bem pro caso de uso.

3 helpers em `OmniNoteApp`: `import_pdf`, `import_chat`, `import_artifact`. Cada um:
1. Lê arquivo
2. Cria nota via `vault.create_note(...)`
3. Substitui content
4. Salva
5. Atualiza vault.notes
6. Define como ativa

## 💭 Notas
`lopdf::extract_text` retorna texto cru. Se layout fiel for problema, considerar trocar por `pdf-extract` crate em futura iteração.

## 🧪 Como testar
1. 📥 Importar → 📄 PDF → escolher PDF → nota criada com `## Página N` headings
2. PDF copiado pra `<vault>/_attachments/<filename>.pdf`
3. 📥 Importar → 🤖 Chat Claude → JSON exportado de claude.ai → nota com `**Você:**` / `**Claude:**` formatado
4. 📥 Importar → 📦 Artefato → arquivo `.rs` ou `.py` → nota tipo Codigo com fenced code block

## 📎 Referências
- [SPEC.md](../SPEC.md) §2.2
- Implementação: [src/ui_modals.rs](../src/ui_modals.rs), [src/import.rs](../src/import.rs), [src/pdf.rs](../src/pdf.rs)
