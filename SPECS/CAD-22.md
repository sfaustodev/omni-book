# CAD-22 — Phase 3 — Daily notes + templates + discipline CLI/MCP

**Notion:** [https://www.notion.so/36673ac79ddb81719384cbc41c959717](https://www.notion.so/36673ac79ddb81719384cbc41c959717)
**Sprint:** v1.2 (2026-06-03 → 2026-06-17)
**Depende:** CAD-21 done
**Critical files:** new `crates/omninote-core/src/{templates,daily,discipline}.rs` + CLI/MCP verb additions

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

- [ ] `templates.rs` — `Templates/` folder, placeholders `{{date:YYYY-MM-DD}}`, `{{time}}`, `{{title}}`
- [ ] `daily.rs` — `omninote daily` cria `Daily/YYYY-MM-DD.md` do template se missing
- [ ] `discipline.rs` — typed read/append: DIARY/SPRINT/HUMAN/PLAN/JIRA/NOTION
- [ ] CLI: `omninote diary append`, `omninote human ask`, `omninote ticket scrum-X status`, `omninote daily`, `omninote template apply`, `omninote discipline sprint show|plan show`
- [ ] MCP expoe mesmos verbs como tools


## Verification

`omninote daily` cria today's `Daily/YYYY-MM-DD.md` do `Templates/daily.md`. `omninote diary append "test"` escreve entry matching discipline format. Cowork chama MCP tool `discipline_diary_append` com sucesso.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb81719384cbc41c959717
