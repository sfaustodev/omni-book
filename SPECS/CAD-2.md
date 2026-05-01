# CAD-2 · 🗂️ Sidebar com árvore de pastas e busca

**Notion:** https://app.notion.com/p/35373ac79ddb81cc886ad4b13c94e3b9
**Phase:** v0.1 · **Status:** 🚧 Em obra · **Priority:** ⚡ Alta · **Size:** 🐎 L (3-5 dias) · **Estimate:** 8h
**Type:** ✨ Feature · **Areas:** UI, Vault

---

## 🎯 Objetivo
Montar a `SidePanel::left` de 280px com header, busca, árvore de pastas/notas recursiva e footer de ações — substituindo o TODO atual em `app.rs`.

## 📖 Contexto
A sidebar é a porta de entrada do OmniNote. Sem ela, o usuário não navega entre notas nem cria pastas. Bloqueante pra v0.1 inteiro.

## ✅ Critérios de aceite
- [x] Header mostra "📓 OmniNote" + nome do vault em fonte menor
- [x] Ícones right-aligned: trocar vault (📂), tema (☀/🌙), config (⚙)
- [x] Busca com hint "🔍 Buscar..." recebe foco em `Ctrl+K`
- [x] Filtro horizontal: "todos" + 6 chips clicáveis (um por NoteType)
- [x] `ScrollArea` vertical com árvore recursiva via `vault.list_folders()`
- [x] Estado de expand/collapse (egui persiste via `id_salt`)
- [x] Right-click em pasta exibe menu: Nova nota / Deletar pasta
- [x] Notas exibem `note.frontmatter.note_type.icon() + note.title`
- [x] Notas raiz (sem folder) listadas no fim, mesmo nível
- [x] Footer com três botões: ➕ Nota / 📁 Pasta / 📥 Importar

## 🛠️ Especificação técnica
`egui::SidePanel::left("sidebar").exact_width(280.0)`. Estado em `OmniNoteApp.query`, `OmniNoteApp.type_filter`. Recursão pelas pastas agrupa por `parent` e renderiza com `CollapsingHeader::new(name).id_salt(format!("folder_{}", path))`. Notas filtradas por query (title + content) e type_filter.

**Padrão crítico:** `show_notes_in_folder` coleta `Vec<(id, label)>` antes de renderizar pra evitar borrow conflict entre `&self.vault` (iter) e `&mut self` (select_note).

## 🧪 Como testar
1. Criar vault novo com 3 pastas aninhadas e notas em cada nível
2. Verificar expand/collapse mantém estado entre frames
3. `Ctrl+K` foca a busca
4. Right-click em pasta: confirmar menu aparece e ações funcionam

## 📎 Referências
- [SPEC.md](../SPEC.md) §1.1
- egui SidePanel: https://docs.rs/egui/latest/egui/struct.SidePanel.html
- Implementação: [src/ui_sidebar.rs](../src/ui_sidebar.rs) (no branch `feat/omninote-v01`)
