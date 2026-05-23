# CAD-21 — Phase 2 — Workspace refactor + CLI + MCP scaffolds

**Notion:** [https://www.notion.so/36673ac79ddb81e6b9f0ee1450e0e1c9](https://www.notion.so/36673ac79ddb81e6b9f0ee1450e0e1c9)
**Sprint:** v1.1 (2026-05-20 → 2026-06-03)
**Depende:** CAD-20 done
**Critical files:** `Cargo.toml` workspace, new `crates/omninote-{core,gui,cli,mcp}/`

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

- [ ] Cargo workspace restructure: 4 crates
- [ ] Move `vault.rs`, `wikilinks.rs`, `frontmatter.rs`, `resolver.rs`, `search.rs` into `omninote-core`
- [ ] `omninote-cli` (clap derive): `vault info`, `note search QUERY`, `link unresolved`, `backlinks FILE`
- [ ] `omninote-mcp` (rmcp): expoe mesmos verbs como MCP tools
- [ ] Binários aceitam `--vault <PATH>` ou `OMNINOTE_VAULT` env
- [ ] GUI builds + runs unchanged (zero regressão)
- [ ] CI updated para `cargo test --workspace`


## Verification

`omninote --vault ~/Documents/Obsidian\ Vault vault info` retorna same files/folders/size que `obsidian vaults verbose`. `omninote-mcp` registrado em `claude_desktop_config.json` + callable via Claude Desktop tool list.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb81e6b9f0ee1450e0e1c9
