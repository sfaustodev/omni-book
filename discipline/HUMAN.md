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

---

## 2026-05-20 — Próximo passo da sessão (4 opções abertas)

**Context:** Sessão fechou 3 PRs stacked (PR #4 chore-discipline, PR #5 CAD-20 link parity + fence fix, PR #6 CAD-21 Phase A workspace). Tudo testado local, sem CI nos stacked (workflow só dispara em PRs pra main). Aguardando próxima direção sua.

**Options I considered:**

1. **Wait for human review** — você testa CAD-20 manualmente (abrir vault Obsidian, navegar wikilinks com aliases/anchors/paths), responde Q-01..Q-30 do `docs/UI_DESIGN_v2.md`, aprova PRs em ordem #4 → #5 → #6. Sem trabalho novo do agente até teu OK.
2. **Continue building** — agente segue paralelo: CAD-21 Phase B (CLI verbs `note search` + `link backlinks`) ou Phase C (MCP rmcp wiring) em nova branch stacked off PR #6. Acumula mais PRs pendentes review.
3. **Fix CI workflow** — pequeno PR que muda `pull_request: branches: [main]` pra padrão que pega stacked (`branches: ['**']` ou `feat/**`/`chore/**`). Desbloqueia CI verde nos PRs #5 e #6 sem teu input.
4. **Outro** — algo fora desse menu.

**My tentative pick (if I had to ship now):** opção **3 + 2 em paralelo** — fix CI primeiro (curto, ajuda review depois) + continuar Phase B em background. Mas a melhor opção depende de quanto tempo você tem pra revisar e se quer ver CI verde antes de aprovar.

**Ask:** qual das 4 atacar próximo?

---

## Resolved
