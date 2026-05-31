# HUMAN — Perguntas pro humano

> Apenas decisões irreversíveis, contratos externos, segurança ou conflitos com SPRINT.md.
> Tom: direto, em pt-BR, opções com tradeoffs explícitos.

---

## Open questions

_(nenhuma — Q-01 a Q-08 resolvidas em batch 2026-05-22)_

---

## Resolved

### Q-01 · Renomear `.caderno/` → `.omninote/` quebra vaults antigos · raised 2026-05-01 · resolved 2026-05-22
**Decisão (humano):** (a) — usar só `.omninote/` daqui pra frente, ignorar vaults pré-rename.
**Razão:** humano é o único usuário e nada foi commitado em vault de produção. Blast radius zero, compat não se justifica.
**Followed-up in:** sem mudança de compat code; `.caderno/` deixa de ser lido.
### Q-02 · Atalhos: `Ctrl+...` ou `Modifiers::COMMAND` · raised 2026-05-01 · resolved 2026-05-22
**Decisão (humano):** (a) — trocar tudo pra `Modifiers::COMMAND` (Cmd no mac, Ctrl no resto, mapeia automático).
**Razão:** convenção Mac importa, custo de troca é zero.
**Followed-up in:** CAD-5 — substituir `Ctrl` literal por `Modifiers::COMMAND` nos atalhos.

### Q-03 · CAD-9 watcher: conflito quando humano edita externamente nota ativa · raised 2026-05-01 · resolved 2026-05-22
**Decisão (humano):** (b) — modal de aviso "arquivo mudou no disco: recarregar (perde edits) ou ignorar (sobrescreve no save)?".
**Razão:** última-grava-vence silencioso é o pior outcome (perde dado sem o usuário saber).
**Followed-up in:** CAD-9 (watcher) implementa o modal de conflito.

### Q-04 · `Ctrl+=` autoformat: só linha atual ou propagar pra aninhados · raised 2026-05-01 · resolved 2026-05-22
**Decisão (humano):** (a) — manter só linha atual. (b) suporte a referências de linha (mini-spreadsheet) arquivado pra v2.0.
**Razão:** escopo minimalista do MVP; (b) é feature creep.
**Followed-up in:** CAD-5 mantém escopo atual. (b) registrado como ideia v2.0.

### Q-05 · Coverage gate em CI — fail PR ou warn-only · raised 2026-05-13 · resolved 2026-05-22
**Decisão (humano):** (a) — manter `cargo llvm-cov --fail-under-lines 90` falhando o PR. CI vermelho = para tudo e arruma.
**Razão:** disciplina forte intencional; humano trata CI red como bloqueante absoluto. Sentir o atrito antes de afrouxar.
**Nota:** não foi criada nenhuma trava nova de processo — gate permanece como já estava. O "para tudo e arruma" é operado pela skill `ci-red-triage`.
**Followed-up in:** CAD-12 mantém o job `coverage` como está.

### Q-06 · `lib.rs` split — adiar pra v0.4 ou nunca · raised 2026-05-13 · resolved 2026-05-22
**Decisão (humano):** (a) — fazer o split junto da v0.4 (CAD-10), aproveitando o trabalho de parser de wikilinks pra estabilizar boundaries.
**Razão:** split é cirúrgico (mover `mod x;` de `main.rs` pra `lib.rs` + ajustar `[lib]`/`[bin]` no Cargo.toml) e v0.4 já é trabalho de parser. Casa natural; desbloqueia integration tests (egui_kittest) e doc tests.
**Followed-up in:** CAD-10 (v0.4 wikilinks) inclui o split do `lib.rs` como side-effect. §0 #10 (proibição de `tests/` até `lib.rs` existir) destrava após.

### Q-07 · `vault::import_attachment` aceita qualquer extensão sem allow-list · raised 2026-05-13 · resolved 2026-05-22
**Decisão (humano):** (a) — não fazer nada agora. Confiar no rfd FileDialog (humano clica → já validou). Gap documentado em comentário + teste.
**Razão:** escopo atual é offline + single-user; threat model não justifica complexidade.
**Trigger pra revisar:** primeiro callsite não-humano de `import_attachment` (watcher CAD-9 ou drag-drop). Aí o allow-list (b) deixa de ser paranoia e vira defense-in-depth.
**Followed-up in:** sem mudança de código. Revisar no CAD-9 se surgir caller automático.

### Q-08 · `Vault::open` quando root é arquivo (não diretório) · raised 2026-05-13 · resolved 2026-05-22
**Decisão (humano):** (b) — adicionar `if root.is_file() { return Err("path is a file, expected dir".into()); }` no início de `Vault::open`. 1 linha + 1 teste, risco zero.
**Razão:** comportamento atual retorna `Ok` com vault vazio (os `fs::create_dir_all` usam `let _ =` e engolem o erro) — pegadinha confusa. Barato de prevenir.
**Followed-up in:** PR follow-up dedicado (não bundlar no CAD-12), pra manter histórico limpo.
### Q-09 · agy (`--dangerously-skip-permissions`) bloqueado pelo classifier no trio gate · raised 2026-05-31 · resolved 2026-05-31
**Decisão (humano):** aprovar a escalation + re-rodar coverage+review com agy pro consenso 3-way estrito.
**Razão:** os 3 agentes cobrem blindspots distintos; agy achou 2 HIGH (char/byte index) que Claude+Codex passaram batido. Valeu o 3º ângulo.
**Followed-up in:** agy rodou no 2º try (background), findings fixados, PRs abertos.
