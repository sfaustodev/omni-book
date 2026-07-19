---
name: OmniNote GUI
description: Caderno Markdown nativo, keyboard-first e compatível com Obsidian.
colors:
  terminal-void: "#010604"
  terminal-panel: "#04120b"
  terminal-text: "#3ce06f"
  terminal-accent: "#6bff9a"
  paper-canvas: "#efe7d3"
  paper-ink: "#2a2620"
  terracotta-accent: "#bf4d26"
  blueprint-canvas: "#0e1a2b"
  blueprint-accent: "#4fc3f7"
  high-contrast-canvas: "#000000"
  high-contrast-ink: "#ffffff"
  high-contrast-accent: "#00ff00"
typography:
  display:
    fontFamily: "Space Grotesk, sans-serif"
    fontSize: "26px"
    fontWeight: 400
    lineHeight: 1.2
  body:
    fontFamily: "JetBrains Mono, monospace"
    fontSize: "13.5px"
    fontWeight: 400
    lineHeight: 1.4
  label:
    fontFamily: "JetBrains Mono, monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.2
rounded:
  none: "0px"
spacing:
  xs: "4px"
  sm: "6px"
  md: "8px"
  control-min: "28px"
components:
  icon-button:
    backgroundColor: "{colors.terminal-panel}"
    textColor: "{colors.terminal-text}"
    rounded: "{rounded.none}"
    padding: "4px"
    size: "28px"
  segmented-active:
    backgroundColor: "{colors.terminal-accent}"
    textColor: "{colors.terminal-void}"
    rounded: "{rounded.none}"
    padding: "6px 8px"
---

# Design System: OmniNote GUI

## 1. Overview

**Creative North Star: "Terminal / Mechanical"**

OmniNote combina a precisão de uma ferramenta de terminal com a legibilidade de um caderno. A interface é plana, rápida e mono-forward; o estado aparece por contraste, tinta, prompt e ritmo, nunca por decoração gratuita. Os nove temas preservam a mesma gramática mesmo quando a atmosfera muda de fósforo verde para papel editorial, blueprint ou alto contraste.

O design serve ao trabalho com arquivos Markdown. Rejeita dashboard genérico de SaaS ou “AI workspace”, glassmorphism, cartões arredondados em excesso, chrome de Windows 98 e ícones-enigma sem contexto.

**Key Characteristics:**

- Controles quadrados, compactos e com feedback imediato.
- Hierarquia por tipografia, espaçamento e tinta sem elevação cosmética.
- Modos e ações destrutivas sempre explícitos em linguagem humana.
- Keyboard-first sem sacrificar mouse, leitor de tela ou alvo de toque.

## 2. Colors

A paleta troca de atmosfera entre presets, mas mantém funções estáveis: canvas, painel, tinta, tinta secundária, borda e accent.

### Primary

- **Terminal Phosphor** (`#6bff9a`): foco, seleção e ação ativa no preset principal.
- **Editorial Terracotta** (`#bf4d26`): accent compartilhado pelos dois Almanac.
- **Drafting Cyan** (`#4fc3f7`): accent do Blueprint escuro.
- **Signal Green** (`#00ff00`): único accent do High Contrast.

### Neutral

- **Terminal Void** (`#010604`): canvas principal escuro.
- **Paper Canvas** (`#efe7d3`) e **Paper Ink** (`#2a2620`): extremo claro Almanac.
- **Absolute Black** (`#000000`) e **Absolute White** (`#ffffff`): base do High Contrast.

**The Role-Stability Rule.** Trocar tema muda os valores, nunca o significado: accent continua foco/ativo; text continua conteúdo; dim continua secundário.

**The Contrast-Pair Rule.** Texto sobre accent usa `accent_ink` do próprio tema; nunca reutiliza a tinta de fundo por conveniência.

## 3. Typography

**Display Font:** Space Grotesk (sans-serif)
**Body Font:** JetBrains Mono, com alternativas configuráveis e OpenDyslexic
**Label/Mono Font:** JetBrains Mono (monospace)

**Character:** títulos geométricos organizam a página; corpo monoespaçado preserva o vínculo com Markdown e terminal. A escala inteira acompanha a preferência de acessibilidade do vault.

### Hierarchy

- **Display** (400, 26 px, 1.2): título principal da nota ou painel.
- **Body** (400, 13.5 px, line-height configurável): leitura e edição.
- **Code** (400, 12.5 px): Markdown e conteúdo técnico.
- **Label** (400, 12 px): botões, campos e comandos.
- **Small** (400, 11 px): metadados e hints, nunca a única pista de uma ação.

**The Accessibility Scale Rule.** Tamanhos relativos acompanham `font_size`; troca de tema não pode apagar `line_height`.

## 4. Elevation

Não há sombras. Profundidade vem de canvas/panel/panel-alt, hairlines seletivas e sobreposição espacial. Hover usa wash leve; active usa accent sólido; High Contrast substitui washes fracos por strokes visíveis.

**The Flat-By-Default Rule.** Superfícies ficam planas em repouso; estado interativo, não decoração, cria a separação temporária.

## 5. Components

### Buttons

- **Shape:** quadrada, sem arredondamento (`0 px`).
- **Icon-only:** alvo mínimo `28 × 28 px`, tooltip obrigatório e nome acessível humano.
- **Hover / Focus:** wash de accent e tinta mais forte; High Contrast recebe outline visível.
- **Active:** fill de accent com `accent_ink`; toggles anunciam estado selecionado.

### Chips

- **Style:** texto mono compacto, sem cápsula arredondada.
- **State:** accent ou prompt indica seleção; cor nunca é a única informação.

### Cards / Containers

- **Corner Style:** reto (`0 px`).
- **Background:** apenas canvas, panel e panel-alt do tema ativo.
- **Shadow Strategy:** nenhuma sombra.
- **Border:** hairline somente onde separa regiões ou reforça High Contrast.
- **Internal Padding:** passos de `4`, `6` e `8 px`.

### Inputs / Fields

- **Style:** fundo de painel, canto reto e texto do tema.
- **Focus:** cursor/accent e stroke visível no High Contrast.
- **Error / Disabled:** status semântico com rótulo; disabled continua legível.

### Navigation

Linhas de navegação usam prompt `>`, barra lateral de `2 px` no selecionado e nomes legíveis. Modos binários usam controle segmentado com as duas opções visíveis, não um glifo que muda de significado.

### Native Menu

Itens nativos espelham o registro de comandos do editor. Ícone inválido degrada para item sem ícone; desenho opcional nunca é requisito para executar a ação.

## 6. Do's and Don'ts

### Do:

- **Do** manter alvos icon-only com pelo menos `28 × 28 px`, tooltip e nome acessível.
- **Do** usar `accent_ink` sobre fills de accent nos nove temas.
- **Do** mostrar as duas metades de um modo binário como `Ler | Editar`.
- **Do** preservar fonte e line-height depois de qualquer troca de tema.
- **Do** degradar adornos inválidos para uma versão textual segura.

### Don't:

- **Don't** criar dashboard genérico de SaaS ou “AI workspace”.
- **Don't** usar glassmorphism, gradientes decorativos, cartões arredondados em excesso ou sombras cosméticas.
- **Don't** reproduzir chrome de Windows 98 com uma caixa em volta de cada elemento.
- **Don't** publicar ícones-enigma sem rótulo acessível ou tooltip.
- **Don't** usar cor como única indicação de foco, seleção ou estado ativo.
- **Don't** adicionar movimento ornamental a um fluxo keyboard-first.
