# HUMAN — Perguntas pro humano

> Apenas decisões irreversíveis, contratos externos, segurança ou conflitos com SPRINT.md.
> Tom: direto, em pt-BR, opções com tradeoffs explícitos.

---

## Open questions

### Q-01 · Renomear `.caderno/` → `.omninote/` quebra vaults antigos · raised 2026-05-01 · context: rebrand
**Why I'm asking:** mudança em path de configuração no vault — irreversível pra vaults criados com Caderno.
**Options I considered:**
- (a) Só usar `.omninote/` daqui pra frente. Vaults antigos perdem config (dark mode, etc) — mas só você usou até agora, então blast radius é zero.
- (b) Manter compat: ler `.caderno/config.json` se existir, depois migrar pra `.omninote/`. Custo: 10 linhas de código + 1 teste.
- (c) Suportar os dois lados sempre. Custo: complexidade permanente.
**My tentative pick (if I had to ship now):** (a) — você é o único usuário e nada foi commitado em vault de produção.
**Ask:** ok ir com (a) e ignorar vaults pre-rename?

### Q-02 · Atalhos: `Ctrl+...` ou `Modifiers::COMMAND` (Cmd no mac, Ctrl no resto)? · raised 2026-05-01 · context: CAD-5
**Why I'm asking:** UX em macOS. No Mac usuário espera `Cmd+N`, não `Ctrl+N`. Atual implementação usa `Ctrl` literal — funciona mas é estranho.
**Options I considered:**
- (a) Trocar tudo pra `Modifiers::COMMAND` (mapeia automaticamente). Funciona em todas plataformas.
- (b) Manter `Ctrl` literal — comporta igual em mac/linux/windows. Usuário Mac estranha.
- (c) Detectar plataforma e branch — boilerplate desnecessário.
**My tentative pick (if I had to ship now):** (a) — convenção Mac importa, é zero custo trocar.
**Ask:** trocar pra `Modifiers::COMMAND`?

### Q-03 · CAD-9 watcher: como tratar conflito quando humano edita externamente nota ativa? · raised 2026-05-01 · context: CAD-9 (próximo sprint)
**Why I'm asking:** decisão de UX impactante. Se você edita no OmniNote enquanto Obsidian/Claude editam o mesmo arquivo, o que ganha?
**Options I considered:**
- (a) Última-grava-vence (silent overwrite). Simples, perde dado se desatento.
- (b) Avisa em modal "arquivo mudou no disco. Recarregar (perde edits) ou ignorar (sobrescreve no save)?". Mais seguro.
- (c) 3-way merge automático. Complexo, exige diff lib.
**My tentative pick (if I had to ship now):** (b) — perde-edits-silenciosamente é o pior outcome.
**Ask:** confirma (b)?

### Q-04 · Ctrl+= autoformat: substituir só na linha atual ou também propagar pra resultados aninhados? · raised 2026-05-01 · context: CAD-5
**Why I'm asking:** spec atual só substitui na linha do cursor. Caso de uso real: planilhas com `=SOMA(A1:A5)` — não suportado, e provavelmente fora de escopo. Quero confirmar antes de virar feature creep.
**Options I considered:**
- (a) Manter só linha atual. Funcional pra cálculos simples (`2+3=`).
- (b) Adicionar suporte a referências de linhas (`A1`, `B2`). Vira mini-spreadsheet.
**My tentative pick (if I had to ship now):** (a) — escopo minimalista do MVP.
**Ask:** confirmar (a) e arquivar (b) como feature pra v2.0?

