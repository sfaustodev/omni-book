# OmniNote

> Caderno digital pessoal em Rust nativo (egui) — vault de arquivos `.md` em disco, 100% compatível com Obsidian e Claude Desktop via MCP filesystem.

[![CI](https://github.com/sfaustodev/omni-book/actions/workflows/ci.yml/badge.svg)](https://github.com/sfaustodev/omni-book/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

---

## O que é

OmniNote é um app de notas desktop escrito em Rust. Diferente de Notion ou Apple Notes, o vault é **só uma pasta de arquivos `.md`** com YAML frontmatter — funciona out-of-the-box com Obsidian, com qualquer MCP filesystem (incluindo o do Claude Desktop), e é git-friendly.

**Features (v1.0):**
- 📓 Sidebar com árvore de pastas, busca e filtros por tipo
- ✏️ Editor markdown com auto-save (debounce 600ms)
- 👁 Modo leitura com `egui_commonmark` (renderização nativa)
- 🔗 Wikilinks `[[Título]]` clicáveis + backlinks automáticos
- 🖼 Embeds inline `![[imagem.png]]` e `![[arquivo.pdf]]` (abre com app default)
- 📥 Importação: PDF (extração de texto), chats Claude (JSON), artefatos Claude (código/HTML)
- 👁️ Watcher de filesystem — sync automático com Obsidian/Claude editando o mesmo vault
- 🎨 Drag-and-drop pra mover notas entre pastas
- ⌨️ Atalhos: `Ctrl+N` (nova), `Ctrl+E` (editar/ler), `Ctrl+K` (busca), `Ctrl+,` (settings), `Ctrl+Shift+D` (tema), `Ctrl+=` (avalia matemática na linha)
- ➕ Slash menu: digite `/` no início de uma linha pra inserir blocos markdown (H1-H3, listas, código, citação, link, wikilink, etc.)
- ♿ Acessibilidade: 3 famílias de fonte, tamanho 11-24pt, espaço entre linhas configurável

---

## Build

### Linux

```bash
# Dependências de sistema (Ubuntu/Debian)
sudo apt-get install -y \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

cargo build --release
./target/release/omninote
```

Binário standalone, ~10MB stripped.

### macOS

```bash
cargo build --release
./target/release/omninote
```

Pra `.app` bundle:
```bash
cargo install cargo-bundle
cargo bundle --release
# Gera target/release/bundle/osx/OmniNote.app
```

### Windows

```bash
cargo build --release
.\target\release\omninote.exe
```

Primeira build demora 5-10 min (compila egui + deps). Depois é incremental e rápido.

---

## Integração com Obsidian e Claude Desktop

O OmniNote usa **filesystem como fonte da verdade** — sem SQLite, sem JSON, sem servidor. Isso significa zero configuração pra integração:

### Obsidian
1. Abre seu vault OmniNote no Obsidian
2. Pronto. Os `.md` com YAML frontmatter são compatíveis sem conversão.
3. Edits feitos no Obsidian aparecem no OmniNote automaticamente (via watcher `notify`).

> **Nota:** OmniNote usa `.omninote/` pra config, Obsidian usa `.obsidian/`. Não conflitam.

### Claude Desktop (MCP nativo)

O `omninote-mcp` é um servidor MCP próprio que expõe operações de vault como tools tipadas. Build + instale, depois adicione ao seu `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "omninote": {
      "command": "/Users/SEU_USUARIO/.local/bin/omninote-mcp",
      "env": { "OMNINOTE_VAULT": "/caminho/pro/seu/vault" }
    }
  }
}
```

Reinicie Claude Desktop. O Claude passa a ter 13 tools:

| Tool | O que faz |
|---|---|
| `vault_info` | path, contagem de notas, stats do resolver |
| `note_search` | busca substring em corpo ou só títulos |
| `link_unresolved` | wikilinks quebrados |
| `link_backlinks` | quem linka pra um arquivo (filename/path/alias) |
| `daily_ensure` | cria/abre `Daily/YYYY-MM-DD.md` (CAD-22) |
| `template_list` | templates em `Templates/` |
| `template_apply` | renderiza template com `{{date}}/{{time}}/{{title}}` |
| `diary_append` | append a `discipline/DIARY.md` (prepend no topo) |
| `human_ask` | abre pergunta no `discipline/HUMAN.md` com Q-NN auto |
| `ticket_status` | grep word-bounded em `NOTION.md` / `JIRA.md` |
| `discipline_show` | dump raw de qualquer sacred file |
| `vault_ask` | RAG semântico (fastembed BGE 384d) + opcional Claude completion (CAD-23.1) |
| `note_auto_tag` | sugere tags + summary via Claude, retorna diff (apply opcional) (CAD-23.2) |

**Setup de API key (pra `vault_ask` com LLM + `note_auto_tag`):**

```bash
export ANTHROPIC_API_KEY=sk-ant-...
# OU em ~/.config/omninote/llm.toml:
# [anthropic]
# api_key = "sk-ant-..."
```

---

## CLI

O `omninote-cli` (CAD-21+22) dá as mesmas operações no terminal. Vault path por `--vault PATH`, `OMNINOTE_VAULT` env, ou `~/.config/omninote/last_vault`.

```bash
# Inspeção
omninote-cli vault info
omninote-cli note search "escrow HMAC" [--titles-only] [--limit 20]
omninote-cli link unresolved
omninote-cli link backlinks "SPEC_V2 - NdA"

# Daily + templates (CAD-22)
omninote-cli daily                        # cria/abre Daily/YYYY-MM-DD.md
omninote-cli daily --date 2026-05-23
omninote-cli template list
omninote-cli template apply daily --title "Hoje"

# Discipline (CAD-22)
omninote-cli diary append "session note" --ticket CAD-22
omninote-cli human ask "Posso usar embeddings locais ou só remoto?"
omninote-cli ticket CAD-22                # grep NOTION/JIRA
omninote-cli discipline show sprint

# AI (CAD-23.x — requer ANTHROPIC_API_KEY)
omninote-cli ask "where did I discuss escrow HMAC?" --top-k 5
omninote-cli ask "..." --no-llm           # só retrieval, sem chamar Claude
omninote-cli tag --auto SPEC_V2           # mostra diff (dry-run)
omninote-cli tag --auto SPEC_V2 --apply   # escreve frontmatter
omninote-cli tag --auto SPEC_V2 --replace # substitui tags em vez de adicionar
```

Todo verbo aceita `--json` pra output machine-readable (envelope `{ok, data, meta}`).

---

## Atalhos de teclado

| Atalho | Ação |
|--------|------|
| `Ctrl+N` | Nova nota (modal com 6 tipos) |
| `Ctrl+E` | Toggle modo Ler ↔ Editar |
| `Ctrl+K` | Foca busca da sidebar |
| `Ctrl+,` | Configurações |
| `Ctrl+Shift+D` | Toggle tema claro/escuro |
| `Ctrl+=` | Avalia expressão matemática na linha (ex: `2 + 3 =` → `2 + 3 = 5`) |
| `/` (no início de linha) | Slash menu pra inserir bloco markdown |
| `Esc` | Fecha modais / slash menu |

---

## Estrutura do vault

```
<vault>/
├── .omninote/              # config persistente (não toque)
│   └── config.json         # tema, fonte, tamanho, etc
├── _attachments/           # PDFs, imagens importadas
│   └── *.pdf, *.png, ...
├── Pasta A/
│   ├── Nota 1.md
│   └── Nota 2.md
└── Nota raiz.md
```

Cada `.md` tem frontmatter YAML compatível com Obsidian:
```yaml
---
id: n_<uuid>
type: resumo               # resumo|citacao|codigo|exercicio|duvida|definicao
tags: [rust, prog]
source: ""
source_link: ""
linked_note: null
attachments: []
created: 2026-05-01T12:00:00Z
---

Conteúdo markdown aqui...
```

---

## Discipline (workflow do projeto)

O projeto segue protocolo de discipline com sacred files em [`discipline/`](./discipline/):

- [SPRINT.md](./discipline/SPRINT.md) — sprint atual + ordered task list + hard rules
- [DIARY.md](./discipline/DIARY.md) — append-only log de cada sessão
- [HUMAN.md](./discipline/HUMAN.md) — perguntas abertas pro humano
- [PLAN.md](./discipline/PLAN.md) — plano do sprint vigente
- [NOTION.md](./discipline/NOTION.md) — index de tickets sincronizados com Notion

Esses arquivos são acessíveis programaticamente via `omninote-cli discipline show <FILE>` e `omninote-cli diary append`/`human ask`/`ticket` (também expostos como MCP tools).

Specs por ticket vivem em [SPECS/](./SPECS/).

---

## Stack técnica

| Crate | Uso |
|-------|-----|
| `eframe` + `egui` 0.29 | UI imediata, single-binary |
| `egui_commonmark` 0.18 | Render de markdown |
| `serde_yaml` | Frontmatter compatível com Obsidian |
| `lopdf` | Extração de texto de PDF |
| `notify` | Watcher de filesystem |
| `rfd` | File dialogs nativos |
| `walkdir` | Varredura recursiva do vault |
| `pulldown-cmark` | (planejado) Parser custom pra wikilinks renderizados inline |
| `open` | Abrir arquivos com app default do sistema |
| `meval` | Avaliação aritmética segura (Ctrl+=) |
| `uuid` | IDs estáveis das notas |

Versões pinadas no [Cargo.toml](./Cargo.toml).

---

## Roadmap

Implementado v1.0:
- ✅ v0.1 — UI sidebar/editor + auto-save + atalhos
- ✅ v0.2 — Modais (new, settings, confirm)
- ✅ v0.3 — Importação PDF + Claude chat/artefato
- ✅ v0.4 — Wikilinks clicáveis + embeds inline
- ✅ v0.5 — Acessibilidade (fonte/tamanho/espaço)
- ✅ v0.6 — Watcher de filesystem com conflict resolution
- ✅ v0.7 — Drag-and-drop entre pastas
- ✅ v0.8 — Slash menu

Implementado v1.1 (sprint 2026-05-20 → 06-03):
- ✅ CAD-20 — Obsidian link parity (`|alias`, `#heading`, `#^block`, frontmatter `aliases`)
- ✅ CAD-21 — Workspace refactor + `omninote-cli` + `omninote-mcp` nativos

Implementado v1.2 (sprint 2026-06-03 → 06-17):
- ✅ CAD-22 — Daily notes + templates + discipline CLI/MCP

Em curso v1.3 (sprint 2026-06-17 → 07-01) — CAD-23 fatiado:
- ✅ CAD-23.1 — RAG search (omninote-ai crate + `ask` verb + `vault_ask` tool)
- ✅ CAD-23.2 — Auto-tag + summary (`tag --auto` verb + `note_auto_tag` tool)
- 🔜 CAD-23.3 — Dictation Whisper local
- 🔜 CAD-23.4 — OCR PDF (tesseract via leptess)

Próximas:
- 🔜 CAD-24 — Power automation (quick-capture global hotkey, multi-vault switcher)
- 🔜 CAD-25 Fase B — UI Design v2 (right rail, command palette, discipline-typed views)

---

## Licença

MIT — © Juan Fausto & Claude
