# SPRINT — OmniNote v1.2 (Daily + Discipline)

> **Sprint goal:** entregar daily notes + templates + discipline CLI/MCP (CAD-22), tornando o `omninote` usável no fluxo diário e expondo sacred files via tools pro Cowork/Claude Desktop.
> **Sprint window:** 2026-06-03 → 2026-06-17 (2 semanas)
> **Tracker:** [Notion · 🚢 Caderno de Bordo](https://app.notion.com/p/35373ac79ddb81fa96bcdb9991425508) — ver [NOTION.md](NOTION.md) pro índice local.
> **Plano de origem:** `~/.claude/plans/greedy-napping-castle.md` (seção "Next session execution plan — 2026-05-23")

**Sprint anterior (v1.1 Foundation):** ✅ CAD-20 (link parity) + ✅ CAD-21 (workspace + CLI + MCP) + ✅ CAD-25 Fase A (UI v2 análise) — todos mergeados em main, binários instalados.

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

## §1 — Sprint v1.2 (atual): Daily + Discipline

| # | ID | Tarefa | Status | Prio | Estimativa | Notas |
|---|------|--------|--------|------|------------|-------|
| 1 | CAD-22 | Phase 3 — Daily notes + templates + discipline CLI/MCP | 👀 Revisão | ⚡ | 18h | merged #15 — aguarda teste humano macOS |
| 2 | CAD-25 (Fase B) | UI Design v2 — implementação egui | 🌱 Backlog | ⚡ | ~22h | bloqueado em Q-01..Q-30 (UI_DESIGN_v2.md) |

### Pull-forward paralelo (sessão 2026-05-31) — v1.3 + salvage

3 PRs per-ticket abertos via fan-out de 3 agentes (worktree-isolados) + trio gate completo (security/coverage/review, 3-way Claude+Codex+agy, 260 tests):

| ID | PR | Status | Escopo |
|----|-----|--------|--------|
| CAD-23 | [#20](https://github.com/sfaustodev/omni-book/pull/20) | ✅ merged (👀 Revisão Notion) | crate `omninote-ai`: LlmProvider + llm.toml scaffold |
| CAD-24 | [#21](https://github.com/sfaustodev/omni-book/pull/21) | 👀 Revisão (CI) | CLI `--json`, multi-vault, `diff --since` |
| CAD-25 | [#22](https://github.com/sfaustodev/omni-book/pull/22) | 👀 Revisão (CI) | tema Obsidian, panic hook, OpenDyslexic, Cmd |

Auto-merge desabilitado no repo → merge manual com CI verde. Branches obsoletos (`v04-v10`/`swiss`/`q01-q02`) NÃO pruned (humano escolheu ship sem prune).

### Dependency graph

```
CAD-22 (daily + discipline)
  └─→ Sprint v1.3 (CAD-23 AI, CAD-24 power) — todos dependem do core estável

CAD-25 Fase B (UI v2) ⟂ paralelo
  └─ blocked on: Q-01..Q-30 batch decision com Fausto
```

### Parallel work strategy

- **CAD-22** é o caminho principal. 6 fases (templates → daily → discipline → CLI → MCP → ship).
- **CAD-25 Fase B** desbloqueia quando Fausto responder batch Q-01..Q-30. Antes disso, fica em standby.

---

## §1.5 — Sprint anterior (v1.1 Foundation) — ✅ shipped

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
