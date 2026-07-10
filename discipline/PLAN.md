# PLAN — OmniNote (active sprint)

> **Sacred file** per discipline rule #15. Append plan no início de toda branch nova OU prompt/spec novo. Read order: SPRINT → DIARY → NOTION → HUMAN → PLAN.

---

## 2026-07-10 — CAD-25 Slice 7 — theme gallery + native macOS menu bar

### Contexto

Prompt Fausto: recuperar os frontends feitos em experimentos anteriores (córtex OFF-arm `cortex/off-{1,2,3}` + stash `omninote-swiss-theme`) e implementá-los como temas trocáveis via dropdown "Tema" na barra de menus nativa do macOS, com um menu "Editar" expondo os mesmos comandos de formatação do menu `/`/botão-direito. Full plan em `~/.claude/plans/memoized-splashing-corbato.md`.

### Escopo

1. `ThemePreset` (omninote-core) 4→9 variantes: `AlmanacLight`/`AlmanacDark` (ex-`cortex/off-1`, parchment/oxblood/terracotta `#BF4D26`), `Blueprint`/`BlueprintLight` (ex-`cortex/off-2`, navy/cyan `#4FC3F7`), `Swiss` (ex-stash `omninote-swiss-theme`, Bauhaus preto/laranja `#FF5A1F`) — aditivo, wire names dos 4 originais preservados.
2. `theme.rs`: 5 novos `Theme::` construtores (cores só — rounding/shadow/spacing seguem universais, mecanismo `apply()` intocado) + testes estendidos.
3. Settings modal: checkbox "Modo escuro" → ComboBox completo sobre `ThemePreset::all()` (fecha órfão pré-existente de `HighContrast`/`Custom`, que não tinham UI nenhuma) + color picker de accent pra `Custom`.
4. `native_menu.rs` novo (crate `muda`, macOS-only via `#[cfg]`, stub no-op em outras plataformas — `app.rs` não precisa de nenhum `#[cfg]` próprio): menu **Tema** (9 `CheckMenuItem`, um por linha, radio manual) + **Editar** (mesmo `MdFormat` do right-click/slash, ⌘B/⌘I novos + Selecionar tudo/Copiar sem accelerator pra não colidir com o que `TextEdit` já trata nativamente) + **Arquivo** mínimo.
5. Cut/Paste/Undo/Redo nativos — deliberadamente fora de escopo (documentado no module doc do `native_menu.rs`): já funcionam via teclado hoje; PredefinedMenuItem provavelmente não alcança o buffer custom-rendered do egui (sem responder chain NSTextView) e Undo/Redo exigiriam stack próprio — não pedido, não construído especulativamente.

### Verificação

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test --workspace   # 100 gui + 253 core + 126 ai + resto — todos verdes
cargo build --release    # confirma muda linka em release
```

Smoke humano macOS obrigatório (menu nativo não é testável por unit test) — checklist em `discipline/MANUAL_TEST_PLAN.md`.

### Next single-step

Rodar `/pre-merge-coverage` → `/codex-cross-review` (ou triad completo, dado que toca `app.rs`/settings) → aguardar Fausto confirmar smoke test macOS em chat → só então `gh pr create` (rule #13, #26).

---

## 2026-06-02 — triad-codex-section — wikilinks adversarial tests

### Contexto

Prompt Fausto: escrever somente testes Rust adversariais para `omninote_core::wikilinks::{section_under_heading, extract_spans}`. Não tocar código de produção. Salvar classes cobertas em `reports_fausto/triad-cov-codex-slice3.md`.

### Escopo

- Novo integration test `crates/omninote-core/tests/triad_codex_section.rs` com `use omninote_core::wikilinks::*;`.
- Cobrir `section_under_heading`: fence não-fechada, fence aninhada com marcador trocado, ATX inválido `#######`, heading sem espaço, heading EOF sem corpo, duplicado primeiro, CJK/multibyte, input vazio, input gigante linear.
- Cobrir `extract_spans`: links colados, inline code, embed sem fechamento, spans byte-exatos com multibyte.

