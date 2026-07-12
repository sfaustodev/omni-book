# QA de formatação da OmniNote GUI

Data: 2026-07-11  
Branch: `fix/cad-25-gui-polish`

## Cobertura automatizada

O teste `formatting_gauntlet_covers_every_action_entrypoint_fixture_cell` percorre
as 640 células (`16 ações × 5 entrypoints × 8 fixtures`). Em cada célula
suportada ele chama o helper puro `apply_editor_action`, captura panic, valida o
range UTF-8 resultante e executa undo + redo. Células não suportadas validam
mecanicamente que a ação não aparece naquele entrypoint.

Um resultado `None` é um no-op seguro, esperado para Matemática sem expressão
completa e para Bloco de código dentro de uma fence já aberta. A semântica
canônica exata das 16 ações é verificada separadamente por
`formatting_semantics_are_exact_for_every_action`.

São 528 células suportadas e 112 N/A: Matemática × Slash (8) mais as 13 ações
sem atalho de teclado × 8 fixtures (104).

Legenda: `A` = comportamento automatizado; `A/N/A` = ausência intencional
automatizada.

| Ação | Menu Editar | Slash `/` | Palette | UI/contexto | Teclado |
|---|---:|---:|---:|---:|---:|
| Negrito | A | A | A | A | A (`Mod+B`) |
| Itálico | A | A | A | A | A (`Mod+I`) |
| Tachado | A | A | A | A | A/N/A |
| Código inline | A | A | A | A | A/N/A |
| Bloco de código | A | A | A | A | A/N/A |
| H1 | A | A | A | A | A/N/A |
| H2 | A | A | A | A | A/N/A |
| H3 | A | A | A | A | A/N/A |
| Lista | A | A | A | A | A/N/A |
| Lista numerada | A | A | A | A | A/N/A |
| Tarefa | A | A | A | A | A/N/A |
| Citação | A | A | A | A | A/N/A |
| Link | A | A | A | A | A/N/A |
| Wikilink | A | A | A | A | A/N/A |
| Divisor | A | A | A | A | A/N/A |
| Matemática | A | A/N/A | A | A | A (`Mod+=`) |

Fixtures exercitadas em todas as células:

1. nota vazia;
2. cursor em zero;
3. cursor em EOF;
4. seleção vazia;
5. seleção multilinha;
6. texto multibyte com emoji e acentos, incluindo índice no meio de codepoint;
7. seleção dentro de bloco de código existente;
8. expressão matemática válida em EOF.

Testes complementares cobrem semântica exata das 16 ações, alvo slash stale ou
no meio de codepoint, bloco de código dentro de fence como no-op, seleção após
operações consecutivas, caret após undo do slash, descarte do ramo antigo de
redo, escopo do estado do editor por nota e roteamento das cinco superfícies
pelo mesmo registro.

Resultado automatizado:

- `timeout 300 cargo test -p omninote-gui </dev/null`: **PASS — 138/138**.
- `timeout 300 cargo test -p omninote-gui formatting_gauntlet_covers_every_action_entrypoint_fixture_cell </dev/null`: **PASS — 1/1**.
- Matriz: **PASS — 640/640 células cobertas**.
- Panic nas células: **0**.

## Checklist manual macOS

O checklist abaixo cobre o que o teste puro não observa: renderização, presença
no menu nativo, foco real e ligação do clique à janela AppKit.

### Menu nativo Editar

- [ ] Com `RUST_BACKTRACE=1`, clicar `Editar → Bloco de código` em nota vazia:
  item executa e o processo continua vivo.
- [ ] Repetir bloco de código com seleção multilinha e texto `🙂 café ação`.
- [ ] Confirmar que as 16 ações da tabela aparecem e que uma ação de cada grupo
  (wrapper, prefixo de linha, inserção e matemática) altera a nota correta.
- [ ] Executar undo e redo após cada ação amostrada.

### Outras superfícies

- [ ] Slash menu mostra 15 ações, omite Matemática e remove o `/` ao executar.
- [ ] Command palette mostra e executa as 16 ações.
- [ ] Menu de contexto do editor mostra e executa as 16 ações.
- [ ] `Mod+B`, `Mod+I` e `Mod+=` funcionam; AltGr não dispara ação.
- [ ] Trocar de nota com menu/palette aberto não aplica comando na nota anterior.

### Affordance e temas

- [ ] `[ Ler | Editar ]` é legível, tem estado ativo inequívoco e tooltip com
  `Mod+E` no Almanac Light.
- [ ] Repetir no High Contrast, incluindo hover, foco por teclado e clique.
- [ ] Verificar tooltips e alvo dos botões icon-only na titlebar, sidebar, abas,
  breadcrumb, calendário e views de discipline.
- [ ] Usar nome de vault longo: as ações da titlebar continuam visíveis.
- [ ] Em discipline, selecionar Raw, ir a Editar e voltar a Ler: Raw permanece.

### Execução

- Descoberta AX do menu nativo: **PASS** — `Editar` expôs as 16 ações e `Tema`
  expôs os nove presets.
- Clique real e inspeção visual: **PENDENTE** — a autorização externa do runner
  para controlar/reabrir a GUI esgotou a cota durante esta sessão. Nenhum PASS
  manual é inferido do teste automatizado.
