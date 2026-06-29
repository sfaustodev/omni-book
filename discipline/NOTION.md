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
| CAD-2 | Sidebar com árvore de pastas e busca | v0.1 | 🚧 Em obra | ⚡ Alta | 🐎 L | 8h | [SPECS/CAD-2.md](../SPECS/CAD-2.md) |
| CAD-3 | Painel central com modo Ler/Editar | v0.1 | 🎯 Pronta | ⚡ Alta | 🐎 L | 10h | [SPECS/CAD-3.md](../SPECS/CAD-3.md) |
| CAD-4 | Auto-save com debounce de 600ms | v0.1 | 🎯 Pronta | 📌 Média | 🐭 S | 3h | [SPECS/CAD-4.md](../SPECS/CAD-4.md) |
| CAD-5 | Atalhos globais | v0.1 | 📝 Refinando | 📌 Média | 🐭 S | 2h | [SPECS/CAD-5.md](../SPECS/CAD-5.md) |
| CAD-6 | Bug: from_id_source rename | v0.1 | ✅ Concluída | 🔥 Crítica | 🐜 XS | 0.5h | [SPECS/CAD-6.md](../SPECS/CAD-6.md) |
| CAD-7 | Modal Nova Nota | v0.2 | 🌱 Backlog | 📌 Média | 🐈 M | 4h | [SPECS/CAD-7.md](../SPECS/CAD-7.md) |
| CAD-8 | Modal Importar | v0.3 | 🌱 Backlog | 📌 Média | 🐈 M | 6h | [SPECS/CAD-8.md](../SPECS/CAD-8.md) |
| CAD-9 | Watcher de filesystem | v0.6 | 🌱 Backlog | 🌿 Baixa | 🐈 M | 5h | [SPECS/CAD-9.md](../SPECS/CAD-9.md) |
| CAD-10 | Spike: wikilinks clicáveis | v0.4 | 🌱 Backlog | 📌 Média | 🐭 S | 4h | [SPECS/CAD-10.md](../SPECS/CAD-10.md) |
| CAD-11 | Doc: README cross-platform | v1.0 | 🌱 Backlog | 🌿 Baixa | 🐭 S | 2h | [SPECS/CAD-11.md](../SPECS/CAD-11.md) |

---

## Active tickets — Sprint v1.1+ (post-v1.0 roadmap)

> Origem: brainstorm 2026-05-20 → `~/.claude/plans/greedy-napping-castle.md`
> Schema atual da DB tem CAD-13 a CAD-19 (não-indexados aqui) + CAD-20..CAD-25 abaixo.

### Sprint v1.1 (2026-05-20 → 2026-06-03) · Foundation

| ID | Title | Status | Prio | Size | Est | Notion URL |
|------|-------|--------|------|------|-----|------------|
| CAD-20 | Phase 1 — Obsidian link parity (parser + resolver + embeds) | 🎯 Pronta | ⚡ | 🐎 L | 16h | [36673ac79ddb81dea5bae6092629aa87](https://www.notion.so/36673ac79ddb81dea5bae6092629aa87) |
| CAD-21 | Phase 2 — Workspace refactor + CLI + MCP scaffolds | 🌱 Backlog | ⚡ | 🐘 XL | 24h | [36673ac79ddb81e6b9f0ee1450e0e1c9](https://www.notion.so/36673ac79ddb81e6b9f0ee1450e0e1c9) |
| CAD-25 | UI Design v2 — implementar handoff Claude Design em egui (Fase A análise paralela) | 🎯 Pronta | ⚡ | 🐘 XL | 30h | [36673ac79ddb81358d53caa9b7b4c46b](https://www.notion.so/36673ac79ddb81358d53caa9b7b4c46b) |

### Sprint v1.2 (2026-06-03 → 2026-06-17) · Discipline + UI core

| ID | Title | Status | Prio | Size | Est | Depende | Notion URL |
|------|-------|--------|------|------|-----|---------|------------|
| CAD-22 | Phase 3 — Daily notes + templates + discipline CLI/MCP | 👀 Revisão | ⚡ | 🐎 L | 18h | CAD-21 | [36673ac79ddb81719384cbc41c959717](https://www.notion.so/36673ac79ddb81719384cbc41c959717) |
| CAD-25 (Fase B) | UI Design v2 — implementação egui | 🔄 In progress | ⚡ | 🐘 XL | 22h | CAD-20 + análise A | slices 1-5 merged (#23-#26, #30) + ui-polish #28; falta só Slice 6 (ui_chat/ui_dictation). Slice 5 👀 aguarda teste humano (.app não registrado) |

### Sprint v1.3 (2026-06-17 → 2026-07-01) · AI + Power

| ID | Title | Status | Prio | Size | Est | Depende | Notion URL |
|------|-------|--------|------|------|-----|---------|------------|
| CAD-23 | Phase 4 — AI-native vault (umbrella: RAG + auto-tag + dictation + OCR) | 🔄 In progress | ⚡ | 🐘 XL | 40h | CAD-21 | [36673ac79ddb81d9b1a9f1df14a8fc9d](https://www.notion.so/36673ac79ddb81d9b1a9f1df14a8fc9d) |
| CAD-23.1 | RAG search (omninote-ai crate + ask CLI/MCP) | 👀 Revisão | ⚡ | 🐎 L | 12h | CAD-21 | mergeado PR #17 + hotfix #18 |
| CAD-23.2 | Auto-tag + summary | 👀 Revisão | ⚡ | 🐂 M | 6h | CAD-23.1 done | mergeado PR #19 (aguarda confirmação humana p/ ✅) |
| CAD-23.3 | Dictation Whisper | 🌱 Backlog | ⚡ | 🐎 L | 10h | CAD-23.1 done | — |
| CAD-23.4 | OCR PDF (leptess/tesseract) | 🌱 Backlog | 📌 | 🐂 M | 8h | CAD-23.1 done | — |
| CAD-24 | Phase 5 — Power automation (quick-capture + multi-vault + diff + JSON) | 🔄 Layer A 👀 (#29) | 📌 | 🐎 L | 20h | CAD-23 done | Layer A mergeado (#29): `omninote capture` + resolve_active core + --json todos verbos. 👀 aguarda teste humano. Layer B (hotkey global) spike-gated (Q-10) |

### Parallel work map

```
v1.1 ──── CAD-20 (sequencial, blocks all)
     │
     └─── CAD-25 Fase A (paralelo, read-only docs)
     │
     └─── CAD-21 (depende CAD-20)
                │
v1.2 ──── CAD-22 ⟂ CAD-25 Fase B (paralelo)
                │
v1.3 ──── CAD-23 ⟂ CAD-24 (paralelo)
```

### Notes

- Área (multi-select) ficou vazio em todos os 6 — Notion MCP wrapper rejeita CSV format, precisa investigar formato correto antes de bulk-set
- Fase select usa "Backlog" pra todos (não havia v1.1/v1.2/v1.3 nos options). Sprint info vive no content/título
- Subtasks dos umbrellas vivem como checklist dentro do content da página Notion (rule global #20 — não fragmentar em tickets por item)

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