### Q-05 · Coverage gate em CI — fail PR ou warn-only? · raised 2026-05-13 · context: CAD-12
**Why I'm asking:** o novo job `coverage` no CI roda `cargo llvm-cov --fail-under-lines 90` nos módulos pure (vault, wikilinks, autoformat, import, pdf, types, actions). Hoje o job FALHA o PR se cobertura cair <90%. Disciplina forte mas pode bloquear PR legítimo (refactor mecânico que toca pure code sem ganho fácil de teste).
**Options I considered:**
- (a) Manter `--fail-under-lines 90` em todo PR. Disciplina forte. Atual.
- (b) Warn-only no CI: roda report mas não falha. Status quo se humano notar regressão.
- (c) Híbrido: fail-under em push pra `main`, warn-only em PR. Permite PR exploratório, main protegido.
**My tentative pick (if I had to ship now):** (a) — vamos sentir o atrito antes de afrouxar.
**Ask:** manter (a) ou trocar pra (b)/(c)?

### Q-06 · `lib.rs` split — adiar pra v0.4 ou nunca? · raised 2026-05-13 · context: CAD-12 / SPRINT §0 #10
**Why I'm asking:** §0 #10 proíbe `tests/` dir até `lib.rs` existir. CAD-12 conseguiu 96.61% sem split, mas integration tests (egui_kittest, headless render) e doc tests dependem de `lib.rs`.
**Options I considered:**
- (a) Adiar pra v0.4 (CAD-10 spike): aproveitar wikilinks parser pra estabilizar boundaries; split como side-effect.
- (b) Adiar pra v0.6 (CAD-9 watcher): notify race conditions são o caso forte pra integration test; split lá.
- (c) Nunca: projeto fica binary-only sempre. OK se nunca quisermos kittest.
**My tentative pick (if I had to ship now):** (a) — split é cirúrgico (mover `mod x;` de `main.rs` pra `lib.rs`, ajustar `[lib]`/`[bin]` em Cargo.toml) e v0.4 é o próximo trabalho de parser.
**Ask:** confirma (a) ou prefere (b)/(c)?

### Q-07 · `vault::import_attachment` aceita qualquer extensão sem allow-list · raised 2026-05-13 · context: CAD-12 / vault.rs
**Why I'm asking:** `import_attachment(src)` chama `fs::copy` sem checar extensão (`.exe`, `.dmg`, `.sh` aceitos). Hoje OK porque o único callsite é a rfd FileDialog (humano clica → já validou). Mas se outro callsite chamar (MCP server, drag-drop), defense-in-depth diz pra ter allow-list.
**Options I considered:**
- (a) Não fazer nada. Confiar no rfd. Documentado em comentário + teste documenta gap.
- (b) Allow-list configurável em `AppConfig.attachment_extensions` com default `["png","jpg","jpeg","gif","webp","bmp","pdf","mp4","mov","mp3","txt","md","json"]`.
- (c) Block-list mínima: rejeitar executáveis (`.exe`, `.dll`, `.so`, `.dylib`, `.bat`, `.cmd`, `.ps1`, `.sh`, `.app`, `.dmg`).
**My tentative pick (if I had to ship now):** (a) — escopo atual da app é offline + single-user, threat model não justifica complexidade.
**Ask:** ok (a)? Se sim, fechar pergunta após 2026-06-01 sem resposta.

### Q-08 · `Vault::open` quando root é arquivo (não diretório) — corrigir ou aceitar? · raised 2026-05-13 · context: CAD-12 / vault.rs:13-32
**Why I'm asking:** se `Vault::open` recebe um path que aponta pra arquivo regular (não pasta), a função retorna `Ok` com vault vazio porque os `fs::create_dir_all` internos usam `let _ =` e ignoram erro. Comportamento confuso — usuário acha que abriu vault, na verdade não abriu nada.
**Options I considered:**
- (a) Manter comportamento atual. Documentado em teste `open_with_file_root_does_not_panic`. Argumento: rfd file picker não permite escolher arquivo como pasta.
- (b) Adicionar `if root.is_file() { return Err("path is a file, expected dir".into()); }` no início. 1 linha + 1 teste, zero risco.
- (c) Validar que após criar `.omninote/`, ela existe — propagar erro se não. Cobre mais casos (permissão negada, etc).
**My tentative pick (if I had to ship now):** (b) — barato + previne pegada estranha.
**Ask:** ok aplicar (b) num PR follow-up dedicado, ou empurra pro CAD-12?

---

## Resolved
