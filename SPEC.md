# Caderno — Especificação Técnica

> Caderno digital pessoal estilo Apple Notes/Obsidian em Rust nativo (egui).
> Vault de arquivos `.md` em disco, 100% compatível com Obsidian e Claude Desktop (via MCP filesystem).

## Estado atual

✅ **Implementado:**
- Estrutura de projeto Rust (eframe + egui 0.29)
- Sistema de Vault com arquivos `.md` + frontmatter YAML em disco
- Tipos: `Note`, `Folder`, `Frontmatter`, `AppConfig`, `NoteType` (6 variantes)
- CRUD de notas e pastas direto no filesystem (`vault.rs`)
- Parser de frontmatter compatível com Obsidian
- Persistência de config em `<vault>/.caderno/config.json`
- Picker de vault na primeira abertura + memória do último vault em `~/.config/caderno/last_vault`
- Avaliação matemática segura (`autoformat.rs`)
- Extração de texto de PDF (`pdf.rs` usando `lopdf`)
- Importação de chats Claude exportados em JSON e artefatos (`import.rs`)
- Sanitização de nomes de arquivo
- Importação de anexos pra `_attachments/`

❌ **Falta implementar (esta spec é o roteiro):**

## 1. UI principal — substituir TODO em `app.rs`

### 1.1 Sidebar (SidePanel::left, 280px)

**Header:**
- Título "📓 Caderno" + nome do vault em fonte menor abaixo
- Ícones (right-aligned): trocar vault (📂), tema (☀/🌙), config (⚙)

**Busca:**
- TextEdit com hint "🔍 Buscar..." (Ctrl+K dá foco)
- Filtro horizontal: "todos" + 6 tipos (chips clicáveis)

**Árvore:**
- ScrollArea vertical
- Recursivo via `vault.list_folders()` agrupando por `parent`
- Botão expand/collapse por pasta (estado em `HashMap<PathBuf, bool>`)
- Hover em pasta mostra 4 ícones: 📄+ (nova nota), 📁+ (subpasta), ✎ (renomear inline), 🗑 (deletar com confirmação)
- Notas exibidas com `note.frontmatter.note_type.icon() + note.title`
- Notas raiz (sem folder) listadas no final, mesmo nível

**Footer:** dois botões (➕ Nota / 📁 Pasta)

### 1.2 Painel central

- **Sem nota ativa:** mensagem com atalhos
- **Com nota ativa:**
  - Header sticky: breadcrumb da pasta · toggle Ler/Editar · 🗑
  - Modo edição:
    - TextEdit::singleline pro título (rename do arquivo no commit)
    - Group: Tipo (ComboBox), Pasta (ComboBox de `vault.list_folders()`), Tags (string CSV), Source/URL/Linked (se Citação)
    - TextEdit::multiline (code_editor) pro content
    - Botão "📎 Anexar" abre `rfd::FileDialog`, chama `vault.import_attachment()`, insere `![[nome]]` no cursor
  - Modo leitura:
    - Título h1
    - Tags clicáveis (preenche query)
    - Bloco de citação (se aplicável)
    - `CommonMarkViewer` renderiza content
    - **Backlinks** ao final (notas com `linked_note == note.id` ou que contêm `[[<title>]]` no content)

### 1.3 Auto-save

- `dirty: bool` setado em qualquer mudança de note
- Em `update()`: se dirty e `last_save.elapsed() > 600ms`, chama `vault.save_note(&note)`
- Em `on_exit()`: força save final + `vault.save_config()`

### 1.4 Atalhos globais

- `Ctrl+N` → mostrar modal new
- `Ctrl+E` → toggle editing (se nota ativa)
- `Ctrl+K` → focar busca
- `Ctrl+,` → mostrar settings
- `Ctrl+Shift+D` → toggle dark
- `Ctrl+=` no editor → calcula linha atual via `autoformat::try_math_substitute`

## 2. Modais

### 2.1 Modal Nova Nota
Grid 2×3 com 6 botões coloridos (um por NoteType). Click → `vault.create_note(folder, "", tipo)`, ativa, entra em edit.

### 2.2 Modal Importar
Três botões verticais:
- **📄 PDF** → FileDialog `.pdf` → `pdf::extract_text` → cria nota com markdown extraído + copia PDF pra `_attachments` + adiciona a `frontmatter.attachments`
- **🤖 Chat Claude (JSON)** → FileDialog `.json` → `import::import_claude_chat` → cria nota
- **📦 Artefato Claude (código/html)** → FileDialog → `import::import_claude_artifact` → cria nota

### 2.3 Modal Settings
- Checkbox "Modo escuro" (sincroniza com `vault.config.dark_mode` + `ctx.set_visuals`)
- Texto: caminho do vault atual
- Botão "Trocar vault" → `pick_vault()`

### 2.4 Modal Confirmação
Genérico via `Option<(msg, action_enum)>`. Ações: `DeleteNote(idx)`, `DeleteFolder(rel_path)`.

## 3. Renderização de PDF inline (v2)

Quando o markdown contém `![[arquivo.pdf]]`:
- Detectar via regex no `MdRenderer` custom
- Mostrar botão "📄 Abrir <nome>.pdf" + preview de 1ª página
- Click abre com app default: `open::that(path).ok()`

Pra v1, basta mostrar como link clicável.

## 4. Watcher de filesystem (v2)

Usar `notify` crate pra detectar mudanças externas no vault (Obsidian editou). Em `app.rs`:

