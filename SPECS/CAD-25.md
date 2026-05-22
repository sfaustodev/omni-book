# CAD-25 — UI Design v2 — implementar handoff Claude Design em egui

**Notion:** [https://www.notion.so/36673ac79ddb81358d53caa9b7b4c46b](https://www.notion.so/36673ac79ddb81358d53caa9b7b4c46b)
**Sprint:** v1.1 (Fase A análise) → v1.2 (Fase B implementação)
**Depende:** CAD-20 parcial (renderização de novos wikilinks)
**Critical files:** `docs/UI_DESIGN_v2.md`, `crates/omninote-gui/src/ui_*.rs` (split/extend), `docs/design/omninote/` (read-only handoff source)

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

**Fase A análise** (Sprint v1.1, paralelo):
  - [ ] Ler `docs/design/omninote/project/styles/07-omninote-obsidian.jsx` (47KB) linha a linha
  - [ ] Mapear cada widget JSX → equivalente egui
  - [ ] Extrair color tokens, spacing, typography → `AppConfig` extensions
  - [ ] Identificar `src/ui_*.rs` novos vs extensões
  - [ ] Escrever `docs/UI_DESIGN_v2.md` com plano de port pixel-perfect
**Fase B implementação** (Sprint v1.2):
  - [ ] Sidebar collapsible Daily/Inbox/Discipline/Projects + tag pane toggle
  - [ ] Editor center: inline `[[wikilink]]` hover preview, `![[Note]]` embed first-200, `![[image]]` thumb, `![[pdf]]` first-page preview, inline `#tag` chips, heading TOC
  - [ ] Right panel (320px toggle) — 3 tabs: Backlinks/Outline/AI Chat
  - [ ] Command palette `Ctrl+P` fuzzy
  - [ ] Slash menu extend: AI/templates/discipline actions
  - [ ] Quick-capture popup UI (binding CAD-24 daemon)
  - [ ] Toast queue bottom-right
  - [ ] Settings extend: LLM provider, hotkey config, daily opt-in, a11y presets
  - [ ] Discipline-typed views (SPRINT/DIARY/HUMAN/PLAN bespoke widgets)
  - [ ] Tickets panel (NOTION+JIRA merged)
  - [ ] Calendar widget (daily picker)
  - [ ] Tag explorer (counts + filter)
  - [ ] Timeline view (snapshot diff render)


## Verification

Screenshot side-by-side mockup vs running app → overlay <2% pixel diff. `Ctrl+P` abre palette. Dark/light themes corretos. Todos atalhos do mockup funcionam.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb81358d53caa9b7b4c46b
