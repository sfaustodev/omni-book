# HUMAN — Perguntas pro humano

> Apenas decisões irreversíveis, contratos externos, segurança ou conflitos com SPRINT.md.
> Tom: direto, em pt-BR, opções com tradeoffs explícitos.

---

## Open questions

### Q-12 · Ignorar RUSTSEC-2026-0194/0195 (quick-xml DoS, transitiva do egui 0.29) no cargo audit · raised 2026-07-10 · context: CI red pós-merge PR #32
**Por que eu perguntei:** decisão de segurança (filtro (c)) tomada em modo decide-and-flag (rule #7) pra destravar o CI de main, que ficou vermelho quando o advisory DB atualizou. As duas cópias vulneráveis de quick-xml (<0.41) são transitivas do stack egui 0.29 pinado, Linux-only, e nunca veem XML de atacante (uma é proc-macro compile-time via wayland-scanner; outra parseia D-Bus local via atspi/accesskit). Nenhum pai aceita ≥0.41 sem bump major do egui (0.29→0.32+), que o CLAUDE.md pina de propósito (egui_commonmark 0.18 ↔ egui 0.29).
**Options que considerei:**
- (a) Ignore documentado por advisory-ID em `.cargo/audit.toml` (reversível, 1 linha por ID), removível no upgrade do egui — **o que apliquei**
- (b) Bump major do stack egui agora (0.29→0.32+) só pra limpar o audit — dias de trabalho de migração de API, fora de escopo de hotfix
- (c) Desligar o job Security Audit — inaceitável, perde cobertura de advisories reais
**Minha escolha (aplicada):** (a). O risco real é ~zero pro threat model (app desktop offline single-user), e o ignore é rastreado + auto-documentado.
**Ask:** (a) tá OK como estado permanente até o upgrade do egui? Ou você prefere priorizar o bump do egui (b) num ticket próprio já?

---

## Resolved

### Q-09 · Adicionar tokio + omninote-ai ao crate GUI (Slice 6 chat) · raised 2026-06-27 · resolved 2026-06-27 · context: CAD-25b Slice 6
**Por que eu perguntei:** quebra o invariante "GUI slim" (§0 #11) e muda o contrato de build — tokio (pin do workspace = `full`) + omninote-ai no binário GUI tem custo de tamanho.
**Decisão (Fausto):** APROVADO. O chat RAG-real (ui_chat) precisa do async runtime + omninote-ai no GUI. Desbloqueia o Slice 6.
**A aplicar:** ao implementar Slice 6 — `omninote-ai.workspace = true` + `tokio.workspace = true` em `crates/omninote-gui/Cargo.toml`. Reavaliar `full` vs `["rt","macros"]` (binary size) no momento.

### Q-10 · CAD-24 hotkey global: daemon standalone vs fold no GUI · raised 2026-06-27 · resolved 2026-06-27 · context: CAD-24 Layer B
**Por que eu perguntei:** contrato externo "o que é o OmniNote rodando" (tray/autostart) + `global-hotkey 0.8` NÃO roda headless (macOS exige event-loop na main thread; Linux X11-only; macOS pede Input-Monitoring).
**Decisão (Fausto):** (b1) — fold o hotkey no event-loop do GUI existente (eframe/winit) com minimize-to-tray, em vez de daemon standalone. Reusa o winit já no Cargo.lock.
**A aplicar:** Layer B fica spike-gated (provar registro do hotkey sob o winit do GUI + permissão macOS) APÓS Layer A (CLI) mergear. Resolver a colisão com `Cmd/Ctrl+Shift+Space` in-app no design do Layer B.

### Q-11 · Views tipadas (Slice 5) podem mutar os sacred files pela UI? · raised 2026-06-27 · resolved 2026-06-27 · context: CAD-25b Slice 5
**Por que eu perguntei:** mutar SPRINT/DIARY/HUMAN via UI toca dados que o protocolo discipline trata como append-only/humano (§0 #6, rule #7).
**Decisão (Fausto):** READ-ONLY em v1.2. As views só renderizam; única exceção é o DIARY "+ append entry" (reusa `discipline::diary_quick`). Edição tipada (drag-reorder, status toggle, mutação) fica para v1.3 "se houver demanda".

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