```rust
use notify::{Watcher, RecursiveMode, Event};
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(&vault.root, RecursiveMode::Recursive)?;
// no update(): drenar rx, se houve mudança, chamar vault.reload_notes()
```

## 5. Acessibilidade (v2)

Adicionar em `AppConfig`:
- `font_family`: "system" | "atkinson" | "lexend" | "opendyslexic"
- `font_size`: f32 (14.0–24.0)
- `letter_spacing`: f32 (0.0–0.2)
- `line_height`: f32 (1.4–2.2)

Aplicar em `cc.egui_ctx`:
```rust
let mut style = (*ctx.style()).clone();
style.text_styles.iter_mut().for_each(|(_, font)| { font.size = config.font_size; });
ctx.set_style(style);
```

Pra fontes customizadas: baixar `.ttf` de `Atkinson-Hyperlegible-Regular.ttf` e `OpenDyslexic-Regular.otf`, embutir com `include_bytes!`, registrar via `egui::FontDefinitions::families`.

## 6. Wikilinks clicáveis (v2)

`egui_commonmark` não dá hook em links. Trocar por parser custom usando `pulldown-cmark`:

```rust
use pulldown_cmark::{Parser, Event, Tag};
let parser = Parser::new(content);
for event in parser {
    match event {
        Event::Text(t) => { /* detectar [[Título]] e renderizar como ui.link */ }
        Event::Start(Tag::Link {dest_url, ..}) => { /* ui.hyperlink_to */ }
        // ...
    }
}
```

Embutimentos `![[arquivo.pdf]]` e `![[imagem.png]]` viram componentes especiais.

## 7. Drag and drop (v2)

`egui` 0.29 tem `egui::DragAndDrop`. Permitir arrastar nota entre pastas:
- `dnd_drag_source(id, payload)` em cada nota da árvore
- `dnd_drop_zone(folder_path)` em cada pasta
- No drop: `fs::rename(note.path, new_folder.join(note.filename))` + `vault.reload_notes()`

## 8. Slash menu (v3)

Difícil em egui (immediate mode + popup com cursor pos). Estratégia:
- Detectar `/` no início de linha no TextEdit
- Mostrar `egui::Window` com `Anchor` calculado a partir de `TextEdit::cursor_range`
- Itens: H1/H2/H3, negrito, código, citação, lista, todo, link, wikilink
- Tab/Enter aplica, Esc fecha

## 9. Integração com Claude Desktop via MCP

Como o vault é só um folder de `.md`, **funciona out-of-the-box** com qualquer MCP filesystem:

1. Configurar Claude Desktop com MCP `filesystem` apontando pro vault root
2. Claude lê/edita `.md` direto, com frontmatter
3. Caderno detecta mudança via `notify` e recarrega

**Sem código adicional necessário.**

## 10. Testes

Adicionar `tests/` com:
- `test_vault.rs`: criar vault, criar nota, salvar, reabrir, conferir frontmatter íntegro
- `test_autoformat.rs`: cobrir casos de math eval (vírgula decimal, parênteses, divisão por zero)
- `test_import.rs`: importar JSON exemplo de export do Claude.ai

## 11. Build e distribuição

```bash
cargo build --release
# Linux: target/release/caderno (~10MB stripped)
# Mac: cargo bundle (precisa cargo-bundle) gera .app
# Windows: target/release/caderno.exe
```

## Roadmap sugerido

| Fase | Escopo | Tempo |
|------|--------|-------|
| **v0.1** | UI sidebar/editor + auto-save + atalhos | 4-6h |
| **v0.2** | Modais (new, import, settings, confirm) | 2-3h |
| **v0.3** | PDF + import Claude funcionando end-to-end | 1-2h |
| **v0.4** | Backlinks + wikilinks renderizados clicáveis | 3-4h |
| **v0.5** | Acessibilidade (fontes, espaçamento, dark) | 2h |
| **v0.6** | Watcher (notify) — sync com Obsidian | 1-2h |
| **v0.7** | Drag-and-drop entre pastas | 2-3h |
| **v0.8** | Slash menu | 4-5h |
| **v1.0** | Polish, testes, build releases | 3-4h |

Total estimado: **22-31h** de Claude Code.

## Decisões arquiteturais

1. **Vault em filesystem, não SQLite/JSON único.** Ganha compatibilidade Obsidian, integração Claude Desktop, git-friendly, recuperação de desastre trivial.

2. **Frontmatter YAML.** Padrão de fato no ecossistema markdown.

3. **`linked_note` por ID, não por path.** Sobrevive a renomear/mover arquivos.

4. **`_attachments/` flat (sem subpastas).** Simplifica embedding `![[nome.pdf]]`.

5. **`.caderno/` ao invés de `.obsidian/`.** Não conflita com Obsidian no mesmo vault.

6. **Sem servidor MCP próprio.** Ferramentas MCP de filesystem oficiais já leem/escrevem `.md`.

## Notas pra Claude Code

- Versões de crates podem precisar de bump. Se algo não compilar, ajusta versão.
- `egui_commonmark` 0.18 espera egui 0.29 — manter alinhado.
- `from_id_source` foi renomeado pra `from_id_salt` em egui 0.29+.
- `lopdf::extract_text` retorna texto cru. Pra layout fiel, considerar `pdf-extract` crate.
- `rfd::FileDialog` é blocking. OK pro caso de uso.
- Sempre `vault.reload_notes()` depois de operação que muda filesystem fora do path conhecido.

---

**Autor:** Juan Fausto + Claude
**Licença:** MIT
**Bom trabalho, Claude Code. Termina isso aí. 🚀**
