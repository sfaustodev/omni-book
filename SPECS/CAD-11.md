# CAD-11 · 📖 Doc: README com build cross-platform e integração MCP

**Notion:** https://app.notion.com/p/35373ac79ddb810fa601c4c3acf9ef9c
**Phase:** v1.0 · **Status:** 🌱 Backlog · **Priority:** 🌿 Baixa · **Size:** 🐭 S (≤meio dia) · **Estimate:** 2h
**Type:** 📚 Doc · **Areas:** Docs, Build

---

## 🎯 Objetivo
README.md decente pro projeto, com instruções de build pros três sistemas operacionais.

## ✅ Critérios de aceite
- [ ] Seção "O que é o OmniNote" em 1 parágrafo
- [ ] Screenshot principal (ou GIF curto da app rodando)
- [ ] **Build no Linux:** `cargo build --release` → `target/release/omninote`
  - Listar deps de sistema: `libgtk-3-dev libxcb-* libxkbcommon-dev libssl-dev`
- [ ] **Build no macOS:** `cargo bundle` (precisa `cargo-bundle`) gera `.app`
- [ ] **Build no Windows:** `cargo build --release` → `target/release/omninote.exe`
- [ ] Seção "Como integrar com Obsidian e Claude Desktop" (apontar MCP filesystem pro vault root)
- [ ] Lista de atalhos de teclado (referenciar SPEC ou CAD-5)
- [ ] Licença (MIT) e créditos (Juan Fausto + Claude)

## 💭 Notas
De preferência incluir um `assets/screenshot.png` versionado no repo. GIF é melhor mas pesa mais (>500KB).

Considerar também um `assets/demo.gif` curto (10s) mostrando: criar nota → escrever → switch view mode → import PDF.

## 📎 Referências
- [SPEC.md](../SPEC.md) §11
- README atual: [README.md](../README.md) (versão antiga, precisa rebrand)