### Verificação

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo test -p omninote-core
```

### Next single-step

Criar testes + relatório, rodar suite crate completa.

---

## 2026-05-13 — feat/cad-12-test-coverage — qa+security hardening

### Contexto

Comprehensive test + coverage hardening sprint. Triggered by Fausto request "escreva testes abrangentes de QA e de segurança e aplique verifique cada botão e funcionalidade coverage minimo de 90%".

Out of v0.1-v0.3 active sprint scope (CAD-2..8). Created **CAD-12** in Notion to track. Honours §0 #10 (no `tests/` dir until lib.rs exists) — all tests stay inline `#[cfg(test)]`.

### Escopo

1. ≥90% line coverage gate em CI pra módulos pure: `vault, wikilinks, autoformat, import, pdf, types, app` (handlers extraídos)
2. Adversarial / fuzz tests em parsers: wikilinks, autoformat, JSON import, YAML frontmatter, PDF
3. Path traversal + symlink + filesystem boundary tests em `vault::*`
4. Refactor: extract `pub mod actions` em `app.rs` com handlers de cada botão da UI; UI render layer fica fora do gate
5. Manual test plan documentado em `discipline/MANUAL_TEST_PLAN.md` pra coisas só-UI (rfd dialogs, panic hook, watcher)
6. Post-merge: `/pre-merge-coverage` + `/codex-cross-review`

### Arquivos críticos

- `Cargo.toml` — add `proptest = "1"` dev-dep (DONE)
- `.github/workflows/ci.yml` — new `coverage` job com `cargo-llvm-cov --fail-under-lines 90` (DONE)
- `src/{vault,wikilinks,autoformat,import,pdf,types}.rs` — append tests inline
- `src/app.rs` — extract `pub mod actions`, append handler tests
- `src/ui_{sidebar,editor,modals}.rs` — substitui closures por `actions::*`
- `discipline/MANUAL_TEST_PLAN.md` — novo
- `discipline/{NOTION,HUMAN,DIARY}.md` — atualizar

### Verificação

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test                                       # ~33 → ~120+
cargo llvm-cov --html --include-files src/vault.rs ...   # visual ≥90%
cargo llvm-cov --fail-under-lines 90 --include-files src/vault.rs ...   # gate
cargo audit
cargo build --release
```

### Milestones

| # | Phase | Status | Tests added |
|---|-------|--------|-------------|
| 1 | Setup deps + CI + sacred files | ✅ | — |
| 2 | vault.rs adversarial | ✅ | +36 |
| 3 | wikilinks.rs proptest | ✅ | +17 (+2 proptest) |
| 4 | autoformat.rs adversarial | ✅ | +18 (+2 proptest) |
| 5 | import.rs adversarial | ✅ | +14 |
| 6 | pdf.rs panic safety | ✅ | +6 (+1 proptest) |
| 7 | types.rs serde | ✅ | +12 |
| 8 | refactor `actions` mod + UI rewiring | ✅ | +30 |
| 9 | discipline updates | 🚧 | — |
| 10 | local llvm-cov verify | ✅ | — |
| 11 | PR + skills | ⏳ | — |

**Final coverage (local, 2026-05-13):**

| File | Lines | Cover |
|------|-------|-------|
| actions.rs | 517 | 95.16% |
| autoformat.rs | 179 | 100.00% |
| import.rs | 209 | 99.52% |
| pdf.rs | 98 | 98.98% |
| types.rs | 151 | 100.00% |
| vault.rs | 665 | 94.14% |
| wikilinks.rs | 186 | 98.92% |
| **TOTAL** | **2005** | **96.61%** |

≥90% gate passes. UI render layer (`ui_*.rs`), eframe glue (`app.rs`),
watcher (`watcher.rs`) and entry (`main.rs`) excluded — covered by
[MANUAL_TEST_PLAN.md](MANUAL_TEST_PLAN.md).

### Next single-step

Open PR `feat: CAD-12 — QA + security coverage hardening`, then run
`/pre-merge-coverage` + `/codex-cross-review`.

### Não-objetivos

- lib.rs split — adiado, Q-06 em HUMAN.md
- egui_kittest harness — exige egui ≥0.30 upgrade, fora de escopo
- 90% UI render coverage — manual_test_plan.md cobre


---

## 2026-05-20 — sprint-2026-05-20-batch — OmniNote v1.1+ roadmap

### Contexto

Brainstorm session com Fausto (caveman mode) resolveu 3 perguntas:

1. **Obsidian compat:** advanced links (`|alias`, `path/note`, `#heading`, `#^block`, frontmatter `aliases`) + daily notes + templates. Skipped: graph view, canvas.
2. **External surface:** **CLI + MCP** com `omninote-core` shared lib.
3. **Killer themes:** AI-native vault + power automation + discipline/ticket sync. Skipped: sync+mobile.

