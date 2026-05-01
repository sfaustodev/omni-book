# CAD-5 · ⌨️ Atalhos globais

**Notion:** https://app.notion.com/p/35373ac79ddb81ed9898fae5e0bbae00
**Phase:** v0.1 · **Status:** 📝 Refinando · **Priority:** 📌 Média · **Size:** 🐭 S (≤meio dia) · **Estimate:** 2h
**Type:** ✨ Feature · **Areas:** UI

---

## 🎯 Objetivo
Atalhos de teclado pras ações mais frequentes, no estilo Apple Notes/Obsidian.

## ✅ Critérios de aceite
- [x] `Ctrl+N` → mostrar modal *Nova nota*
- [x] `Ctrl+E` → toggle Ler/Editar (se nota ativa)
- [x] `Ctrl+K` → focar campo de busca
- [x] `Ctrl+,` → mostrar settings
- [x] `Ctrl+Shift+D` → toggle dark mode
- [x] `Ctrl+=` no editor → calcula linha atual via `autoformat::try_math_substitute`
- [ ] **PENDENTE Q-02:** trocar pra `Modifiers::COMMAND` pra mapear Cmd↔Ctrl automaticamente em macOS

## 🛠️ Especificação técnica
`ctx.input(|i| ...)` no top do `update()`, retornando tuple de bools. Importante ler todos shortcuts numa única call pra evitar borrow conflict subsequente.

Pra `Ctrl+=` o cursor pos vem de `output.cursor_range` (precisa `TextEdit::show()`, não `ui.add`).

## 💭 Notas
No macOS o atalho convencional é `Cmd+...` — vale considerar usar `Modifiers::COMMAND` que mapeia automaticamente Cmd no mac e Ctrl no resto. **Pergunta aberta no [HUMAN.md Q-02](../HUMAN.md).**

## 🧪 Como testar
1. Abrir app → Ctrl+N → modal aparece
2. Selecionar nota → Ctrl+E → muda entre view e edit
3. Foco em qualquer lugar → Ctrl+K → busca da sidebar fica focada
4. Ctrl+, → settings modal
5. Ctrl+Shift+D → tema alterna
6. No editor, digite `2 + 3 =` → Ctrl+= → vira `2 + 3 = 5`

## 📎 Referências
- [SPEC.md](../SPEC.md) §1.4
- Implementação: [src/app.rs::update](../src/app.rs)
