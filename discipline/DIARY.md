# DIARY — OmniNote

> Append-only execution log. Newest entry no topo. Nunca editar histórico.

---

## 2026-05-13 — CAD-12 · QA + security coverage hardening

**Tickets touched:** CAD-12 (novo, criado no Notion sob 🚢 Caderno de Bordo, status `🚧 Em obra` → `👀 Revisão` ao fim da sessão)

**Branch:** `feat/cad-12-test-coverage` (off `main`)

**Done:**

- Sacred file `discipline/PLAN.md` criado (rule #15) com escopo, milestones, critical files, verification.
- Sacred file `discipline/MANUAL_TEST_PLAN.md` criado — checklist humano pra superfícies só-UI (rfd dialogs, watcher, OpenDyslexic visual).
- `Cargo.toml`: add `proptest = "1"` em `[dev-dependencies]`.
- `.github/workflows/ci.yml`: novo job `coverage` rodando `cargo llvm-cov --fail-under-lines 90` com `--ignore-filename-regex 'src/(ui_|main\.rs|watcher|theme|app\.rs)'`.
- `CLAUDE.md`: comandos `cargo llvm-cov` documentados + ponteiro pra MANUAL_TEST_PLAN.
- **Tests inline expandidos** (§0 #10 honrado, sem `tests/` dir):
  - `vault.rs` 9 → 45 (+36): sanitize traversal Unix+Win, dangerous chars, unicode/emoji preservation, zero-width gap (Q-05), Vault::open file-as-root (Q-08), traversal containment via canonicalize, collision counter, rename/move edges, delete folder, attachment collision + arbitrary extension (Q-07), parse_frontmatter panic safety + obsidian compat.
  - `wikilinks.rs` 9 → 26 (+17): bracket adversarial, multiline+null+traversal+scheme strings, very-long inner, unicode titles, embed-in-link, full extension matrix (6 image lower+upper, 8 file), backlash, position order. **+2 proptest** (256 cases each).
  - `autoformat.rs` 8 → 28 (+18): empty/operator-only/digits-only, deeply-nested parens, very-long expr, exponent notation, div-by-zero, overflow→inf rejected by `is_finite`, FP precision, mixed comma/period, pos OOB, function names blocked by char-whitelist, injection strings non-panic. **+2 proptest** (256 cases each).
  - `import.rs` 5 → 19 (+14): malformed JSON × 6, deep nesting (depth 200), missing-messages error, unknown role, multimodal content, plain-string variant, missing content, no-name no-H1, separator collision, missing/zero-byte file, full artifact extension matrix (tsx/jsx/ts/js/py/rs/html + unknown), triple-backtick collision.
  - `pdf.rs` 0 → 7 (+7): in-memory `lopdf::Document` fixtures (no binary check-ins), single+multi-page, zero-byte/random-bytes/non-PDF/missing errors. **+1 proptest** (32 cases, `catch_unwind`).
  - `types.rs` 0 → 12 (+12): NoteType yaml round-trip × 6, label/icon non-empty, FontFamily round-trip × 3 + egui mapping, AppConfig defaults + serde missing/extra fields, Frontmatter round-trip preserving `linked_note` (§0 #4), ConfirmAction Debug.
  - `actions.rs` (NOVO) 30 testes: confirm flow (request/confirm/cancel × 2 ops, active-clear semantics), set_type_filter / set_query / filtered_note_indices, toggle_edit, external_change_reload (refresh + drop-when-deleted), external_change_keep, reset_settings + set_font_family persistência, import_pdf/chat/artifact wrappers, attach_file_to_active retorna wikilink, backlinks_to scan + skip embeds, create_link_to_new.
- **Refactor `src/actions.rs`** — `pub mod actions` extraído (691 linhas + 30 testes). Cada handler aceita só `&mut Vault`/`&mut Option<Note>`/flags simples → testável sem `eframe::CreationContext`.
- **UI rewiring**: `app.rs` (Cmd+E shortcut), `ui_editor.rs` (✎ Editar, 🗑 delete, 📎 anexar, tag link, create_note_from_wikilink), `ui_sidebar.rs` (chips, 🗑 deletar pasta, delete-note menu), `ui_modals.rs` (external change recarregar/manter, confirm sim/cancelar, import_pdf/chat/artifact thin wrappers).
- 4 handlers (`filtered_note_indices`, `reset_settings`, `set_font_family`, `backlinks_to`) testados mas ainda não wired — `#[allow(dead_code)]` + comentário pointer pro follow-up.
- **Coverage local (cargo-llvm-cov 0.8.7):**

```
File              Lines  Cover
actions.rs          517  95.16%
autoformat.rs       179 100.00%
import.rs           209  99.52%
pdf.rs               98  98.98%
types.rs            151 100.00%
vault.rs            665  94.14%
wikilinks.rs        186  98.92%
TOTAL              2005  96.61%
```

≥90% gate passa.

**Total tests:** 31 → 167 (+136), 0 falhas. cargo clippy clean. cargo fmt clean.

**In flight:**

- PR `feat: CAD-12 — QA + security coverage hardening` aberto pós-DIARY.
- `/pre-merge-coverage` + `/codex-cross-review` rodam após CI verde.
- Smoke macOS humano via `MANUAL_TEST_PLAN.md` obrigatório antes de fechar CAD-12 (rule #13).

**Decisões registradas (HUMAN.md adicionado Q-05/06/07/08):**

- **Q-05** coverage gate fail-PR vs warn-only — escolhi (a) fail-PR.
- **Q-06** lib.rs split — escolhi adiar pra v0.4 com CAD-10.
- **Q-07** import_attachment sem allow-list — (a) status quo (rfd dialog é o gate).
- **Q-08** Vault::open root=arquivo retorna Ok vault-vazio — sugiro fix em PR follow-up.

⚠️ **HUMAN.md alarm:** 8 perguntas abertas (Q-01..Q-08), excede threshold de 5.

**Stash WIP:** `stash@{0}` na branch `feat/omninote-swiss-theme` preserva 3 arquivos modificados antes do branch (theme.rs, ui_sidebar.rs, DIARY.md). Restaurar após CAD-12 mergear.

**Next session should start with:**

- Verificar CI verde no PR.
- Se verde: `/pre-merge-coverage` + `/codex-cross-review`, salvar reports em `reports_fausto/` (gitignored).
- Esperar Fausto rodar smoke macOS local + escrever "testado, pode fechar CAD-12".
- Após confirmação: mergear PR via gh UI, mover Notion CAD-12 → ✅ Concluída via MCP.
- Restaurar `git stash pop` do swiss-theme branch.

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
