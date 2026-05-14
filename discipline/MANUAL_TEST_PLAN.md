# MANUAL TEST PLAN — OmniNote

> Checklist humano pra superfícies que não podem ser automatizadas (rfd FileDialog nativo, eframe window, watcher real, OpenDyslexic visual). Rodar antes de marcar release como pronta. Pareado com gate automatizado de coverage ≥90% pra módulos pure.

**Pré-requisito:** `cargo run --release` em macOS (preferencial), Linux ou Windows. Vault temp em `/tmp/omninote-smoke/` (criar à mão se preciso).

---

## A · Cold start (welcome screen)

- [ ] App abre sem vault salvo → tela de boas-vindas com `📓 OmniNote`, label "Escolha uma pasta...", botão `📂 Abrir / Criar Vault`
- [ ] Window respeita 1200×800 inicial, mín 600×400 (resize não quebra layout)
- [ ] Panic hook em `main.rs` — provocar panic via `--panic-test` (se houver flag) ou esperar erro real; assert mensagem no stderr legível, não silenciosa

## B · Vault picker (rfd FileDialog)

- [ ] Click `📂 Abrir / Criar Vault` → abre dialog nativo macOS Finder / GTK Linux / Win32
- [ ] Cancelar dialog → app não crash, fica em welcome
- [ ] Selecionar pasta vazia → vira vault novo (`.omninote/` + `_attachments/` criados)
- [ ] Selecionar pasta com `.md` files (Obsidian vault) → carrega notes existentes
- [ ] Selecionar pasta com `.caderno/` legacy → migra silenciosamente pra `.omninote/`
- [ ] Selecionar pasta sem permissão de write → mensagem de erro decente

## C · Sidebar

- [ ] `🔍 busca` campo aceita texto, filtra notes em real-time (case insensitive)
- [ ] `Cmd+K` (mac) / `Ctrl+K` (linux/win) foca busca
- [ ] Type chips (Resumo / Citação / Código / Exercício / Dúvida / Definição) togglable, filtram lista
- [ ] Folder tree expand/collapse OK
- [ ] Hover folder → ✎ Edit + 🗑 Delete buttons aparecem
- [ ] Click folder → footer mostra `📄+ Nova nota aqui` + `🗑 Deletar pasta`
- [ ] `+ NOTE` no rodapé abre modal nova nota
- [ ] Botões `⚙` settings, `📂` vault, `📥` import, `📁` new folder funcionam

## D · Editor

- [ ] Click note na sidebar → abre no editor (modo view por default)
- [ ] Toggle Read/Edit (`Cmd+E` ou botão) alterna mode
- [ ] Edit mode: title TextEdit aceita rename, type ComboBox lista 6 tipos com ícones, tags adicionar/remover
- [ ] Content TextEdit: typing OK, scroll OK, copy/paste OK
- [ ] `Cmd+=` em linha terminando com `=` avalia aritmética: `2 + 3 =` vira `2 + 3 = 5`. Vírgula brasileira `1,5 + 1,5 =` vira `3`. `÷` `×` operadores unicode funcionam
- [ ] View mode: CommonMarkViewer renderiza markdown (headings, bold, italic, lists, blockquote, fenced code com syntax highlight, tables, links externos)
- [ ] `📎 Anexar arquivo` botão → abre rfd dialog → escolhe imagem → wikilink `![[name.png]]` inserido no cursor
- [ ] Backlinks section mostra notes que linkam pra atual; click navega
- [ ] `🗑` delete note → confirma modal antes

## E · Modais

- [ ] **Nova nota** (Cmd+N): grid 2x3 com 6 tipos, click cria nota e abre no editor
- [ ] **Settings** (Cmd+,): font family radio (System / Monospace / Serif / OpenDyslexic), font size slider, line height slider, letter spacing, dark mode toggle, "↩ Restaurar padrões" reset, "📂 Trocar vault" button
- [ ] **OpenDyslexic** seleção troca fonte visualmente em todo app (verificar peso característico das letras)
- [ ] **Confirm delete** com botão vermelho "🗑 Sim, deletar" + "Cancelar"
- [ ] **Import** abre 3 botões: `📄 PDF`, `🤖 Chat Claude`, `📦 Artefato`. Cada um abre rfd dialog específico
- [ ] **External conflict** (modificar arquivo .md externamente enquanto editing): modal "Recarregar" / "Manter edits"
- [ ] Modal close X / clicar fora fecha

## F · Atalhos globais (Cmd no macOS, Ctrl no resto)

- [ ] Cmd+N — nova nota
- [ ] Cmd+E — toggle edit mode (só com active note)
- [ ] Cmd+K — foca busca
- [ ] Cmd+, — abre settings
- [ ] Cmd+= — autoformat math na linha atual
- [ ] Cmd+Shift+D — dark mode toggle (no-op em v1.0 swiss-only, deve apenas re-aplicar tema)

## G · Auto-save (CAD-4)

- [ ] Edit content → após 600ms idle, save dispara silenciosamente
- [ ] Indicator visual `● UNSAVED` → `● SAVED` toggle
- [ ] Crash during edit → último save de até 600ms atrás persiste (perda máx 600ms de typing)

## H · Filesystem watcher (CAD-9)

- [ ] Modificar arquivo `.md` no vault via editor externo (ex: `echo "x" >> note.md` em outra terminal) → app detecta dentro de ~1s
- [ ] Se note sendo editada (dirty) → modal external conflict
- [ ] Se note não sendo editada → reload silencioso, conteúdo atualiza
- [ ] Save próprio do app NÃO dispara reload-loop (self_write_until 400ms window)
- [ ] Deletar arquivo `.md` externamente → note some da sidebar
- [ ] Renomear arquivo `.md` externamente → app detecta como delete + create

## I · Imports

- [ ] **PDF**: importar `manual.pdf` (multi-page) → vira note nova com `## Página N` headings + texto extraído
- [ ] **Chat Claude JSON**: importar export oficial `.json` → vira note com `**Você:**` / `**Claude:**` blocks
- [ ] **Artefato**: importar `.tsx` → wrap em fence ` ```tsx ... ``` `; `.md` passa direto; extension desconhecida → fence com nome da extension

## J · Casos negativos

- [ ] Vault picker cancel → no crash
- [ ] Import PDF corrompido → mensagem de erro decente, app continua
- [ ] Import JSON inválido → mensagem de erro
- [ ] Vault em path com spaces / acentos / emoji → funciona
- [ ] Note com 100K caracteres → editor não trava
- [ ] 1000 notes na sidebar → render OK (scroll, busca)

## K · Cross-platform (futuro v1.0)

- [ ] macOS: bundle `.app` via `cargo bundle --release` instala drag-drop
- [ ] Linux: `cargo build --release` produz binário ~10MB stripped, roda em GTK
- [ ] Windows: `cargo build --release` cross-compile (não testado v0.x)

---

## Confirmação

Após todo checklist verde, registrar no chat:

> "Smoke macOS feito, todos OK. Pode fechar CAD-12."

Sem essa string explícita, **CAD-12 fica em status `👀 Revisão` no Notion** (per discipline rule #13).

Bugs encontrados → commits adicionais na branch `feat/cad-12-test-coverage`, **não SCRUM novo** (per discipline rule #13).
