# NOTION — Tracker index

> Substitui o JIRA.md. Esta é a fonte canônica do projeto. Ver Notion pra updates em tempo real.

**Workspace board:** [🚢 Caderno de Bordo](https://app.notion.com/p/35373ac79ddb81fa96bcdb9991425508)
**Database:** [Tarefas](https://app.notion.com/p/b9118e56f7d941d7a542e92f425c5dab)
**Data source ID:** `collection://200a086e-4f9e-4d44-a2a0-acb6906def89`

---

## Schema (Notion → local)

| Notion | Local |
|--------|-------|
| ID auto (`CAD-XX`) | nome do arquivo em `SPECS/` |
| Tarefa (title) | `# Title` no spec |
| Status | tabela em SPRINT.md §1 |
| Prioridade | tabela em SPRINT.md §1 |
| Tamanho / Estimativa (h) | tabela em SPRINT.md §1 |
| Fase (v0.1..v1.0) | grupo em SPRINT.md §1 |
| Conteúdo da página | corpo de `SPECS/CAD-XX.md` |

---

## Active tickets

| ID | Title | Phase | Status | Priority | Size | Estimate | Spec |
|------|-------|-------|--------|----------|------|----------|------|
| CAD-2 | Sidebar com árvore de pastas e busca | v0.1 | 🚧 Em obra | ⚡ Alta | 🐎 L | 8h | [SPECS/CAD-2.md](SPECS/CAD-2.md) |
| CAD-3 | Painel central com modo Ler/Editar | v0.1 | 🎯 Pronta | ⚡ Alta | 🐎 L | 10h | [SPECS/CAD-3.md](SPECS/CAD-3.md) |
| CAD-4 | Auto-save com debounce de 600ms | v0.1 | 🎯 Pronta | 📌 Média | 🐭 S | 3h | [SPECS/CAD-4.md](SPECS/CAD-4.md) |
| CAD-5 | Atalhos globais | v0.1 | 📝 Refinando | 📌 Média | 🐭 S | 2h | [SPECS/CAD-5.md](SPECS/CAD-5.md) |
| CAD-6 | Bug: from_id_source rename | v0.1 | ✅ Concluída | 🔥 Crítica | 🐜 XS | 0.5h | [SPECS/CAD-6.md](SPECS/CAD-6.md) |
| CAD-7 | Modal Nova Nota | v0.2 | 🌱 Backlog | 📌 Média | 🐈 M | 4h | [SPECS/CAD-7.md](SPECS/CAD-7.md) |
| CAD-8 | Modal Importar | v0.3 | 🌱 Backlog | 📌 Média | 🐈 M | 6h | [SPECS/CAD-8.md](SPECS/CAD-8.md) |
| CAD-9 | Watcher de filesystem | v0.6 | 🌱 Backlog | 🌿 Baixa | 🐈 M | 5h | [SPECS/CAD-9.md](SPECS/CAD-9.md) |
| CAD-10 | Spike: wikilinks clicáveis | v0.4 | 🌱 Backlog | 📌 Média | 🐭 S | 4h | [SPECS/CAD-10.md](SPECS/CAD-10.md) |
| CAD-11 | Doc: README cross-platform | v1.0 | 🌱 Backlog | 🌿 Baixa | 🐭 S | 2h | [SPECS/CAD-11.md](SPECS/CAD-11.md) |

---

## Sync workflow

1. **Pull from Notion** (no início da sessão se mudou): `notion-search` na database, atualizar tabela acima + specs.
2. **Push to Notion** (no fim da sessão): atualizar Status via `notion-update-page` quando task muda de estado.
3. **Fechar task**: somente após confirmação humana escrita no chat. Mover pra `✅ Concluída` no Notion + atualizar tabela.

---

## Status legend

| Notion | Significado |
|--------|------------|
| 🌱 Backlog | Não começou ainda |
| 📝 Refinando | Spec sendo detalhada, não pronto pra começar |
| 🎯 Pronta | Spec OK, pode começar agora |
| 🚧 Em obra | Em desenvolvimento |
| 👀 Revisão | Code complete, aguarda teste humano |
| ✅ Concluída | Testado e aprovado |
| 🧊 Congelada | Pausada |
| 📦 Arquivada | Não vai ser feita |
