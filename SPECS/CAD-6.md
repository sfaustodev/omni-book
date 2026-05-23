# CAD-6 · 🐛 Bug: `from_id_source` renomeado em egui 0.29 quebra build

**Notion:** https://app.notion.com/p/35373ac79ddb813dab5cd55f0b8da8f5
**Phase:** v0.1 · **Status:** ✅ Concluída · **Priority:** 🔥 Crítica · **Size:** 🐜 XS (≤2h) · **Estimate:** 0.5h
**Type:** 🐛 Bug · **Areas:** Build

---

## 🎯 Objetivo
O `cargo build` quebra com erro de método não encontrado: `from_id_source` foi renomeado pra `from_id_salt` em egui 0.29.

## 🛠️ Solução aplicada
Find/replace no projeto inteiro: `from_id_source` → `from_id_salt`. Já aplicado em todo `src/ui_*.rs` durante o desenvolvimento de v0.1.

## 🧪 Como testar
1. `cargo clean && cargo build` → 0 erros
2. Abrir o app → árvore de pastas usa `CollapsingHeader` corretamente

## 📎 Referências
- Changelog egui 0.29: https://github.com/emilk/egui/blob/master/CHANGELOG.md
- [SPEC.md](../SPEC.md) §"Notas pra Claude Code"
