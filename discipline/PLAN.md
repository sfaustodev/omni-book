# PLAN — OmniNote (active sprint)

> **Sacred file** per discipline rule #15. Append plan no início de toda branch nova OU prompt/spec novo. Read order: SPRINT → DIARY → NOTION → HUMAN → PLAN.

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

| # | Phase | Status |
|---|-------|--------|
| 1 | Setup deps + CI + sacred files | 🚧 |
| 2 | vault.rs adversarial | ⏳ |
| 3 | wikilinks.rs proptest | ⏳ |
| 4 | autoformat.rs adversarial | ⏳ |
| 5 | import.rs adversarial | ⏳ |
| 6 | pdf.rs panic safety | ⏳ |
| 7 | types.rs serde | ⏳ |
| 8 | refactor handlers + tests | ⏳ |
| 9 | docs + MANUAL_TEST_PLAN | ⏳ |
| 10 | discipline updates | ⏳ |
| 11 | PR + skills | ⏳ |

### Next single-step

Phase 1 conclude → start Phase 2 (vault adversarial expansion).

### Não-objetivos

- lib.rs split — adiado, Q-06 em HUMAN.md
- egui_kittest harness — exige egui ≥0.30 upgrade, fora de escopo
- 90% UI render coverage — manual_test_plan.md cobre
