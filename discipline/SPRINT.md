# SPRINT — OmniNote v1.1 (Foundation)

> **Sprint goal:** entregar (a) link parity com Obsidian — abre vault Obsidian e zero unresolved, (b) Cargo workspace com `omninote-core` + `omninote-cli` + `omninote-mcp` rodando, (c) análise UI Design v2 pronta pra implementação na próxima sprint.
> **Sprint window:** 2026-05-20 → 2026-06-03 (2 semanas)
> **Tracker:** [Notion · 🚢 Caderno de Bordo](https://app.notion.com/p/35373ac79ddb81fa96bcdb9991425508) — ver [NOTION.md](NOTION.md) pro índice local.
> **Plano de origem:** `~/.claude/plans/greedy-napping-castle.md`

---

## §0 — Hard rules (não-negociáveis)

1. **Vault em filesystem é fonte da verdade.** Nunca cachear estado em SQLite/JSON único — quebra compatibilidade Obsidian e MCP filesystem.
2. **`.omninote/` (não `.obsidian/`).** Coexistência com Obsidian no mesmo vault sem conflito.
3. **Frontmatter YAML compatível com Obsidian.** Nunca quebrar o parser do Obsidian — testar em vault compartilhado se houver dúvida.
4. **`linked_note` por ID, não por path.** Sobrevive a renomear/mover.
5. **OmniNote-MCP é PRÓPRIO (não filesystem genérico).** A partir da v1.1 OmniNote ship seu próprio MCP server (`omninote-mcp` crate, `rmcp`). Filesystem MCP externo deixa de ser a recomendação.
6. **Nunca commitar `.omninote/` ou `_attachments/` de vaults reais.** Dados do usuário, não do projeto.
7. **`from_id_salt` (não `from_id_source`).** API egui 0.29.
8. **`active_note: Option<Note>` clonado.** Nunca `active_idx: usize` — índices invalidam em mutação.
9. **CommonMarkCache em `OmniNoteApp.md_cache`.** Nunca recriar por frame.
10. **Tests inline (`#[cfg(test)]`).** Workspace refactor (CAD-21) pode introduzir `tests/` per crate quando necessário.
11. **Workspace core = `omninote-core` é a única source of truth de vault ops.** GUI, CLI e MCP consomem via direct fn calls. Zero duplicação de lógica.

---

## §1 — Sprint v1.1 (atual): Foundation

| # | ID | Tarefa | Status | Prio | Estimativa | Notas |
|---|------|--------|--------|------|------------|-------|
| 1 | CAD-20 | Phase 1 — Obsidian link parity | 🎯 Pronta | ⚡ | 16h | bloqueia tudo. Sequential. |
| 2 | CAD-21 | Phase 2 — Workspace refactor + CLI + MCP | 🌱 Backlog | ⚡ | 24h | depende CAD-20 done |
| 3 | CAD-25 (Fase A) | UI Design v2 — análise + plano de port | 🎯 Pronta | ⚡ | ~8h | **PARALELO** com CAD-20/21 (só leitura + doc) |

### Dependency graph

```
CAD-20 (link parity)
  └─→ CAD-21 (workspace refactor) ──→ Sprint v1.2 + v1.3
       └─→ CAD-25 Fase B (UI implementation, Sprint v1.2)

CAD-25 Fase A (análise) ⟂ paralelo com tudo (read-only mockups + escrita de docs)
```

### Parallel work strategy

- **Solo dev (Fausto):** atacar CAD-20 primeiro (foundation), CAD-25 Fase A em paralelo nas pausas (análise não bloqueia código)
- **Múltiplos agentes:** CAD-20 + CAD-25 Fase A em paralelo (sem conflito de arquivos — CAD-20 toca `src/wikilinks.rs`+`vault.rs`+`resolver.rs`+`ui_editor.rs`, CAD-25 Fase A só escreve em `docs/UI_DESIGN_v2.md`)
- CAD-21 começa só depois de CAD-20 mergeado (refactor toca os mesmos files)

---

## §1.5 — Backlog Sprint v1.2 (2026-06-03 → 2026-06-17)

Sprint goal: discipline CLI/MCP + UI core implementation.

| ID | Tarefa | Status | Prio | Est | Depende |
|------|--------|--------|------|-----|---------|
| CAD-22 | Phase 3 — Daily/Templates/Discipline CLI+MCP | 🌱 Backlog | ⚡ | 18h | CAD-21 |
| CAD-25 (Fase B) | UI Design v2 — implementação egui | 🌱 Backlog | ⚡ | ~22h | CAD-20 + análise CAD-25A |

Parallel: CAD-22 ⟂ CAD-25 Fase B (touchpoint mínimo — CAD-22 mexe em `omninote-core`/CLI, CAD-25 mexe em `omninote-gui/src/ui_*.rs`)

---

## §1.6 — Backlog Sprint v1.3 (2026-06-17 → 2026-07-01)

Sprint goal: AI-native + power automation.

| ID | Tarefa | Status | Prio | Est | Depende |
|------|--------|--------|------|-----|---------|
| CAD-23 | Phase 4 — AI-native vault | 🌱 Backlog | ⚡ | 40h | CAD-21 |
| CAD-24 | Phase 5 — Power automation | 🌱 Backlog | 📌 | 20h | CAD-21 |

Parallel: CAD-23 ⟂ CAD-24 (CAD-23 = nova crate `omninote-ai` ou feature flag, CAD-24 = nova bin `omninote-capture` + extensões CLI; arquivos disjuntos)

---

## §2 — Definition of Done

Pra cada CAD-XX considerar pronto somente após:

1. Código compila sem warnings (`cargo build --workspace`)
2. `cargo clippy --workspace --all-targets -- -D warnings` passa
3. Tests aplicáveis passam (`cargo test --workspace`)
4. Coverage ≥90% nos módulos pure (`omninote-core`) via `cargo llvm-cov`
5. **Humano testou em macOS local e confirmou no chat** ("testado, pode fechar")
6. Notion task movida pra `✅ Concluída` via MCP
7. `NOTION.md` atualizado com status final
8. Entrada em `DIARY.md` com label `[CAD-XX done]`

---

## §3 — Branch strategy

- `main` — protegida; recebe merge de feat branches via PR
- `feat/cad-XX-slug` — uma branch por ticket (não mais por fase)
- Branches stacked permitidas: CAD-21 sai de CAD-20 antes de CAD-20 mergear (refactor depende de novos files); PR de CAD-21 → CAD-20 até CAD-20 mergear, depois rebase pra main
- Bugs descobertos em testes humano = commits adicionais na **mesma** branch
- Discipline files (SPRINT/DIARY/HUMAN/NOTION/PLAN/SPECS) vivem em main, atualizadas a cada sessão

---

## §4 — PR-first workflow

**Regra:** toda feature/fix fecha por PR no GitHub, nunca por `git merge` local pra main.

**Fluxo:**

1. `git checkout -b feat/cad-XX-slug main`
2. Commitar atomicamente (1 commit por mudança lógica)
3. `git push -u origin <branch>`
4. `gh pr create --base main --head <branch> --title "..." --body "..."`
   - Title: 1 linha imperativa (`feat(wikilinks): support [[Note|Alias]] and #heading anchors`)
   - Body em pt-BR: **Resumo**, **Mudanças**, **Como testar**, **Riscos**, **Closes #CAD-XX**
5. Aguardar review (Fausto solo-dev: auto-merge OK quando CI verde — memory rule `feedback_auto_merge_when_ci_green`)
6. Pós-merge: `git checkout main && git pull && git branch -d <branch>`

**Stacked branches (CAD-21 stacked em CAD-20):**

- PR CAD-20 → main primeiro
- PR CAD-21 → CAD-20 (não → main) pra diff isolado
- Após CAD-20 mergear, GitHub auto-atualiza PR CAD-21 pra main

**Quando NÃO fazer PR:**

- Mudanças apenas em discipline files (SPRINT/DIARY/HUMAN/NOTION/SPECS/PLAN) → commit direto em main

---

## §5 — Pre-merge gates (rule #19 global)

Antes de mergear qualquer PR de feature:

1. Local tests verdes
2. CI verde
3. `/pre-merge-coverage` rodou + escreveu testes adversariais adicionais
4. `/codex-cross-review` rodou + issues P1/P2/P3 corrigidas in-PR
5. Notion ticket → 👀 Revisão

Apenas após confirmação humana → ✅ Concluída (rule #13).
