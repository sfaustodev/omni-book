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

## Resolved
