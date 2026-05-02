# HUMAN — Perguntas pro humano

> Apenas decisões irreversíveis, contratos externos, segurança ou conflitos com SPRINT.md.
> Tom: direto, em pt-BR, opções com tradeoffs explícitos.

---

## Open questions

(none)

---

## Resolved

### Q-04 · Ctrl+= autoformat scope · raised 2026-05-01 · resolved 2026-05-02 · context: CAD-5
**Resposta humano (Fausto, 2026-05-02):** **(a)** — manter só linha atual por enquanto.
**Aplicação:** zero código novo. Comportamento atual mantido. (b) arquivado pra v2.0+ se demanda real surgir.

---

### Q-03 · CAD-9 watcher conflict UX · raised 2026-05-01 · resolved 2026-05-02 · context: CAD-9
**Resposta humano (Fausto, 2026-05-02):** **(b)** confirmado — modal "Recarregar (perde edits) / Manter edits (sobrescreve no save)".
**Aplicação:** já implementado no `feat/omninote-v06-watcher` (mergeado em main `b327207`). `external_change_pending: bool` em `OmniNoteApp`. Modal renderizado em `ui_modals::show_modal_external_change`. Self-write window 400ms evita reload-loop dos próprios saves.

---

### Q-02 · Atalhos: Ctrl literal vs `Modifiers::COMMAND` · raised 2026-05-01 · resolved 2026-05-02 · context: CAD-5
**Resposta humano (Fausto, 2026-05-02):** **(a)** — Mac usa Cmd. Trocar pra `Modifiers::COMMAND` (auto-mapeia Cmd no mac, Ctrl no resto).
**Aplicação:** branch `feat/omninote-q01-q02-cmd-migrate`. Substituído `i.modifiers.ctrl` → `i.modifiers.command` em:
- `src/app.rs:305-308` (Cmd+N, Cmd+E, Cmd+,, Cmd+Shift+D)
- `src/ui_editor.rs:193` (Cmd+= autoformat)
- `src/ui_sidebar.rs:67` (Cmd+K busca)

Sem novo teste — egui mapeia internamente. Validação manual macOS pendente.

---

### Q-01 · Renomear `.caderno/` → `.omninote/` quebra vaults antigos · raised 2026-05-01 · resolved 2026-05-02 · context: rebrand
**Resposta humano (Fausto, 2026-05-02):** "se for útil renomeia, se for deletar só deleta" → **migrate, não delete** (config.json tem dark_mode + last_active úteis).
**Aplicação:** branch `feat/omninote-q01-q02-cmd-migrate`. Em `Vault::open`:
- Se `.caderno/` existe e `.omninote/` não → `fs::rename(.caderno → .omninote)` preserva config
- Se ambos existem (raro) → drop `.caderno/`, `.omninote/` ganha
- Se só `.omninote/` existe → fluxo normal

2 testes novos: `migrates_legacy_caderno_dir_to_omninote` + `drops_legacy_caderno_when_omninote_already_exists`.
