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

### Claude Desktop (MCP filesystem)

Adicione ao seu `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "omninote-vault": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/caminho/pro/seu/vault"]
    }
  }
}
```

Reinicie Claude Desktop. Agora o Claude lê e edita seu vault diretamente — e o OmniNote detecta as mudanças via watcher.

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

O projeto segue protocolo de discipline com 4 sacred files:

- [SPRINT.md](./SPRINT.md) — sprint atual + ordered task list + hard rules
- [DIARY.md](./DIARY.md) — append-only log de cada sessão
- [HUMAN.md](./HUMAN.md) — perguntas abertas pro humano
- [NOTION.md](./NOTION.md) — index de tickets sincronizados com Notion

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

Implementado v1.0 (esta release):
- ✅ v0.1 — UI sidebar/editor + auto-save + atalhos
- ✅ v0.2 — Modais (new, settings, confirm)
- ✅ v0.3 — Importação PDF + Claude chat/artefato
- ✅ v0.4 — Wikilinks clicáveis + embeds inline
- ✅ v0.5 — Acessibilidade (fonte/tamanho/espaço)
- ✅ v0.6 — Watcher de filesystem com conflict resolution
- ✅ v0.7 — Drag-and-drop entre pastas
- ✅ v0.8 — Slash menu

Próximas (v2.0+):
- 🔜 Wikilinks renderizados *inline* dentro do CommonMark (parser custom com `pulldown-cmark`)
- 🔜 Fontes acessíveis embutidas: Atkinson Hyperlegible, Lexend, OpenDyslexic
- 🔜 Sync com mobile via algum protocolo (Syncthing? rclone?)
- 🔜 Plugin system

---

## Licença

MIT — © Juan Fausto & Claude
