# CAD-20 — Phase 1 — Obsidian link parity (parser + resolver + embeds)

**Notion:** [https://www.notion.so/36673ac79ddb81dea5bae6092629aa87](https://www.notion.so/36673ac79ddb81dea5bae6092629aa87)
**Sprint:** v1.1 (2026-05-20 → 2026-06-03)
**Depende:** none — blocker para CAD-21/22/23/24/25 Fase B
**Critical files:** `src/wikilinks.rs`, `src/vault.rs`, new `src/resolver.rs`, `src/ui_editor.rs`

## Goal

Ver Notion page (link acima) — body Notion tem goal completo.

## Checklist

- [ ] Extend `wikilinks.rs` grammar: `WikilinkRef { target, alias, anchor }` where `anchor = Heading | Block | None`
- [ ] Parse `[[Note|Alias]]` — split on `|`, render alias as link text
- [ ] Parse `[[Note#Heading]]` — split on `#`, navigate on click
- [ ] Parse `[[Note#^block-id]]` — split on `#^`, recognize `^id` lines as anchors
- [ ] Parse `[[folder/path/note]]` — path-based resolution
- [ ] New `resolver.rs`: order = exact filename → path → frontmatter `aliases` → case-insensitive → unresolved
- [ ] `![[Note]]` and `![[Note#Heading]]` embed-of-note rendering (separate from File variant)
- [ ] Inline `#tag` parser (body) feeding existing tag chip filter
- [ ] Vault index built on load, kept fresh by watcher
- [ ] Adversarial fuzz on `|/#/^` combinations


## Verification

Open `~/Documents/Obsidian Vault`. `cargo test wikilinks:: resolver::` green. Manual: clicar em [[SPEC_V2 - NdA|spec da NdA]] navega + renderiza alias.

## Source

- Plano de origem: `~/.claude/plans/greedy-napping-castle.md`
- Sprint context: `discipline/SPRINT.md`
- Notion ticket (source of truth pra status): https://www.notion.so/36673ac79ddb81dea5bae6092629aa87
