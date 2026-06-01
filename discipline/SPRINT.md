# SPRINT — OmniNote v1.3 (AI-native + Power)

> **Sprint goal:** chat-com-vault funcional via RAG semântico (CAD-23.x slices) + power automation (CAD-24). User pode perguntar "where did I discuss X?" e Claude responde citando `[[wikilinks]]`.
> **Sprint window:** 2026-06-17 → 2026-07-01 (2 semanas, fatiado em CAD-23.1/.2/.3/.4 + CAD-24)
> **Tracker:** [Notion · 🚢 Caderno de Bordo](https://app.notion.com/p/35373ac79ddb81fa96bcdb9991425508) — ver [NOTION.md](NOTION.md) pro índice local.
> **Plano de origem:** `~/.claude/plans/greedy-napping-castle.md` (seção "Sprint v1.3 — CAD-23 + CAD-24")

**Sprint anterior (v1.2 Daily + Discipline):** ✅ CAD-22 mergeado main (PR #16), Notion 👀 Revisão aguardando teste humano. Discipline CLI/MCP rodando (omninote daily, diary append, human ask, ticket, discipline show + tools MCP equivalentes).

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

## §1 — Sprint v1.3 (atual): AI-native + Power

CAD-23 foi fatiado em 4 subtasks shipáveis (per decisão AskUserQuestion 2026-05-23, plano em greedy-napping-castle.md).

| # | ID | Tarefa | Status | Prio | Estimativa | Notas |
|---|------|--------|--------|------|------------|-------|
| 1 | CAD-23.1 | RAG search (omninote-ai + ask CLI/MCP) | ✅ Done (PR #17 + hotfix #18) | ⚡ | 12h | mergeado main |
| 2 | CAD-23.2 | Auto-tag + summary | ✅ Done (PR #19) | ⚡ | 6h | mergeado main |
| 3 | CAD-23.3 | Dictation Whisper (local) | 🌱 Backlog | ⚡ | 10h | depende 23.1 ✅ · whisper-rs offline |
| 4 | CAD-23.4 | OCR PDF (local) | 🌱 Backlog | 📌 | 8h | depende 23.1 ✅ · tesseract local |
| 5 | CAD-24 | Power automation | 🔄 Parcial | 📌 | 20h | multi-vault + diff + `--json` portados pro main; falta só o daemon `omninote-capture` (hotkey global) |
| 6 | CAD-25 (Fase B) | UI Design v2 — implementação egui | 🔄 Em execução | ⚡ | ~50h | **DESBLOQUEADO** (Q-01..Q-30 resolvidas, doc §10). Fatiado em 6 slices. Slice 1 (fundação) gated pelo trio, em PR. |

### CAD-25 Fase B — slices

| Slice | Escopo | Status |
|-------|--------|--------|
| 1 | Fundação: `Theme` struct + 4 presets + AppConfig UI-v2 fields + status bar | 🔄 trio-gated, em PR |
| 2 | Shell 3-painéis + chrome (titlebar/breadcrumb/tabs/right-rail) | 🌱 |
| 3 | Renderer markdown custom (`md_render.rs`, hover-preview/embeds/#tags) | 🌱 |
| 4 | Overlays (command palette, settings, toasts, calendar, onboarding) | 🌱 |
| 5 | Views tipadas (sprint/diary/human/tickets/timeline/daily) | 🌱 |
| 6 | AI surfaces (chat RAG real) + dictation hidden + a11y polish | 🌱 |

### Dependency graph

```
CAD-23.1 RAG ✅ → CAD-23.2 auto-tag ✅
                  ├─→ CAD-23.3 dictation (whisper-rs local)  ⟂ paralelo
                  └─→ CAD-23.4 OCR (tesseract local)         ⟂ paralelo
CAD-24 power: multi-vault/diff/json ✅ no main · daemon pendente
CAD-25 Fase B: desbloqueado → slices 1→6 incrementais
```

### Parallel work strategy

- **CAD-25 Fase B** é o caminho principal agora (desbloqueado). Incremental, 1 PR por slice, gate #26 por slice.
- **CAD-23.3/.4** (dictation/OCR local) podem rodar em paralelo quando o foco voltar pra AI — módulos disjuntos em `omninote-ai`.
- **CAD-24 daemon** (`omninote-capture`) é a última peça de power automation.

---

## §1.5 — Sprint anterior (v1.2 Daily + Discipline) — ✅ shipped

| ID | Tarefa | Status | Notas |
|------|--------|--------|-------|
| CAD-22 | Phase 3 — Daily/Templates/Discipline CLI/MCP | ✅ Done (PR #16) | 7 tools MCP novos, ~780 LOC core, 60 tests novos |

---

## §1.6 — Sprint v1.1 Foundation — ✅ shipped

| ID | Tarefa | Status | Notas |
|------|--------|--------|-------|
| CAD-20 | Phase 1 — Obsidian link parity | ✅ Done | merged main, 93 tests |
| CAD-21 | Phase 2 — Workspace refactor + CLI + MCP | ✅ Done | merged main, 4 MCP tools |
| CAD-25 (Fase A) | UI Design v2 — análise + plano de port | ✅ Done | docs/UI_DESIGN_v2.md (2756 linhas) |

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