Plano-fonte: `~/.claude/plans/greedy-napping-castle.md`

### Escopo

5 fases umbrella + 1 ticket UI = 6 tickets Notion (CAD-20..CAD-25), agrupados em 3 sprints de 2 semanas:

- **Sprint v1.1 (2026-05-20 → 2026-06-03):** CAD-20 (link parity, blocker) + CAD-21 (workspace+CLI+MCP) + CAD-25 Fase A (UI análise paralela)
- **Sprint v1.2 (2026-06-03 → 2026-06-17):** CAD-22 (discipline CLI) ⟂ CAD-25 Fase B (UI implementação)
- **Sprint v1.3 (2026-06-17 → 2026-07-01):** CAD-23 (AI-native) ⟂ CAD-24 (power automation)

### Arquitetura

Workspace Cargo:

```
omninote/
├── crates/
│   ├── omninote-core/   (lib: vault/wikilinks/resolver/search/templates/daily/discipline)
│   ├── omninote-gui/    (egui app atual minus core)
│   ├── omninote-cli/    (clap binary `omninote`)
│   └── omninote-mcp/    (rmcp server `omninote-mcp`)
```

Single source of truth em `omninote-core`. GUI/CLI/MCP consomem via direct fn calls.

### Parallel work strategy

```
v1.1 ──── CAD-20 (sequencial, blocks all)
     │
     ├─── CAD-25 Fase A (paralelo, read-only docs)
     │
     └─── CAD-21 (depende CAD-20)
                │
v1.2 ──── CAD-22 ⟂ CAD-25 Fase B
                │
v1.3 ──── CAD-23 ⟂ CAD-24
```

### Verificação

- Phase 1: `cargo test wikilinks:: resolver::` green + abrir `~/Documents/Obsidian Vault` no OmniNote → zero unresolved
- Phase 2: `omninote --vault X vault info` match `obsidian vaults verbose`; `omninote-mcp` callable from Claude Desktop
- Phase 3: `omninote daily` cria `Daily/YYYY-MM-DD.md`; `omninote diary append` escreve DIARY entry
- Phase 4: `omninote ask "escrow HMAC"` retorna `[[SPEC_V2 - NdA]]` top-3; dictation WER < 10% pt-BR; OCR legível
- Phase 5: quick-capture hotkey → `Inbox.md` cresce 1 linha; `omninote diff --since 1d` match `git log --since=1.day`

### Out of scope

Graph view, canvas, sync+mobile, web clipper, plugin system, E2E encryption.

### Next single-step

Spawnar `frontend-design` subagent em sessão dedicada com prompt em `~/.claude/plans/greedy-napping-castle.md` seção "Brief para Claude design subagent". Em paralelo, começar CAD-20 (Phase 1 link parity) — sequencial blocker.
