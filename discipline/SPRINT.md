# SPRINT — OmniNote v0.1 → v0.3

> **Sprint goal:** entregar OmniNote utilizável end-to-end — sidebar com tree, editor markdown com auto-save, modais (nova nota / importar / settings / confirm), atalhos globais, import de PDF + chats Claude.
> **Sprint window:** 2026-04-28 → 2026-05-09 (1 sprint × 2 semanas)
> **Tracker:** [Notion · 🚢 Caderno de Bordo](https://app.notion.com/p/35373ac79ddb81fa96bcdb9991425508) — ver [NOTION.md](NOTION.md) pro índice local.

---

## §0 — Hard rules (não-negociáveis)

1. **Vault em filesystem é fonte da verdade.** Nunca cachear estado em SQLite/JSON único — quebra compatibilidade Obsidian e MCP filesystem.
2. **`.omninote/` (não `.obsidian/`).** Coexistência com Obsidian no mesmo vault sem conflito.
3. **Frontmatter YAML compatível com Obsidian.** Nunca quebrar o parser do Obsidian — testar em vault compartilhado se houver dúvida.
4. **`linked_note` por ID, não por path.** Sobrevive a renomear/mover.
5. **Sem servidor MCP próprio.** O vault é só um folder de `.md` — qualquer MCP filesystem oficial lê/escreve.
6. **Nunca commitar `.omninote/` ou `_attachments/` de vaults reais.** São dados do usuário, não do projeto.
7. **`from_id_salt` (não `from_id_source`).** API renomeada em egui 0.29 — usar a nova consistentemente.
8. **`active_note: Option<Note>` clonado.** Nunca usar `active_idx: usize` — índices invalidam em mutação.
9. **CommonMarkCache em `OmniNoteApp.md_cache`.** Nunca recriar por frame.
10. **Tests inline (`#[cfg(test)]`).** Não criar `tests/` dir até `lib.rs` existir.

---

## §1 — Ordered task list (atual)

| # | ID | Tarefa | Fase | Status | Prio | Notas |
|---|------|--------|------|--------|------|-------|
| 1 | CAD-6 | Bug `from_id_source` → `from_id_salt` | v0.1 | ✅ | 🔥 | Resolvido no rebase pra 0.29 |
| 2 | CAD-2 | Sidebar com árvore de pastas e busca | v0.1 | 👀 | ⚡ | feat/omninote-v01 — pendente teste humano |
| 3 | CAD-3 | Painel central modo Ler/Editar | v0.1 | 👀 | ⚡ | feat/omninote-v01 — pendente teste humano |
| 4 | CAD-4 | Auto-save 600ms debounce | v0.1 | 👀 | 📌 | feat/omninote-v01 — pendente teste humano |
| 5 | CAD-5 | Atalhos globais (Ctrl+N/E/K/,) | v0.1 | 👀 | 📌 | feat/omninote-v01 — pendente teste humano |
| 6 | CAD-7 | Modal Nova Nota grid 2×3 | v0.2 | 👀 | 📌 | feat/omninote-v01 — pendente teste humano |
| 7 | CAD-8 | Modal Importar (PDF/JSON/Artefato) | v0.3 | 👀 | 📌 | feat/omninote-v01 — pendente teste humano |
| 8 | — | CI pipeline (lint/test/build/audit) | v0.1 | 👀 | 📌 | feat/omninote-v01 — pendente run remoto |

**Backlog próximo (próximo sprint):**

| ID | Tarefa | Fase | Prio | Estimativa |
|------|--------|------|------|------------|
| CAD-10 | Spike: wikilinks clicáveis | v0.4 | 📌 | 4h |
| CAD-9 | Watcher de filesystem (notify) | v0.6 | 🌿 | 5h |
| CAD-11 | README cross-platform + MCP | v1.0 | 🌿 | 2h |

---

## §2 — Definition of Done

Pra cada CAD-XX considerar pronto somente após:

1. Código compila sem warnings (`cargo build`)
2. `cargo clippy --all-targets -- -D warnings` passa
3. Tests aplicáveis passam (`cargo test`)
4. **Humano testou em macOS local e confirmou no chat** ("testado, pode fechar")
5. Notion task movida pra `✅ Concluída` via MCP
6. `JIRA.md` (aqui: NOTION.md) atualizado com status final

---

## §3 — Branch strategy

- `main` — protegida; recebe merge de feat branches via PR
- `feat/omninote-vXX` — uma branch por fase (v0.1, v0.2, v0.3)
- Bugs descobertos em testes do humano = commits adicionais na **mesma** branch da feature, não SCRUM novo
- Discipline files (SPRINT/DIARY/HUMAN/NOTION/SPECS) **vivem em main** — atualizadas a cada sessão

---

## §4 — PR-first workflow (não merge direto pra main)

**Regra:** toda feature/fix branch fecha por PR no GitHub, nunca por `git merge` local pra main. Fausto quer revisar diff no GitHub UI pra aprender a code review.

**Fluxo:**

1. Criar branch off main: `git checkout -b feat/<scope> main`
2. Commitar trabalho atomicamente (1 commit por mudança lógica)
3. Push: `git push -u origin <branch>`
4. Abrir PR via `gh pr create --base main --head <branch> --title "..." --body "..."`
   - Title: 1 linha imperativa (ex.: `feat(theme): apply Swiss/Bauhaus dark design`)
   - Body em pt-BR com seções: **Resumo**, **Mudanças**, **Como testar**, **Riscos**
5. Aguardar Fausto revisar + mergear no UI ("Merge pull request" → squash ou normal)
6. Após merge, atualizar local: `git checkout main && git pull && git branch -d <branch>`

**Branches stacked (B saiu de A antes de A mergear):**

- PR de A → main primeiro
- PR de B → A (não → main) pra diff isolado
- Após A mergear em main, GitHub auto-atualiza o PR B pra apontar pra main

**Quando NÃO fazer PR:**

- Mudanças apenas em discipline files (SPRINT/DIARY/HUMAN/NOTION/SPECS) → commit direto em main
- Hotfix crítico em produção (não aplica ainda — sem prod)

**Por que:**

- Fausto está aprendendo code review sistemático
- PR cria audit trail GitHub (CI run, comments, decisões)
- Revisão antes de merge previne bugs que só aparecem no smoke
- Branch protection futura no GitHub (require PR + approval) fica trivial de habilitar
