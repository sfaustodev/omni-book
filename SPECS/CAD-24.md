# CAD-24 — Phase 5 — Power automation (quick-capture + multi-vault + diff + JSON)

**Notion:** [https://www.notion.so/36673ac79ddb813784afe6f9920adedc](https://www.notion.so/36673ac79ddb813784afe6f9920adedc)
**Sprint:** v1.3 (2026-06-17 → 2026-07-01)
**Depende:** CAD-21 done
**Critical files:** new bin `omninote-capture/`, `~/.config/omninote/vaults.toml`, CLI extensions

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

- [ ] Quick-capture global hotkey daemon (`omninote-capture` bin) — macOS via `tao`/NSEvent tap, Linux via `global-hotkey` crate
- [ ] Popup 1-line textarea → appends to `Inbox.md` → close sem stealing focus > 2s
- [ ] Multi-vault switcher: `omninote vault switch|list|add` → `vaults.toml`
- [ ] Snapshot diff: `omninote diff [--since 1d|7d]` git-aware, graceful se não-git
- [ ] JSON output: `--json` flag em todos verbs
- [ ] Docs `docs/CLI_RECIPES.md` com pipelines exemplos (`jq`, `xargs`)


## Verification

Quick-capture hotkey → `Inbox.md` cresce 1 linha. `omninote vault list` mostra todos. `omninote diff --since 1d --json | jq '.changed[]'` produz lista válida.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb813784afe6f9920adedc
