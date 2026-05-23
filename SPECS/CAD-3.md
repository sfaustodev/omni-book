# CAD-3 · 📝 Painel central com modo Ler/Editar

**Notion:** https://app.notion.com/p/35373ac79ddb81abb782c4570cdb5d62
**Phase:** v0.1 · **Status:** 🎯 Pronta · **Priority:** ⚡ Alta · **Size:** 🐎 L (3-5 dias) · **Estimate:** 10h
**Type:** ✨ Feature · **Areas:** UI

---

## 🎯 Objetivo
O painel central da app: header sticky com breadcrumb + toggle Ler/Editar, e dois modos de exibição (edição com TextEdit + leitura com CommonMark renderizado).

## 📖 Contexto
Uma vez que a sidebar deixa o usuário escolher uma nota, é aqui que ele lê e escreve. Dois modos porque editar markdown puro é diferente de ler renderizado.

## ✅ Critérios de aceite
- [x] Sem nota ativa: mensagem com atalhos (Ctrl+N pra criar, etc)
- [x] Header sticky: breadcrumb da pasta · toggle Ler/Editar · 🗑
- [x] Modo edição com `TextEdit::singleline` pro título (rename do arquivo no commit)
- [x] Group: Tipo (ComboBox), Tags (CSV), campos extra se Citação (Fonte/URL)
- [x] `TextEdit::multiline` em modo `code_editor` pro content
- [x] Botão "📎 Anexar" abre `rfd::FileDialog` e insere `![[nome]]` no fim do conteúdo
- [x] Modo leitura: título h1, tags clicáveis (preenchem query), bloco de citação
- [x] `CommonMarkViewer` renderiza content com `md_cache` persistente
- [x] **Backlinks** ao final: notas com `linked_note == note.id` ou que contêm `[[<title>]]`

## 🛠️ Especificação técnica
`egui::CentralPanel::default()` com `ScrollArea`. `editing: bool` em `OmniNoteApp`. Backlinks scan varre `vault.notes` em cada frame de view mode — em vault grande pode pesar (>1000 notas), considerar cache invalidado em save num próximo sprint.

**Crítico:** usar `TextEdit::show()` (retorna `TextEditOutput` com `cursor_range`) ao invés de `ui.add(TextEdit)` pra ter acesso ao cursor pos no `Ctrl+=` autoformat. Drop o output ANTES de mutar `note.content` (borrow checker).

## 🧪 Como testar
1. Abrir nota, alternar modo várias vezes — estado preservado
2. Editar título → wait 600ms → arquivo renomeado no disco com sanitização
3. Anexar PDF → arquivo copiado pra `_attachments/` e `![[nome.pdf]]` inserido
4. Backlinks aparecem ao final em modo leitura quando outra nota linka via `[[título]]`

## 📎 Referências
- [SPEC.md](../SPEC.md) §1.2
- Implementação: [src/ui_editor.rs](../src/ui_editor.rs) (no branch `feat/omninote-v01`)
