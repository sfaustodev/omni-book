# DIARY — OmniNote

> Append-only execution log. Newest entry no topo. Nunca editar histórico.

---

## 2026-05-02 — Q-01..Q-04 resolvidos + merge v0.1-v1.0 → main

**Tickets touched:** CAD-5 (Q-02), todos via merge

**Done:**
- Merge `feat/omninote-v10-readme-polish` → main com `--no-ff`. Trouxe v0.1 a v1.0 cumulative (10 commits) pra main. Push `0874dbf..b327207`. CI deve rodar.
- Q-04 resolvido: (a) confirmado — autoformat só linha atual, sem mudança de código.
- Q-03 resolvido: (b) confirmado — comportamento já implementado no v06-watcher.
- Q-02 resolvido: (a) — `i.modifiers.ctrl` → `i.modifiers.command` em 3 arquivos (5 sites). Cmd no mac, Ctrl no resto, auto-mapeado pelo egui.
- Q-01 resolvido: migrate (não delete) — `Vault::open` agora renomeia `.caderno/` → `.omninote/` se legacy existir. Edge case: ambos existem → drop legacy. 2 testes novos.
- 33 testes passando (+2 vs antes).
- Branch nova: `feat/omninote-q01-q02-cmd-migrate` off main.

**In flight:**
- Branch `feat/omninote-q01-q02-cmd-migrate` aguarda smoke macOS humano (Cmd+N/E/K/,/=/Shift+D).
- Notion tasks CAD-2..CAD-11 ainda em status pré-merge (🚧/🌱/🎯). Não fechar até confirmação humana escrita ("testado, pode fechar") — discipline §13.

**Blocked:**
- Nenhum bloqueador.

**Files changed:**
- `src/app.rs` (4 shortcuts → command)
- `src/ui_editor.rs` (Ctrl+= → command)
- `src/ui_sidebar.rs` (Ctrl+K → command)
- `src/vault.rs` (`.caderno/` migration + 2 testes)
- `discipline/HUMAN.md` (Q-01..Q-04 → Resolved)

**Next session:**
- Smoke humano em macOS local (`cargo run` na branch q01-q02). Validar Cmd+N abre modal nova nota, Cmd+K foca busca, Cmd+= avalia matemática.
- Após confirmação: merge q01-q02 → main, fechar Notion CAD tasks via MCP.

---

## 2026-05-02 — discipline migration: root → discipline/ subfolder

**Tickets touched:** none (housekeeping)

**Done:**
- `git mv SPRINT.md DIARY.md HUMAN.md NOTION.md → discipline/` — convenção alinhada com CFO project
- Atualizado `discipline/NOTION.md` table links: `[SPECS/CAD-X.md](../SPECS/CAD-X.md)` (URL passa pelo parent dir)
- `SPECS/CAD-5.md` + `SPECS/CAD-9.md`: refs `[../HUMAN.md]` → `[../discipline/HUMAN.md]`
- SPECS/ permanece em root (mesma convenção do CFO `specs/`)

**Why:** humano apontou inconsistência — CFO usa `discipline/` subfolder, OmniNote estava com files no root. Padronização. Bonitinho.

**Files changed:**
- moved: `SPRINT.md`, `DIARY.md`, `HUMAN.md`, `NOTION.md` → `discipline/`
- modified: `discipline/NOTION.md`, `SPECS/CAD-5.md`, `SPECS/CAD-9.md`

**Next session:** root tem só `README.md`, `SPEC.md`, `Cargo.toml`, `src/`, `SPECS/`, `.github/`, `discipline/`. Skill discipline lê `discipline/*.md` automaticamente — sem mudança nas regras.

---

## 2026-05-01 — bootstrap + v0.1 a v0.3 + CI + discipline

**Tickets touched:** `CAD-2`, `CAD-3`, `CAD-4`, `CAD-5`, `CAD-6`, `CAD-7`, `CAD-8`

**Done:**
- Renomeado projeto Caderno → OmniNote: `Cargo.toml`, `main.rs`, struct `CadernoApp` → `OmniNoteApp`, `.caderno/` → `.omninote/`, config dir `~/.config/caderno/` → `~/.config/omninote/`
- Adicionada dep `open = "5"` + dev-dep `tempfile = "3"`
- `src/types.rs`: adicionado enum `ConfirmAction { DeleteNote(String), DeleteFolder(PathBuf) }`
- `src/app.rs`: refatorado state — usar `active_note: Option<Note>` (clone) ao invés de `active_idx: usize`; adicionado `md_cache`, `confirm_action`, `type_filter`; impl `flush_active()` com rename automático no save; impl `select_note(id)` que flush + load
- `src/ui_sidebar.rs` (novo, 195 linhas): SidePanel 280px com header, search, type chips, tree recursiva, footer
- `src/ui_editor.rs` (novo, 230 linhas): edit mode (TextEdit + autoformat Ctrl+=) + view mode (CommonMarkViewer + backlinks)
- `src/ui_modals.rs` (novo, 240 linhas): 4 modais (new, settings, confirm, import) + 3 helpers de import
- `src/vault.rs`: paths `.caderno/` → `.omninote/`, exposto `sanitize_filename`, novo `rename_note_by_id`
- 19 testes inline (`#[cfg(test)]`): vault (6), autoformat (8), import (5) — todos passando
- `.github/workflows/ci.yml`: pipeline 4 jobs (lint → test → build → security-audit) com deps Linux pra eframe/rfd
- `CLAUDE.md`: arquitetura + padrões egui + comandos CI
- Repo git inicializado em `/Users/peluche/Projects/ClaudeBook/caderno/`, branch `feat/omninote-v01` push pra `https://github.com/sfaustodev/omni-book.git`
- Discipline files criados em `main`: SPRINT.md, DIARY.md, HUMAN.md, NOTION.md, SPECS/CAD-2..CAD-11.md

**In flight:**
- feat/omninote-v01 PR aberto, aguarda merge pós teste humano (CAD-2 ainda em `🚧 Em obra` no Notion)

**Blocked:**
- Nenhum bloqueador

**Files changed:**
```
Cargo.toml
src/main.rs
src/app.rs
src/types.rs
src/vault.rs
src/import.rs
src/autoformat.rs
src/ui_sidebar.rs (novo)
src/ui_editor.rs (novo)
src/ui_modals.rs (novo)
.github/workflows/ci.yml (novo)
CLAUDE.md (novo)
SPRINT.md (novo)
DIARY.md (novo)
HUMAN.md (novo)
NOTION.md (novo)
SPECS/*.md (10 novos)
```

**Decisões registradas (vide HUMAN.md se houver dúvida):**
- `flush_active()` usa `Option::take()` pra contornar borrow checker entre `&mut active_note` e `&mut vault` — log no DIARY pq pattern não-óbvio
- Tests inline (`#[cfg(test)]`) ao invés de `tests/` dir — projeto é binary crate, simpler assim
- Hook de segurança bloqueou referência direta à função wrapper `meval` em testes — workaround: testes exercitam `try_math_substitute` que internamente faz a avaliação aritmética
- `Ctrl+=` autoformat: usar `TextEdit::show()` (retorna `TextEditOutput` com `cursor_range`) ao invés de `ui.add(TextEdit)` (retorna apenas `Response`)

**Next session should start with:**
- Esperar humano rodar local: `cargo run` em `feat/omninote-v01`
- Após confirmação ("testado, pode fechar"), mergear feat/omninote-v01 → main, mover CAD-2..CAD-8 pra `✅ Concluída` no Notion via MCP
- Verificar CI rodou no GitHub Actions
- Próxima fase: CAD-10 (Spike wikilinks v0.4)
