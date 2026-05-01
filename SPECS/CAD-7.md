# CAD-7 · 🆕 Modal Nova Nota com grid 2×3 de tipos

**Notion:** https://app.notion.com/p/35373ac79ddb8174a434ffcacef323f7
**Phase:** v0.2 · **Status:** 🌱 Backlog · **Priority:** 📌 Média · **Size:** 🐈 M (1-2 dias) · **Estimate:** 4h
**Type:** ✨ Feature · **Areas:** Modais, UI

---

## 🎯 Objetivo
Modal que aparece em `Ctrl+N` mostrando 6 botões (um por NoteType). Click → `vault.create_note(folder, "", tipo)`, ativa, entra em edit.

## ✅ Critérios de aceite
- [x] Grid 2×3 com 6 botões (um por variante de `NoteType`)
- [x] Cada botão mostra ícone + nome do tipo
- [x] Click cria nota vazia na pasta raiz (TODO: pasta selecionada via context)
- [x] Nota recém-criada vira a nota ativa
- [x] Modo edição ligado automaticamente
- [x] `flush_active()` chamado antes pra não perder edits pendentes
- [ ] Cores via `Color32` correspondentes ao `NoteType.color()` — não implementado, opcional pra polish

## 🛠️ Especificação técnica
`egui::Window::new("Nova Nota").collapsible(false).resizable(false).anchor(CENTER_CENTER)`. Grid via `egui::Grid::new("note_type_grid").num_columns(3)`.

## 🧪 Como testar
1. Ctrl+N → modal aparece centralizado
2. Click em "💬 Citação" → nota criada com `note_type = Citacao`, frontmatter populado, modo edit ligado
3. Esc fecha modal sem criar nada
4. Editar enquanto modal aberto → flush salva trabalho anterior antes de criar nova

## 📎 Referências
- [SPEC.md](../SPEC.md) §2.1
- Implementação: [src/ui_modals.rs::show_modal_new](../src/ui_modals.rs)
