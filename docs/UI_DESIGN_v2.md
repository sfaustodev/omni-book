# UI_DESIGN_v2 — OmniNote Obsidian-class three-pane port (egui 0.29)

> **Phase:** CAD-25 Fase A — analysis + port plan. No Rust code touched in this phase. Fase B will execute this plan.
> **Design source:** `docs/design/omninote/project/styles/07-omninote-obsidian.jsx` (primary), `06-swiss.jsx` / `03-aurora.jsx` / `05-cyberpunk.jsx` / `01-brutalist.jsx` / `02-neumorphic.jsx` / `04-editorial.jsx` (reusable patterns).
> **Target stack:** egui 0.29 + eframe, single-binary desktop. egui_commonmark 0.18 for markdown render. pulldown-cmark planned for the inline wikilink renderer (per SPEC.md §6).
> **Existing surface to extend:** `src/ui_sidebar.rs` (280 px left), `src/ui_editor.rs` (CentralPanel), `src/ui_modals.rs` (4 windows). State lives entirely on `OmniNoteApp` struct (`src/app.rs`).
> **Hard constraints:** see CAD-25 spec + SPRINT v1.1 §0; vault-on-disk is the source of truth; everything keyboard-first; accessibility presets honored.

---

## 0. Table of contents

1. Design token map (color, spacing, typography, motion)
2. Layout sketches — 15 entry points
3. Layout sketches — 17 artifacts
4. State map — UI state transitions and triggers
5. Egui code structure proposal — file split + method signatures
6. Keyboard shortcut table (consolidated)
7. CLI output style guide
8. MCP tool registry
9. Open questions for Fausto

---

## 1. Design token map

### 1.1 Color tokens (dark default)

Source: `07-omninote-obsidian.jsx` lines 5–34. Tokens map directly to RGB tuples consumed via `egui::Color32::from_rgb(r,g,b)`. We propose a `theme.rs` module owning a `Theme` struct so every `ui_*.rs` file pulls the same palette.

| Token | Hex | Role | egui usage |
|---|---|---|---|
| `bg` | `#1b1d22` | Editor canvas | `Visuals.window_fill`, `panel_fill` for CentralPanel |
| `chrome` | `#171a1e` | Title bar, status bar | Top/bottom `TopBottomPanel.frame.fill` |
| `panel` | `#212429` | Sidebar, right rail | `SidePanel.frame.fill` |
| `panel2` | `#272a30` | Raised: tab strip, hovered rows | `widgets.hovered.bg_fill` |
| `inset` | `#15171b` | Search field, code blocks | `extreme_bg_color`, code block frame |
| `border` | `#2d3037` | Default 1 px stroke | `widgets.noninteractive.bg_stroke` |
| `border_strong` | `#373b44` | Modal borders, popups | `widgets.active.bg_stroke` for windows |
| `divider` | `#23262c` | Inter-section separators inside panels | `ui.separator()` color override |
| `text` | `#dadde2` | Body text | `text_color` for `Body` style |
| `text_strong` | `#f0f2f5` | Headings, active labels | `Heading` style + active `selectable_label` |
| `dim` | `#8b8f98` | Secondary labels, icons inactive | `weak_text_color`, dim metadata |
| `dimmer` | `#5e6470` | Tertiary, placeholder | `placeholder_text` for `TextEdit.hint_text` |
| `accent` | `#8b7cff` | Brand violet, active state | NEW `AppConfig.accent_color`, default `#8b7cff` |
| `accent_dim` | `#5b4fc4` | Gradient pair | derived: `accent.linear_multiply(0.65)` |
| `accent_bg` | `rgba(139,124,255,0.12)` | Active row fill | `Color32::from_rgba_unmultiplied(139,124,255,30)` |
| `accent_bg_strong` | `rgba(139,124,255,0.20)` | Pressed, focused | `Color32::from_rgba_unmultiplied(139,124,255,51)` |
| `green` | `#5dcc8f` | NoteType `codigo`, "saved", done | semantic |
| `amber` | `#e5b14a` | NoteType `exercicio`, "in progress" | semantic |
| `red` | `#e5715a` | NoteType `duvida`, REC indicator, broken link | semantic |
| `blue` | `#5fb2f0` | NoteType `resumo`, JIRA glyph | semantic |
| `pink` | `#e07cc4` | NoteType `citacao`, HUMAN glyph | semantic |
| `violet` (=accent) | `#8b7cff` | NoteType `definicao`, PLAN glyph | semantic |

### 1.2 NoteType ↔ color + glyph

Existing `NoteType` (in `src/types.rs`) currently emits emoji icons (`📄 💬 💻 ✏ ❓ 💡`). The Obsidian-class mockup replaces those with monospace glyphs paired to semantic color so they sit cleanly inside text runs.

| NoteType | Glyph (mono) | Color token | Current emoji to retire |
|---|---|---|---|
| `Resumo` | `▤` | `blue` | `📄` |
| `Citacao` | `"` | `pink` | `💬` |
| `Codigo` | `⌘` | `green` | `💻` |
| `Exercicio` | `◇` | `amber` | `✏` |
| `Duvida` | `?` | `red` | `❓` |
| `Definicao` | `§` | `accent` (violet) | `💡` |

**Migration:** add `glyph()` returning `&'static str` and `color(theme: &Theme) -> Color32` to `impl NoteType`. Keep `icon()` for backward compat (CLI fallback when terminal has no monospace UI font). Update `selectable_label` callers in `ui_sidebar.rs` chips and folder tree.

### 1.3 Discipline file glyphs + colors

Discipline files become first-class sidebar entries with bespoke glyphs (source `07-omninote-obsidian.jsx` lines 183–188):

| File | Glyph | Color | Typed view? |
|---|---|---|---|
| `SPRINT.md` | `◈` | `green` | Yes — task list (see §3.4) |
| `DIARY.md` | `✎` | `amber` | Yes — entries collapsed by date (§3.5) |
| `JIRA.md` | `◧` | `blue` | Yes — ticket index merged with NOTION (§3.6) |
| `NOTION.md` | `◨` | `dim` | Yes — same merged view |
| `HUMAN.md` | `☻` | `pink` | Yes — Q&A pairs (§3.7) |
| `PLAN.md` | `≡` | `accent` | Yes — checklist (§3.8) |
| `ETERNAL.md` | `∞` | `dim` | Yes — cross-sprint projects (§3.9, optional) |
| `Inbox.md` | `▼` | `accent` | Lightweight typed view — quick-capture sink |
| `Daily/YYYY-MM-DD.md` | `◉` (today) `○` (other) | `accent`/`text` | Daily view (§3.3) |

### 1.4 Spacing scale

Direct extraction from the mockup. We adopt 4-pt rhythm.

| Token | px | Use |
|---|---|---|
| `s0` | 2 | Inline icon padding |
| `s1` | 4 | Chip spacing, badge padding |
| `s2` | 6 | Default row gap |
| `s3` | 8 | Section gap, button padding-x |
| `s4` | 10 | Sidebar header padding, vault switcher |
| `s5` | 12 | Sidebar item padding, modal padding |
| `s6` | 14 | File-row inner padding |
| `s7` | 18 | Command palette inner padding |
| `s8` | 24 | Title H1 bottom margin |
| `s9` | 32 | Editor body padding (small viewport) |
| `s10` | 36 | Editor body padding (default) |
| `s11` | 64–80 | Editor body horizontal padding (wide reading column) |

### 1.5 Typography

| Style | Family | Size | Weight | Use |
|---|---|---|---|---|
| `Heading` (egui `Heading`) | sans `Inter` | 21 | 600 | H2 in note body |
| `H1` (custom) | sans `Inter` | 32 | 700 | Note title in read view |
| `Body` (egui `Body`) | sans `Inter` | 14.5 | 400 | Paragraph |
| `BodyStrong` (custom) | sans `Inter` | 14.5 | 600 | Embed card title |
| `Small` (egui `Small`) | sans `Inter` | 11–12 | 400 | Sidebar meta, status bar |
| `Monospace` (egui `Monospace`) | mono `JetBrains Mono` | 12.5 | 400 | Code inline, frontmatter, IDs |
| `Kbd` (custom) | mono `JetBrains Mono` | 10 | 500 | Keybinding hints |
| `Glyph` (custom) | mono `JetBrains Mono` | 11–14 | 400 | NoteType glyphs |
| `Label` (custom) | sans `Inter` | 10 | 600 uppercase, letter-spacing 1.5 | Section headers (`TODAY`, `INBOX`, …) |

`AppConfig.font_size` already scales the whole tree (existing code in `OmniNoteApp::apply_style`). The Obsidian-class layout assumes 14 px base — keep that default. Sliders 11–24 pt continue to scale via the `scale = base / 14.0` multiplier.

**Font registration plan:** in `omninote-gui/src/main.rs`, register `Inter` (3 weights: 400/500/600/700) and `JetBrains Mono` (400/500) as embedded fonts via `egui::FontDefinitions`, mapped to families `proportional` and `monospace`. Fallback to system fonts when these are not embedded (initial v1.2 ship — embed only `Inter-Variable` + `JetBrainsMono-Regular` to keep binary < 12 MB).

### 1.6 Motion + animation

egui is immediate-mode so all "animation" is per-frame interpolation against an `Instant`. Targets:

| Effect | Where | Implementation hint |
|---|---|---|
| Recording-dot pulse | Title-bar REC pill | `ui.ctx().request_repaint_after(Duration::from_millis(50))`, modulate alpha `0.4 + 0.6 * (t.sin() * 0.5 + 0.5)` |
| Quick-capture popup fade-in | Quick capture | 120 ms linear ease, opacity 0→1; egui's `Window` ships no built-in transition — implement via `ui.set_opacity()` if exposed in 0.29, else accept hard cut |
| Toast slide-in | Bottom-right | Slide 24 px up + fade-in over 150 ms |
| Smooth scroll on wikilink-jump | Editor | Use `ScrollArea::scroll_to_rect` (no animation lib needed) |
| Spinner for AI Chat in-flight | Right rail | `egui::Spinner::new()` |

Animations are nice-to-have. Mark as P2 in Fase B; ship hard cuts if time-constrained.

### 1.7 New `AppConfig` fields required

Append to `src/types.rs::AppConfig` (Fase B):

```rust
// Right rail (added v1.2)
pub right_rail_open: bool,                     // default true
pub right_rail_tab: RightRailTab,              // Backlinks | Outline | AiChat

// Daily / templates
pub daily_auto_open: bool,                     // opt-in (rule from CAD-22)
pub default_template_for_daily: Option<String>, // "daily.md" or None

// AI chat
pub llm_provider: LlmProvider,                 // Claude | Grok | Ollama | Disabled
pub llm_model_id: String,                      // "claude-sonnet-4.5" etc

// Accent + theme presets
pub accent_color: [u8; 3],                     // default [139, 124, 255]
pub theme_preset: ThemePreset,                 // ObsidianDark | ObsidianLight | HighContrast | Custom

// Quick capture
pub quick_capture_hotkey: String,              // "Ctrl+Shift+Space" — config string

// Dictation
pub dictation_hotkey: String,                  // "Ctrl+Shift+M"
pub dictation_locale: String,                  // "pt-BR"

// Command palette
pub palette_hotkey: String,                    // "Ctrl+P"
```

Plus new enums `RightRailTab`, `LlmProvider`, `ThemePreset`. All `#[serde(default)]` for backwards-compat with v1.0 `config.json`.

### 1.8 High-contrast preset

For accessibility (CAD-25 constraint #4):

| Token | Standard | High contrast |
|---|---|---|
| `bg` | `#1b1d22` | `#000000` |
| `panel` | `#212429` | `#0a0a0a` |
| `text` | `#dadde2` | `#ffffff` |
| `dim` | `#8b8f98` | `#cccccc` |
| `border` | `#2d3037` | `#666666` |
| `accent` | `#8b7cff` | `#ffff00` |
| `accent_bg` | `rgba(139,124,255,0.12)` | `rgba(255,255,0,0.30)` |

Applied via `Theme::high_contrast()` constructor. Same struct shape — every `ui_*.rs` call already pulls `theme.bg` etc., so the high-contrast switch is a single `Theme` re-bind in `OmniNoteApp::apply_style`.

A light preset (`ObsidianLight`) also derives from the same shape — invert luminance, keep accent. Stub it in v1.2; final light tuning can wait until a user complaint.

---

## 2. Layout sketches — 15 entry points

Each entry point gets: visual ASCII mock + behavioral notes + the `OmniNoteApp` state fields it reads/writes + the existing-or-new `show_*` method that renders it.

### 2.1 GUI cold-start (no last_vault)

Existing code already handles this in `app.rs::update()` lines 331–349. Refine with onboarding tips.

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│                                                                  │
│                          📓 OmniNote                             │
│                  Native Rust notebook                            │
│                                                                  │
│             ┌────────────────────────────────────────┐           │
│             │   📂  Open or create vault              │           │
│             └────────────────────────────────────────┘           │
│                                                                  │
│            Compatible with Obsidian and Claude Desktop           │
│                                                                  │
│             ─── tips ───                                         │
│             ⌘N  — New note                                       │
│             ⌘P  — Command palette                                │
│             ⌘K  — Search                                         │
│             ⌘⇧Space — Quick capture                              │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

- State: `vault: None`, `error_msg: None`.
- Method: `OmniNoteApp::show_cold_start(ctx)`.
- Behavior: click "Open or create vault" → `rfd::FileDialog::pick_folder` (existing `pick_vault`). On success, transitions to **2.2 warm-start path** in the next frame.

### 2.2 GUI warm-start

`vault` is `Some`. Existing `apply_style` runs. New: restore right-rail open/tab from `AppConfig`, restore last active note from `AppConfig.last_active` (already a field in v1.0 but currently unused — wire it in Fase B), restore scroll position (best-effort: persist on `on_exit` only).

Sequence on `new()`:
1. Load last_vault path from `~/.config/omninote/last_vault`.
2. `Vault::open()`.
3. `apply_style(ctx)`.
4. If `daily_auto_open` and no daily note for today → create from `Templates/daily.md`, set `active_note`.
5. Else if `last_active` is Some → select that note id.

No new UI surface — just startup logic.

### 2.3 GUI sidebar (left 280 px) — extended

Source: `07-omninote-obsidian.jsx` lines 132–218.

```
┌──────────────────────────────────┐
│ ⬢ ClaudeBook ▾                  ⚙│  ← vault switcher row (10 px pad)
│ ~/Projects/caderno               │
├──────────────────────────────────┤
│ ⌕  Buscar no vault…       [⌘P]   │  ← search → opens command palette
├──────────────────────────────────┤
│ [▤ resumo 18] [⌘ codigo 9]       │  ← type chips (selectable)
│ [? duvida 4]  [§ definicao 11]   │
│ [" citacao 6]                    │
├──────────────────────────────────┤
│ ▾ TODAY                       📅 │  ← section header (uppercase 10 px)
│   ◉ 2026-05-20.md  ← active     │
│   ○ 2026-05-19.md                │
│   ○ 2026-05-18.md                │
│      open calendar…              │
│                                  │
│ ▾ INBOX                          │
│   ▼ Inbox.md              [3]   │  ← badge = unread/uncategorized
│                                  │
│ ▾ DISCIPLINES (6)                │
│   ◈ SPRINT.md         (green)    │
│   ✎ DIARY.md          (amber)    │
│   ◧ JIRA.md           (blue)     │
│   ☻ HUMAN.md          (pink)     │
│   ≡ PLAN.md           (accent)   │
│   ◨ NOTION.md         (dim)      │
│                                  │
│ ▾ PROJECTS (4)                   │
│   ◆ caderno                      │
│   ◆ cfo-pocket                   │
│   ◆ omni-mcp                     │
│   ◆ kappa-trade (dim — archived) │
│                                  │
│ ▾ VAULT (142)                    │
│   ▦ Templates                    │
│   ▦ _attachments (dim)           │
│   ▦ 01 — Notes                   │
│     ⌘ egui patterns.md           │
│     § rust borrow checker.md     │
│     ▤ Whisper benchmarks.md      │
│   ▦ 02 — Reading                 │
│   ▦ 03 — Refs                    │
├──────────────────────────────────┤
│ ＋ ⧉ ♯ ◷       142 notes · 3.2MB │  ← footer toolbar
└──────────────────────────────────┘
```

**Section ordering** (all collapsible, persisted in `AppConfig.sidebar_sections_open: Vec<String>`):
1. **Today** — pinned. Shows the 3 most recent dailies + "open calendar…" affordance.
2. **Inbox** — single `Inbox.md` row. Badge = count of bullet-list items currently in file (cheap regex on load).
3. **Disciplines** — fixed list of files. Only show those that exist on disk.
4. **Projects** — folders under `Projects/` directory. Each renders as a `◆` row; click expands inline (no separate view).
5. **Vault** — full tree (existing behavior). Suppress display of `.omninote/`, `Templates/`, `_attachments/`, `Daily/`, `Projects/`, `Inbox.md` and discipline files here (they're already up top).

**Footer icon meanings:**
- `＋` — new note (opens existing **NewNote modal**, Ctrl+N).
- `⧉` — toggle right rail (Ctrl+\).
- `♯` — toggle tag explorer overlay (Ctrl+Shift+T) (see §3.10).
- `◷` — toggle timeline view (Ctrl+Shift+H) (see §3.16).

**Right-aligned label** in footer: `<note_count> notes · <human_readable_size>`. Compute lazily from `vault.notes.len()` + sum of file sizes (cached every 5 s).

- State reads: `vault.notes`, `vault.config.sidebar_sections_open`, `type_filter`, `query`, `active_note.frontmatter.id`.
- State writes: `query`, `type_filter`, `active_note` (via `select_note`), `show_new`, plus `show_right_rail`, `show_tag_explorer`, `show_timeline`.
- Methods: existing `show_sidebar(ctx)` keeps the entry; add private helpers `show_today_section`, `show_inbox_section`, `show_discipline_section`, `show_projects_section`, `show_vault_section`.

### 2.4 GUI editor center panel — extended

Source: `07-omninote-obsidian.jsx` lines 258–440.

```
┌─[2026-05-20.md]─[egui patterns.md]─[SPRINT.md]─────────────[↶ ↷ ⊞]┐
│ Daily / 2026 / 2026-05-20 › Sprint review   ed 2m · 412w   👁 ✎ ⋯│
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│   ┌──────────────────────────────────────────────────────────┐    │
│   │ ── frontmatter ─────────────────────                     │    │
│   │ type: daily                                              │    │
│   │ aliases: [Today, Quarta]                                 │    │
│   │ tags: [#sprint, #daily, #cfo-pocket]                     │    │
│   │ ───                                                      │    │
│   └──────────────────────────────────────────────────────────┘    │
│                                                                    │
│   # 2026-05-20 · Quarta                                            │
│   ← [[2026-05-19]] · [[2026-05-21]] →            #daily #sprint    │
│                                                                    │
│   ## Sprint review                                                 │
│   Fechamos a integração [[iFood Linking]] no branch `feat/focus    │
│   NFe`. O fluxo do [[Motoboys module]] está em #review — falta    │
│   o passo de OWASP audit (ver [[OWASP-2021 checklist]]). Quebrei  │
│   o registro de entrega em três rotas: `POST /api/v1/...`         │
│                                                                    │
│   Conversei com a Ju sobre o [[Fee Tier Matrix]] (broken,         │
│   dashed-red underline) — ainda preciso criar essa nota. Para o   │
│   app Flutter, ver discussão em [[HUMAN#login-flow]].             │
│                                                                    │
│   ┌── ![[Motoboys module]] ─────────────────── open full → ┐      │
│   │ ▤  preview                                              │      │
│   │ Motoboys module — spec v1.0                             │      │
│   │ Entregar um fluxo operacional onde o motoboy faz login  │      │
│   │ num app Flutter dedicado, registra a entrega digitando…│      │
│   └─────────────────────────────────────────────────────────┘     │
│                                                                    │
│   ## Próximos passos                                               │
│   ☑ Fechar branch feat/focusNFe                                    │
│   ☐ OWASP audit — top 10 / 2021                                    │
│   ☐ Criar nota Fee Tier Matrix                                     │
│   ☐ Validar fluxo offline (failsafe)                               │
│                                                                    │
│   ┌──────────── ![[architecture-diagram.png]] ──────────────┐      │
│   │           [        thumbnail rendered here         ]    │      │
│   │ architecture-diagram.png · 1.2MB              open →   │      │
│   └─────────────────────────────────────────────────────────┘     │
│                                                                    │
│   Comecei a esboçar a próxima discussão /                         │
│   ┌─ AI ACTIONS ─────────────────────────────────┐                │
│   │ ✦ /summarize     Resumir esta nota       ↵   │ ← active       │
│   │ ❓ /ask           Perguntar ao vault          │                │
│   │ ✎ /tag-auto      Auto-tag via LLM             │                │
│   │ ─ TEMPLATES ─                                 │                │
│   │ ▦ /template meeting                            │                │
│   │ ▦ /template daily                              │                │
│   └────────────────────────────────────────────────┘                │
└────────────────────────────────────────────────────────────────────┘
```

**Tab strip** (32 px tall, above the breadcrumb):
- Each tab shows note glyph + truncated title (max 140 px) + dirty dot (5 px circle) + close `×`.
- Active tab gets accent top border + `bg` color matching editor canvas.
- Right-side cluster: undo `↶`, redo `↷`, toggle right rail `⊞`.
- Tabs are a NEW concept in OmniNote — current code only supports single active note. See §5.5 for the tab state plan.

**Breadcrumb row** (30 px tall):
- Path from vault root to current note: `Daily / 2026 / 2026-05-20`.
- Plus optional heading anchor when wikilink jumped to one: `› Sprint review`.
- Right cluster: edited timestamp + word count + view/edit toggle + menu.

**Body padding:** `36 px top/bottom, 64 px left, 80 px right`. Max content width 820 px (centered). Headings get a left-margin "hanging" `#`/`##` glyph in `dimmer` (positioned `left: -32 px` from heading text). In egui: render the glyph in a left-hand column of a 2-column horizontal layout, or via `ui.allocate_painter` with absolute positioning.

**Frontmatter block:** rendered as a callout with `accent` left border, `inset` background, monospace 11.5 px, 1.8 line-height. Editable inline in edit mode; read-only collapsed by default in view mode (click to expand).

**Inline wikilink rendering** (new — requires `pulldown-cmark` switch per SPEC.md §6):
- `[[Note]]` → `accent` colored, no underline; hover → underline + preview popup (see §3.2).
- `[[Note|Alias]]` → renders "Alias" with same styling, click navigates to "Note".
- `[[Note#Heading]]` → renders "Note > Heading" or just "Note#Heading" verbatim; click navigates + smooth-scrolls to heading.
- `[[Note#^block-id]]` → same as heading variant.
- **Broken** (target not found): dashed red underline, color `red`, hover tooltip "Note not found — click to create".

**Inline embed rendering** (`![[…]]`):
- `![[Note]]` → embedded card: 1 px border, `panel` bg, 12×16 pad. First line = title in `text_strong` 15 px. Next line(s) = first 200 chars of content in `dim` 13 px, 1.55 line-height. Header strip = NoteType glyph + monospace label `!![[Title]]` + "open full →" link. Click anywhere → navigate to embedded note.
- `![[Note#Heading]]` → same card, content = the section under that heading.
- `![[image.ext]]` → renders inline via `egui::Image::new(file://…)`. Card wraps image + 6×12 footer with filename + size + "open" link. Max width = `available_width.min(720)`.
- `![[file.pdf]]` → 180 px tall placeholder (until PDF first-page render is wired). Card footer same as image. "open" → `open::that(path)`.
- `![[file.other]]` → just card footer, no preview.

**Inline `#tag` chips:** body-position `#word` (not in code blocks) becomes a clickable chip — `accent` text on `accent_bg`, 6 px x-pad, 3 px radius. Click → set `query = tag` and focus sidebar search.

**Slash menu (extended):** existing v0.8 slash menu only inserts markdown blocks. Extended menu (anchored at cursor position via TextEdit cursor_range as today) splits into sections:
- **AI ACTIONS** (top, accent header): `/summarize`, `/ask`, `/translate`, `/tag-auto`, `/extract-todos`, `/explain-this`.
- **TEMPLATES**: dynamically populated from `Templates/*.md` filenames.
- **DISCIPLINE**: `/diary append`, `/human ask`, `/ticket SCRUM-XX`, `/plan note`.
- **MARKDOWN** (existing): H1/H2/H3/bold/italic/code/quote/list/todo/link/wikilink/divider.

Each row: glyph + monospace command + 1-line description + `↵` kbd hint on selected row.

**Heading TOC** (right rail Outline tab, not in editor body — see §2.5).

- State reads: `active_note`, `editing`, `slash_menu_pos`, `tabs`.
- State writes: `active_note.content`, `dirty`, `editing`, `active_note.frontmatter.*`, `slash_menu_pos`, `tabs.active`.
- Methods: keep `show_editor(ctx)`; split internals into `show_tab_strip(ui)`, `show_breadcrumb(ui)`, `show_edit_panel(ui)` (existing), `show_view_panel(ui)` (existing — but the rendering shifts to `pulldown-cmark`-based custom renderer in `md_render.rs`).

### 2.5 GUI right panel — NEW (320 px, toggleable)

Source: `07-omninote-obsidian.jsx` lines 444–538.

```
┌─[ Backlinks 7 ][ Outline 4 ][ AI Chat ●]──┐
│ ● claude-sonnet-4.5                      ▾│  ← provider switcher
├───────────────────────────────────────────┤
│  J │ Como está o status do módulo         │
│    │ Motoboys? Resumir os blockers.       │
│    │ [+ SPRINT.md] [+ Motoboys]           │
│                                           │
│  ✦ │ Status: 95% no branch `focusNFe`.    │
│    │ Três blockers ativos:                │
│    │  1. OWASP audit pendente — ver       │
│    │     [[OWASP-2021 checklist]]         │
│    │  2. [[Fee Tier Matrix]] ainda não    │
│    │     foi criada (broken)              │
│    │  3. Failsafe offline não validado    │
│    │     em campo                         │
│    │ O fluxo do app Flutter está fechado. │
│    │ ┌── SOURCES · 4 ───────────────┐     │
│    │ │ ↳ [[Motoboys module]]         │     │
│    │ │ ↳ [[SPRINT#review-2026-05-20]]│     │
│    │ │ ↳ [[iFood Linking]]           │     │
│    │ │ ↳ [[HUMAN#login-flow]]        │     │
│    │ └───────────────────────────────┘     │
├───────────────────────────────────────────┤
│ [2026-05-20.md ×]  [+ attach]             │
│ Pergunte ao vault…                        │
│ 📎 🎙                       ⌘↵   [Send]   │
└───────────────────────────────────────────┘
```

**Tabs (top, 36 px):**
- **Backlinks** — list of notes that reference current note. Show count badge in tab.
- **Outline** — current note's headings + block IDs. Click jumps + smooth-scrolls.
- **AI Chat** — chat-with-vault (RAG). Only enabled when `llm_provider != Disabled`.

Active tab = `text_strong`, `bg` bg, accent bottom border 2 px.

**Backlinks tab:**
```
3 incoming links

▤  2026-05-19.md
   "…ver [[Motoboys module]] pra spec atualizada…"
   (excerpt around link, 90 chars)

⌘  iFood Linking
   "Depende de [[Motoboys module]] estar fechado para…"

§  SPEC_V2 - NdA
   "Conforme [[Motoboys module]] §3, o fluxo de checklist…"
```
Each row: NoteType glyph, title, 1-line excerpt (90 chars around the link). Click → navigate.

**Outline tab:**
```
# 2026-05-20 · Quarta
  ## Sprint review
  ## Próximos passos
  ## Lições
```
Indent by heading level. Active heading (the one currently in viewport) gets accent left-border bar.

**AI Chat tab (only when `llm_provider != Disabled`):**

- Provider switcher row (top): green dot + monospace model id + dropdown caret. Click → opens settings modal positioned to LLM section.
- Messages area: scrollable column.
  - User msg: 18 px monogram on the left (panel2 bg, dim text). Message body 12.5 px, 1.55 line-height. Below: monochrome chips of "attached" notes added to context (`+ SPRINT.md`, `+ Motoboys`).
  - Assistant msg: 18 px monogram with accent border + `✦` accent glyph. Message body same. After body: collapsible "SOURCES · N" block (inset bg, border) listing wikilinks to cited notes.
- Input area:
  - Attached-notes chip strip (active accent chips with `×` removers, plus `+ attach` link).
  - Multiline TextEdit, hint "Pergunte ao vault…".
  - Bottom row: paperclip 📎 (attach note), mic 🎙 (dictate prompt), right side `⌘↵` kbd + accent Send button.

When `llm_provider == Disabled`: render placeholder with one CTA "Configure LLM in Settings (Ctrl+,)".

- State reads: `right_rail_tab`, `vault.config.llm_provider`, `chat_session` (new — see §5.6), `active_note`.
- State writes: `right_rail_tab`, `chat_session.messages`, `chat_session.attached_notes`.
- Methods: NEW `show_right_rail(ctx)` in new file `src/ui_right_rail.rs`. Sub-methods: `show_backlinks_tab`, `show_outline_tab`, `show_chat_tab`.

### 2.6 GUI command palette (Ctrl+P)

Source: `07-omninote-obsidian.jsx` lines 620–722.

```
                       ╭──── motob_ ─────────────────── [esc] ────╮
                       │ ⌘                                          │
                       │ [All] [Notes] [Commands] [Tags] [Discip.] │
                       │ ─────────────────────────────────────────  │
                       │  NOTES                                     │
                       │  ▤  Motob<accent>oys module                │
                       │       Specs · 2m ago                  ↵   │
                       │  ⌘  iFood Linking                          │
                       │       cfo-pocket · 4h ago                  │
                       │  ▤  Motoboy fee tiers                       │
                       │       Drafts · 2d ago                       │
                       │                                            │
                       │  COMMANDS                                  │
                       │  ✦  AI · Resumir nota atual         ⌘⇧S   │
                       │  +  Nova nota a partir de template  ⌘T    │
                       │  🎙 Dictation · Toggle gravação     ⌘⇧M   │
                       │  ⊞  Toggle right rail               ⌘\    │
                       │                                            │
                       │  TAGS                                      │
                       │  #motoboy #cfo-pocket #sprint              │
                       │ ─────────────────────────────────────────  │
                       │  ↑↓ navigate  ↵ open  ⌘↵ open in split    │
                       │                          23 results · 18ms│
                       ╰────────────────────────────────────────────╯
            (rest of UI dimmed behind 0.5 alpha overlay)
```

**Behavior:**
- Anchored center-top, 620 px wide, opens with Ctrl+P.
- Esc / click outside → close.
- Tab cycles scope chips (All → Notes → Commands → Tags → Disciplines → All).
- ↑↓ navigate result rows; ↵ opens; ⌘↵ opens in a new tab (split).
- Fuzzy match on title + content (notes) and verb + description (commands). Highlight matching chars in accent.
- Results capped at 50, sectioned. Performance budget: <50 ms for 1000-note vaults.

**Implementation:**
- New file `src/ui_palette.rs`.
- State: `palette_open: bool`, `palette_query: String`, `palette_scope: PaletteScope`, `palette_results: Vec<PaletteResult>`, `palette_cursor: usize`.
- Render with `egui::Window` (no title bar, anchored). Backdrop = full-screen `Area` painting `Color32::from_rgba_premultiplied(0,0,0,127)` on top of CentralPanel but below the Window.
- Fuzzy match: simple subsequence + position weight (port a tiny version of `fuzzy-matcher` crate, or just use the crate — `fuzzy-matcher = "0.3"` adds ~30 KB).

### 2.7 GUI slash menu (extended)

Already covered in §2.4 inline. Spec the items in code:

```rust
pub enum SlashCommand {
    // AI
    AiSummarize, AiAsk, AiTranslate, AiTagAuto, AiExtractTodos, AiExplainThis,
    // Templates (dynamic from Templates/)
    Template(String),
    // Discipline
    DiaryAppend, HumanAsk, TicketStatus, PlanNote,
    // Markdown (existing v0.8)
    H1, H2, H3, Bold, Italic, CodeInline, CodeBlock, Quote,
    BulletList, NumberedList, Todo, Link, Wikilink, Divider,
}
```

Each `SlashCommand` exposes `label() -> &str`, `description() -> &str`, `glyph() -> &str`, `kind() -> SlashKind` (for grouping), and `apply(note: &mut Note, slash_pos: usize)`.

Render order: AI → Templates → Discipline → Markdown. Each section has an uppercase header label.

Esc / `/`-cancel closes. Arrow keys + Enter to select.

### 2.8 GUI quick-capture popup (Ctrl+Shift+Space)

Source: `07-omninote-obsidian.jsx` lines 812–858.

```
              ╭──── ▼ Quick capture  → Inbox.md       ⌘⇧Space ────╮
              │                                                     │
              │  Lembrar: revisar a planilha de fee tiers com a Ju│ │
              │                                                     │
              │  TYPE  [▤ resumo]●  [? duvida]  [◇ exercicio]      │
              │                                                     │
              │ ─────────────────────────────────────────────────  │
              │  +0 attachments         esc cancel  ↵ [Capturar]   │
              ╰─────────────────────────────────────────────────────╯
```

- 520 px wide, anchored top-center 64 px from top.
- 14 px header + 14 px body padding.
- Header: ▼ glyph, "Quick capture" label, monospace destination "→ Inbox.md", right-aligned kbd hint.
- Body: single-line TextEdit (autofocus, growable to 4 lines), type chip row (defaults to last-used type), attachment counter.
- Submit (↵ or Capturar button): append a markdown bullet to `Inbox.md` of the form:
  ```
  - YYYY-MM-DD HH:MM · <type-glyph> <text>
  ```
- Cancel (Esc): close without saving.
- The window does NOT steal focus from the underlying editor when launched. Per CAD-24: bound to global hotkey via separate `omninote-capture` binary on macOS/Linux; in v1.2 GUI scope it's just the local-app shortcut (no global hotkey daemon yet).

State: `quick_capture_open: bool`, `quick_capture_buf: String`, `quick_capture_type: NoteType`.

Method: NEW `show_quick_capture(ctx)` in `src/ui_palette.rs` (small enough to co-locate with the command palette).

### 2.9 GUI daily-note auto-open

Triggered on `OmniNoteApp::new()`. Only fires when:
- `vault.config.daily_auto_open == true`
- File `Daily/YYYY-MM-DD.md` for `today` does not exist OR was not last accessed today.

Flow:
1. Compute today's date in user's local TZ.
2. Path = `<vault>/Daily/2026-05-20.md`.
3. If missing: read `Templates/daily.md`, render `{{date:YYYY-MM-DD}}`, `{{time}}`, `{{date_pt_br}}` placeholders, write the file.
4. Open as active note in **read mode** (not edit — user often just wants to glance).

No standalone UI surface — this is startup logic. Setting toggle lives in §3.13 settings panel.

### 2.10 GUI dictation surface (Ctrl+Shift+M)

Source: `07-omninote-obsidian.jsx` lines 555–566 (title bar pill) + 860–878 (overlay).

**Title-bar pill (always when recording):**
```
[● REC 00:34]   🎙 ⌘ ⚙
```
Red dot pulses, monospace timer, `red` border + faint `red` bg. Click pill → stop recording. Click 🎙 icon when idle → start recording.

**Bottom-center overlay (recording mode):**
```
       ╭─────────────────────────────────────────────╮
       │ ●  Dictation · recording                    │
       │    ▮▮▯▮▮▮▯▮▮▮▮▯▮▮▮▮▯▮▮▮ (waveform) 01:24   │
       │                              [⌘⇧M to stop] │
       ╰─────────────────────────────────────────────╯
```
- 380 px wide, 56 px from bottom.
- Pulsing red dot (12 px), label, 20-bar waveform (sampled from mic peak meter), elapsed time.
- Stop → spawn Whisper transcription → create new note pre-filled with transcript draft (`NoteType::Resumo`), open in edit mode.

State: `dictation: Option<DictationSession>` (new), where `DictationSession` holds start instant + bar buffer.

Method: NEW `show_dictation_overlay(ctx)` in `src/ui_dictation.rs`. Pill rendered inside title bar (see §3.13 chrome).

For v1.2 ship, the actual mic capture + Whisper wiring is CAD-23 scope (Sprint v1.3). v1.2 ships the UI surface with a stub that creates an empty note labeled "[dictation stub — wiring in v1.3]" so the UX flow is end-to-end testable.

### 2.11 GUI AI Chat panel

Covered in §2.5 (right-rail tab) and §3.5 (deeper).

### 2.12 CLI output style guide

Not a GUI surface but a UX surface. See §7 for full guide. Summary:
- Default human output: ANSI colors mirroring GUI accent (`accent`, `green` ok, `red` error). Tables via single-line borders.
- `--json` flag: every command emits `{ok: true|false, data: ..., error?: ...}` shape. No ANSI in JSON mode.
- Errors to stderr, exit code matches semantic (0 ok, 1 user error, 2 internal, 3 vault not found, 4 LLM provider not configured).

### 2.13 MCP tool descriptions

Not a GUI surface. See §8 full registry. The "UX" is the tool descriptions LLMs read — they must clearly state the verb, the side effects, and the JSON parameters. Format: 1-line summary + 3-line elaboration covering "when to use" + "side effects" + "return shape".

### 2.14 Web share target (`omninote://capture?...`)

Future-scope (CAD-24 Phase 5). The macOS plist registration + URL handler maps to the same code path as quick-capture: parse query string, append to `Inbox.md`, optionally focus the app.

No UI in v1.2 — only the URL handler stub.

### 2.15 Watcher event toasts

Source: `07-omninote-obsidian.jsx` lines 582–598, 611–615.

```
                                         ┌──────────────────────────┐
                                         │ Note reloaded          × │
                                         │ 2026-05-20.md was edited │
                                         │ by Obsidian — re-read    │
                                         │ from disk.               │
                                         └──────────────────────────┘
                                         ┌──────────────────────────┐
                                         │ OCR finished           × │
                                         │ paper.pdf · 12 pages ·   │
                                         │ paper.ocr.md linked.     │
                                         └──────────────────────────┘
```

- Bottom-right stack, 36 px from bottom (above status bar), 16 px right.
- Each toast: 240–320 px wide, 8×12 pad, left border 3 px in kind-color (info=accent, ok=green, warn=amber, err=red).
- Auto-dismiss after 5 s (warn/err: 8 s). Manual dismiss via `×`.
- Queue: max 4 visible; further events collapsed into "+ N more" toast.

State: `toasts: VecDeque<Toast>`. Each `Toast` carries `id`, `kind`, `title`, `body`, `created_at`.

Method: NEW `show_toasts(ctx)` in `src/ui_toasts.rs`. Called once per frame from `update()`.

Sources of toasts:
- Watcher: external file change → reload toast.
- OCR completion (CAD-23): "OCR finished" toast.
- Save errors: "Save failed" err toast.
- AI Chat: "Embedding cache rebuilt" info toast.
- Quick capture: "Captured to Inbox.md" ok toast (in title bar, not from popup, since popup closes).

---

## 3. Layout sketches — 17 artifacts

Each artifact is a discrete UI region that can be opened/closed/embedded. Several appear inside the entry-point sketches above; here we spec each in detail (data shape, edge cases, empty state, error states).

### 3.1 Note edit view

Already covered structurally in §2.4. Edit-specific concerns:

- **Title `TextEdit::singleline`** at top, fills width. Heading style. Hint "Título da nota". On commit (focus loss), `flush_active` renames the file via `vault.rename_note_by_id`.
- **Metadata row** (existing): Type ComboBox + Tags CSV input. Add: **Aliases** input — CSV of alternative titles that the resolver consults (see CAD-20 §A).
- **Frontmatter expandable strip** (collapsed by default): edit-mode shows the YAML inline with syntax highlight (use `egui::TextEdit::multiline` with `code_editor()`). Read-only mode in §3.2 below.
- **Citation-only fields** (existing): Source + URL inputs visible only when `NoteType == Citacao`.
- **Content `TextEdit::multiline`** with `code_editor()` style. Existing wiring: `cursor_range` extracted for Ctrl+= math and slash menu position. Continue.
- **Attach button** (existing): `📎 Anexar arquivo` → `rfd::FileDialog`, insert `![[name]]` at cursor.
- **NEW: Insert template button** next to attach: opens template picker, applies via `Templates::apply_to_note`.
- **NEW: Insert wikilink picker** (Ctrl+L when cursor is in textedit): opens lightweight palette (subset of §2.6) filtered to notes; insertion inserts `[[Title]]` at cursor.

Edge cases:
- Title empty + dirty + auto-save → file renamed to `Untitled.md` (or `Untitled (N).md` if collision).
- Title equals an existing filename → save errors; show toast "Cannot rename — file exists".
- Frontmatter contains keys not in `Frontmatter` struct → preserve via a `unknown: serde_yaml::Value` field (extend `Frontmatter` in Fase B).

### 3.2 Note read view + linked-note hover preview popup

Source: `07-omninote-obsidian.jsx` lines 418–437.

**View mode layout** (same body padding as edit mode for parity):

```
   # 2026-05-20 · Quarta
   #daily #sprint
   ─── (separator)
   <CommonMark-rendered body with inline wikilinks + embeds + tags>
   ─── (separator)
   🔗 3 incoming backlinks (collapsible)
   ─── (separator)
   📎 attachments / 🖼 images / 🔗 referenced notes (collapsible groups)
```

Existing render: `egui_commonmark::CommonMarkViewer`. **Migration plan:** replace with custom `pulldown-cmark` walker (per SPEC.md §6) that intercepts `Event::Text` to swap `[[…]]` and `![[…]]` and `#tag` tokens with `ui.link`/embed widgets. CAD-20 owns the parser side of this; UI side is Fase B of CAD-25.

**Hover preview popup** (when mouse hovers a `[[wikilink]]` for 500 ms):

```
                       ╭─────────────────────────────────╮
                       │ ▤  preview          3 backlinks │
                       │ iFood Linking                    │
                       │ Branch `focusNFe`. Sistema de    │
                       │ vinculação opcional de pedidos   │
                       │ do app ao backend do iFood, com  │
                       │ fallback para fluxo manual…      │
                       │ #integration  #cfo-pocket        │
                       ╰─────────────────────────────────╯
```

- 340 px wide, anchored near hover point (avoid screen edge — see `egui::AreaState::set_pos`).
- 12×14 pad, `panel` bg, 1 px `border_strong` border, shadow.
- Content: NoteType glyph + "preview" + backlink count, then title (14 px, 600), 5-line excerpt (12 px, 1.5 lh, `dim`), then tag chips.
- Hover-out (after 200 ms grace): close. Esc: close.
- Click anywhere in popup → navigate to target note.
- For broken wikilinks: popup shows "Note not found — click to create" instead.

State: `hover_preview: Option<HoverPreview>` where `HoverPreview = { target: String, anchor: Pos2, since: Instant }`.

Method: NEW `show_hover_preview(ctx)` in `src/md_render.rs` (where wikilink rendering lives).

### 3.3 Daily-note view

Same as read view (§3.2) but with **date-nav header**:

```
   ← yesterday: [[2026-05-19]]                    [[2026-05-21]] :tomorrow →
   ─── (subtle divider, accent if today) ───
   # 2026-05-20 · Quarta
   ...
```

- Prev/next links auto-resolve to `Daily/YYYY-MM-DD.md`. If missing on click → create from template, then navigate.
- If today's daily → render the date in `accent`, otherwise `text`.
- Add small calendar widget icon top-right (`📅`) → toggles calendar popover (§3.10).

Triggered by: opening any file in `Daily/`. Detect via `note.rel_path.starts_with("Daily")`.

Method: NEW `show_daily_view(ui, note)` in `src/ui_editor.rs`, called instead of generic view when `is_daily(note)`.

### 3.4 Discipline view — SPRINT.md

Source: `07-omninote-obsidian.jsx` lines 724–809.

```
┌─────────────────────────────────────────────────────────────────────┐
│ ◈  Sprint 26 · cfo-pocket   [ACTIVE]                              ⤤│
│ May 12 → May 26 · day 9 of 14 · 47 pts committed · 32 done        │
│                                                                     │
│ ████████████████████████████░░░░░░ progress                        │
│ ● 32 done · ● 6 in progress · ● 3 blocked · ● 6 todo               │
│                                                                     │
│ IN PROGRESS · 6                                                    │
│ □  SCRUM-241  Motoboys module — finalizar OWASP audit    motoboy  J 5p │
│ □  SCRUM-238  Inline embeds no editor (markdown render)  ui       J 8p │
│ ▣  SCRUM-235  Failsafe offline para fluxo de entrega    motoboy  M 3p │ (blocked → red border)
│                                                                     │
│ TODO · 6                                                           │
│ □  SCRUM-244  Criar nota Fee Tier Matrix                          J 2p │
│ □  SCRUM-243  MCP tool descriptions para todos os verbos CLI      J 5p │
│ □  SCRUM-242  Calendar widget para daily notes           ui       J 3p │
│                                                                     │
│ DONE · 32  (collapsed — click to expand)                           │
│ ☑  SCRUM-237  iFood linking — branch focusNFe            ui       J 8p │
│ ☑  SCRUM-234  Three-pane layout v1                                J 5p │
└─────────────────────────────────────────────────────────────────────┘
```

**Header row:**
- Sprint glyph `◈` + title (parsed from first H1 in SPRINT.md) + ACTIVE pill (green) or CLOSED (dim).
- Right link: "open SPRINT.md raw →" — switches to generic edit view of the file.

**Meta row:** date range + day counter + points (extracted from sprint frontmatter or parsed from table).

**Progress bar:** stacked horizontal — green (done) / amber (in progress) / red (blocked) / dim (todo). Total proportional to points.

**Legend row:** colored dots + counts.

**Task list (per status section):**
- Section header: uppercase 10 px, `dimmer`, letter-spacing 1.5.
- Each task row: status checkbox (color per status) + monospace ID (60 px) + title + optional kind tag + optional `#tag` chip + owner avatar + points.
- Status colors: `done → green`, `doing → amber`, `todo → dim`, `blocked → red`.
- Click row → opens the underlying ticket spec note in `SPECS/CAD-XX.md` (or whatever the file convention is).
- Right-click → context menu: change status, edit, copy ID.

**Parser side:** new `SprintParser` reads `SPRINT.md` and extracts task rows from markdown tables. Convention: any markdown table with columns matching `[ID, Tarefa, Status, ...]` becomes a task list. Fall back to bullet-list parsing for non-table SPRINTs.

**Edit mode:** for any discipline view, a toggle `Typed ↔ Raw` lives in the breadcrumb row. Raw mode = generic editor. Typed = this bespoke widget. Typed mode is read-only initially; edit affordances (drag to reorder, click-to-toggle status) ship in v1.3 if there's demand.

Method: NEW `show_sprint_view(ui, note)` in `src/ui_discipline.rs`.

### 3.5 Discipline view — DIARY.md

```
┌─────────────────────────────────────────────────────────────────────┐
│ ✎  DIARY · cfo-pocket                                               │
│ 47 entries · last 2026-05-20 14:32                                 │
├─────────────────────────────────────────────────────────────────────┤
│ ▼ 2026-05-20  (3 entries)                                           │
│   14:32 [CAD-25 fase a]  Wrote UI_DESIGN_v2.md covering 15 entry…  │
│   11:08 [pre-merge-coverage]  Ran on CAD-20 PR — added 12 adver…   │
│   08:42 [session-start]  Pulled main, checked SPRINT, picked CA…   │
│                                                                     │
│ ▼ 2026-05-19  (2 entries)                                           │
│   23:14 [CAD-12 done]  Coverage hardened to 96.61%. Closed PR…     │
│   16:08 [codex-cross-review]  Found 2 P2 issues in vault.rs path…  │
│                                                                     │
│ ▶ 2026-05-18  (1 entry)                                             │
│ ▶ 2026-05-17  (4 entries)                                           │
│ ...                                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Header:** ✎ glyph + "DIARY · <project>" + meta (entry count, last entry timestamp).

**Body:** entries grouped by date (newest first). Each date is a collapsing header with entry count. Expanded entries show:
- HH:MM timestamp (mono, dim, 10 px).
- `[label]` chip (extracted from `[…]` markers in the entry — `[CAD-XX done]`, `[plan-mode-bypass]`, etc).
- First 80 chars of the entry body (truncated).
- Click row → opens raw DIARY.md scrolled to that entry.

**Edit affordance:** dedicated "+ Append entry" button top-right opens a dialog with a multi-line input + label dropdown. On submit, appends a fresh entry to DIARY.md following the discipline format.

Method: NEW `show_diary_view(ui, note)` in `src/ui_discipline.rs`.

### 3.6 Tickets panel — JIRA.md + NOTION.md merged

```
┌─────────────────────────────────────────────────────────────────────┐
│ Tickets   [ All ] [ In Progress ] [ Todo ] [ Done ]   ⌕ filter…    │
├─────────────────────────────────────────────────────────────────────┤
│ ACTIVE                                                              │
│  NOTION  CAD-25  🎯 Em desenvolvimento                              │
│          UI Design v2 — implementar handoff Claude Design          │
│          Sprint v1.1 · ~8h · last edit 14:32                       │
│                                                                     │
│  JIRA    SCRUM-241  🚧 In Progress · blocker: OWASP                │
│          Motoboys module — finalizar OWASP audit                   │
│          Sprint 26 · 5p · J · #motoboy                              │
│                                                                     │
│ TODO                                                                │
│  NOTION  CAD-22  🌱 A fazer                                         │
│          Daily/Templates/Discipline CLI+MCP                         │
│          Sprint v1.2 · 18h                                          │
│                                                                     │
│ DONE (recent — collapsed)                                          │
│  NOTION  CAD-20  ✅ Concluída                                       │
│  JIRA    SCRUM-234  ✅ Done                                         │
└─────────────────────────────────────────────────────────────────────┘
```

**Header:** "Tickets" + filter chips (All / status buckets) + search input.

**Row format:**
- Provider tag (NOTION/JIRA), monospace 10 px, color-coded (`accent` for NOTION, `blue` for JIRA).
- Ticket ID (mono, 11 px).
- Status icon + label (color per state).
- Title (text, 13 px).
- Meta row (12 px dim): sprint, points, owner, tags.

Click row → open the ticket spec `SPECS/CAD-XX.md` (NOTION) or `SPECS/SCRUM-XX.md` (JIRA, if local mirror exists). If no local mirror, open browser to the ticket URL parsed from frontmatter.

**Parser side:** read both `JIRA.md` and `NOTION.md`, parse the standard `### CAD-XX · …` / `### SCRUM-XX · …` block format. Merge, sort by status then by date.

**Sync indicators:**
- Tiny `⟲` icon next to a ticket = remote state newer than local. Click → run `omninote ticket pull <ID>` (MCP-backed; out of v1.2 scope, stub).
- Tiny `⤴` icon = local newer than remote. Click → push.

Method: NEW `show_tickets_panel(ui)` in `src/ui_discipline.rs`. Toggleable via Ctrl+Shift+J. Renders in CentralPanel when active, replacing editor body.

### 3.7 AI Chat panel

Already specified in §2.5.

**Additional details:**

- **First-run state:** if no messages yet, show greeter card: "Pergunte sobre seu vault.\nTente: `Que decisão tomei sobre escrow?` ou `Resumir SPRINT atual`."
- **Provider error state:** if last LLM call failed, persistent banner at top of messages area in `red`/dim: "Última chamada falhou: <error>. [Retry]".
- **Attached notes context:** chips render with `accent_bg` border, accent text, `×` remover. Limit 8 attached notes (model context window heuristic — show warning above limit).
- **Sources rendering:** each source is `[[wikilink]]`-styled, clickable. Click → navigate to that note + scroll to relevant section if anchor is present.
- **Conversation persistence:** chat history stored in `.omninote/chat_sessions/<uuid>.json`. Last 10 conversations retained. Optional: ship "Clear chat" + "New chat" buttons in tab header.

### 3.8 Command palette

Already specified in §2.6.

**Additional details:**
- **Empty query:** show recent notes (5) + recent commands (5).
- **Tag query (`#`-prefix):** filter scope to tags automatically.
- **Discipline query (`@`-prefix):** filter scope to discipline files.
- **Result row hover:** subtle `panel2` bg.
- **Selected row:** `accent_bg` + 2 px accent left border + `text_strong` color.
- **Performance:** fuzzy-match cached per query prefix; 1000-note vault should sort in <30 ms.

### 3.9 Quick-capture popup

Already specified in §2.8.

### 3.10 Calendar widget (daily-note picker)

```
                       ╭─────────────── May 2026 ───────────────╮
                       │  ‹                                    › │
                       │  Sun Mon Tue Wed Thu Fri Sat            │
                       │                                          │
                       │       1   2   3   4   5   6             │
                       │   7•  8•  9  10• 11  12  13•           │
                       │  14  15  16  17• 18  19  20◉            │
                       │  21  22  23  24  25  26  27             │
                       │  28  29  30  31                          │
                       │                                          │
                       │  [Go to today]  [Settings ⚙]            │
                       ╰──────────────────────────────────────────╯
```

- 7-column grid.
- Dot under day = daily note exists. Filled dot (`●`) = has content > 200 chars. Empty dot (`○`) for stub.
- Today highlighted with `◉` accent ring.
- Click day → open or create daily note.
- ←/→ in widget: navigate month.
- Anchored to whichever 📅 icon was clicked (sidebar "open calendar…" link, daily-view 📅 button).

Method: NEW `show_calendar(ctx, anchor)` in `src/ui_calendar.rs`.

### 3.11 Tag explorer

```
┌──────────────────────────────────────────────────────────────────┐
│ Tags   ⌕ filter…                                Sort: [count▾]  │
├──────────────────────────────────────────────────────────────────┤
│  #sprint           42  ████████████████                          │
│  #cfo-pocket       38  ███████████████                           │
│  #motoboy          22  ██████████                                │
│  #ux               14  ██████                                    │
│  #security         11  █████                                     │
│  #api               8  ████                                      │
│  ...                                                              │
└──────────────────────────────────────────────────────────────────┘
```

- Two columns: tag chip + count + horizontal bar (proportional).
- Sort dropdown: count (desc), alpha (asc).
- Click chip → set sidebar `query = #tag`, switch back to main view, show notes filtered.

Triggered: Ctrl+Shift+T, or sidebar footer ♯ icon. Toggles a CentralPanel takeover (same pattern as Tickets panel §3.6).

Method: NEW `show_tag_explorer(ui)` in `src/ui_palette.rs` (small enough).

### 3.12 Backlinks + outline panel

Both are right-rail tabs — covered in §2.5.

### 3.13 Vault picker / multi-vault switcher

**Sidebar header dropdown:**

```
┌─────────────────────────────────────────────┐
│ ⬢  ClaudeBook ▾                           ⚙ │
└─────────────────────────────────────────────┘
                ↓ click ↓
       ╭─────────────────────────────────╮
       │ Recent                           │
       │  ●  ClaudeBook   ~/Projects/c…  │
       │  ○  Obsidian Vault  ~/Documen…  │
       │  ○  archive  ~/old/notes        │
       │ ─────────────────────────────── │
       │  ⌂  + Add vault…                │
       │  ⚙  Manage vaults…              │
       ╰─────────────────────────────────╯
```

- Dropdown opens on click of vault row in sidebar header.
- Recent list pulled from `AppConfig.recent_vaults: Vec<PathBuf>` (already in v1.0!).
- Active vault marked `●`. Click another → close current, open chosen.
- "+ Add vault" → `rfd::FileDialog::pick_folder`, append to recents, switch.
- "Manage vaults" → opens settings modal at the "Vaults" section (rename/remove from recents).

Method: NEW `show_vault_switcher(ctx, anchor)` in `src/ui_sidebar.rs` (small enough).

### 3.14 Settings panel — extended

Source: existing `ui_modals.rs::show_modal_settings` + new fields from §1.7.

**Layout (single column, scrollable, max 600 px wide):**

```
╭─────────── Configurações ──────────────────────────────╮
│  GERAL                                                 │
│  ☑ Modo escuro                                         │
│  Tema preset: [Obsidian Dark ▾] (Obsidian Light /      │
│                                  High Contrast / Custom)│
│  Cor de destaque (accent):  [█] #8b7cff                │
│                                                         │
│  ACESSIBILIDADE                                        │
│  Fonte: [Sistema (sans-serif) ▾]                       │
│  Tamanho: [────●────] 14pt                             │
│  Espaço entre linhas: [───●──] 1.4                     │
│  ↩ Restaurar padrões                                   │
│                                                         │
│  PAINEL DIREITO                                        │
│  ☑ Abrir ao iniciar                                    │
│  Aba padrão: [AI Chat ▾]                               │
│                                                         │
│  DAILY NOTES                                           │
│  ☐ Abrir nota diária automaticamente                   │
│  Template: [Templates/daily.md ▾]                      │
│                                                         │
│  AI / LLM                                              │
│  Provider: [Claude ▾]  (Claude / Grok / Ollama / Off)  │
│  Model ID: claude-sonnet-4.5                           │
│  API key: ●●●●●●●●●●●●● (.omninote/llm.toml)            │
│  ↪ Editar llm.toml                                     │
│                                                         │
│  ATALHOS                                               │
│  Quick capture: ⌘⇧Space     [edit]                     │
│  Command palette: ⌘P        [edit]                     │
│  Dictation: ⌘⇧M             [edit]                     │
│  Toggle dark: ⌘⇧D           [edit]                     │
│                                                         │
│  VAULTS                                                │
│  Vault atual: ~/Projects/caderno                       │
│  Recentes:                                              │
│   • ~/Documents/Obsidian Vault       [open] [remove]   │
│   • ~/old/archive                    [open] [remove]   │
│  [📂 Trocar vault]                                     │
│                                                         │
│  AVANÇADO                                              │
│  ↪ Editar .omninote/config.json (raw)                  │
│  ↪ Ver logs                                            │
│  ↪ Sobre / versão                                      │
╰────────────────────────────────────────────────────────╯
```

- Sections separated with section headers (uppercase 10 px) and dividers.
- "Atalhos / [edit]" — opens a 1-line capture-keystroke modal; user presses the desired combo; on capture, persist. Hard-block reserved combos (Ctrl+Q etc).
- "Editar llm.toml" / "Editar config.json (raw)" / "Ver logs" → `open::that(path)` open in default editor.

Method: keep existing `show_modal_settings(ctx)`; refactor body into sub-renders `settings_section_general`, `settings_section_a11y`, `settings_section_rail`, `settings_section_daily`, `settings_section_ai`, `settings_section_keys`, `settings_section_vaults`, `settings_section_advanced`.

### 3.15 Onboarding / first-run modal

Triggered on first vault open (detect via `vault.config` being defaults + `vault.notes.is_empty()`).

```
╭──────────────── 👋 Bem-vindo ao OmniNote ────────────╮
│                                                       │
│  Seu vault está vazio. Vamos começar?                 │
│                                                       │
│  [📓 Criar nota de boas-vindas]                       │
│      Cria uma nota explicando o app, tipos, atalhos.  │
│                                                       │
│  [📅 Habilitar daily notes]                           │
│      Cria pasta Daily/ + Templates/daily.md template. │
│                                                       │
│  [📁 Importar vault Obsidian existente]               │
│      Selecione pasta de vault — sem conversão.        │
│                                                       │
│  [⏭ Pular — só explorar]                              │
│                                                       │
│  ─────────────────────────────────────────────       │
│  Atalhos essenciais:                                  │
│   ⌘N  Nova nota         ⌘P  Command palette          │
│   ⌘K  Buscar            ⌘⇧Space  Quick capture       │
╰───────────────────────────────────────────────────────╯
```

- Single CentralPanel takeover (not a `Window`, so it can't be accidentally minimized).
- Each option triggers a side effect + closes onboarding.
- Pulse-highlight the sidebar `Today` section after closing if "Enable daily notes" was selected.

Method: NEW `show_onboarding(ctx)` in `src/ui_modals.rs`.

### 3.16 Toast queue

Already specified in §2.15.

### 3.17 Timeline view (snapshot diff)

```
┌─────────────────────────────────────────────────────────────────────┐
│ Timeline   [Today ▾]  [This week]  [This month]  [Since: 1d ▾]      │
├─────────────────────────────────────────────────────────────────────┤
│ 2026-05-20  Wed                                                     │
│  14:32  ✎ DIARY.md  (+47 lines)                                     │
│  14:08  ✎ SPRINT.md  (-3 lines, +5 lines)                           │
│  11:42  ⊕ SPECS/CAD-25.md  (created)                                │
│                                                                     │
│ 2026-05-19  Tue                                                     │
│  23:14  ✎ NOTION.md  (+12 lines)                                    │
│  23:14  ✎ JIRA.md  (+8 lines)                                       │
│  17:08  ⊝ Notes/old-spec.md  (deleted, was 412 lines)               │
│                                                                     │
│ 2026-05-18  Mon                                                     │
│  ...                                                                │
└─────────────────────────────────────────────────────────────────────┘
```

- Each row: timestamp + change-glyph (✎ edit, ⊕ created, ⊝ deleted, ⤴ renamed) + path + line-delta.
- Click row → opens a side-by-side diff modal (new) showing the before/after of that change.
- Filter chips switch the time window.
- Empty state: "No changes in the selected window — try expanding."

**Implementation:** wrap `git log -- :^.omninote :^_attachments --since=<window>` via `git2` crate. Graceful fallback when vault is not a git repo: show "Initialize git in vault to enable timeline" + button.

Method: NEW `show_timeline_view(ui)` in `src/ui_timeline.rs`. Toggleable via Ctrl+Shift+H.

### 3.18 Dictation overlay

Already specified in §2.10.

### 3.19 Tab strip (NEW concept)

Not in the original 17 list, but the editor tab strip is a separate concern worth specifying.

- State: `tabs: Vec<TabId>` where `TabId = NoteId`. `tab_active: Option<usize>` (index into `tabs`).
- Open new tab: `Ctrl+T` (in palette: select + ⌘↵) or middle-click on sidebar note row.
- Close tab: `Ctrl+W` or click `×`. Closing active → switch to neighbor (right preferred).
- Reorder: drag tab.
- Tabs persist across sessions in `AppConfig.open_tabs: Vec<String>` (note IDs).
- Max 12 visible; overflow → horizontal scroll the strip.

For v1.2 MVP: ship single-tab behavior matching existing app (one active note), reserve the multi-tab feature for v1.3. This keeps Fase B scope manageable. Render the tab strip in single-tab mode anyway so the visual chrome is consistent.

---

## 4. State map

The full state surface of `OmniNoteApp`, grouped by concern. Existing fields marked `[v1.0]`; new fields `[v1.2]`.

### 4.1 State fields

```rust
pub struct OmniNoteApp {
    // Core (v1.0)
    pub vault: Option<Vault>,                       // [v1.0]
    pub active_note: Option<Note>,                  // [v1.0]
    pub editing: bool,                              // [v1.0]
    pub query: String,                              // [v1.0]
    pub type_filter: Option<NoteType>,              // [v1.0]
    pub dirty: bool,                                // [v1.0]
    pub last_save: Instant,                         // [v1.0]
    pub error_msg: Option<String>,                  // [v1.0]
    pub md_cache: CommonMarkCache,                  // [v1.0]
    pub watcher: Option<VaultWatcher>,              // [v1.0]
    pub self_write_until: Instant,                  // [v1.0]
    pub external_change_pending: bool,              // [v1.0]
    pub slash_menu_pos: Option<usize>,              // [v1.0]
    pub confirm_action: Option<ConfirmAction>,      // [v1.0]
    pub show_settings: bool,                        // [v1.0]
    pub show_new: bool,                             // [v1.0]
    pub show_import: bool,                          // [v1.0]

    // Tabs (v1.2 — placeholder for v1.3 multi-tab)
    pub tabs: Vec<String>,                          // note IDs [v1.2]
    pub tab_active: Option<usize>,                  // [v1.2]

    // Right rail (v1.2)
    pub right_rail_open: bool,                      // [v1.2]
    pub right_rail_tab: RightRailTab,               // [v1.2]
    pub outline_cache: Option<OutlineCache>,        // [v1.2]
    pub backlinks_cache: Option<BacklinksCache>,    // [v1.2]

    // AI chat (v1.2)
    pub chat_session: ChatSession,                  // [v1.2]
    pub chat_in_flight: bool,                       // [v1.2]

    // Command palette (v1.2)
    pub palette_open: bool,                         // [v1.2]
    pub palette_query: String,                      // [v1.2]
    pub palette_scope: PaletteScope,                // [v1.2]
    pub palette_results: Vec<PaletteResult>,        // [v1.2]
    pub palette_cursor: usize,                      // [v1.2]

    // Quick capture (v1.2)
    pub quick_capture_open: bool,                   // [v1.2]
    pub quick_capture_buf: String,                  // [v1.2]
    pub quick_capture_type: NoteType,               // [v1.2]

    // Dictation (v1.2)
    pub dictation: Option<DictationSession>,        // [v1.2]

    // Toasts (v1.2)
    pub toasts: VecDeque<Toast>,                    // [v1.2]

    // Calendar (v1.2)
    pub calendar_open_anchor: Option<Pos2>,         // [v1.2] None = closed

    // Hover preview (v1.2)
    pub hover_preview: Option<HoverPreview>,        // [v1.2]

    // Discipline / Tickets / Tag / Timeline overlays (v1.2)
    pub central_overlay: CentralOverlay,            // [v1.2] enum
    // CentralOverlay::None | Tickets | TagExplorer | Timeline | Onboarding

    // Vault switcher (v1.2)
    pub vault_switcher_open: bool,                  // [v1.2]
}
```

### 4.2 New supporting types

```rust
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RightRailTab {
    Backlinks,
    Outline,
    #[default]
    AiChat,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmProvider {
    Claude, Grok, Ollama,
    #[default]
    Disabled,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemePreset {
    #[default]
    ObsidianDark,
    ObsidianLight,
    HighContrast,
    Custom,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum PaletteScope {
    #[default]
    All,
    Notes,
    Commands,
    Tags,
    Disciplines,
}

#[derive(Clone, Debug)]
pub struct PaletteResult {
    pub kind: PaletteKind,         // Note | Command | Tag | Discipline
    pub id: String,
    pub label: String,
    pub meta: String,              // "Specs · 2m ago" / shortcut display
    pub score: i64,                // fuzzy match score
    pub glyph: &'static str,
    pub color: Color32,
}

#[derive(Clone, Debug, Default)]
pub struct ChatSession {
    pub id: String,                // uuid
    pub messages: Vec<ChatMessage>,
    pub attached_notes: Vec<String>, // note IDs
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub body: String,
    pub sources: Vec<String>,      // wikilink targets cited
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRole { User, Assistant, System }

#[derive(Clone, Debug)]
pub struct DictationSession {
    pub started_at: Instant,
    pub waveform: VecDeque<f32>,   // last N peak samples
    pub status: DictationStatus,   // Recording | Transcribing | Done
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,           // Info | Ok | Warn | Err
    pub title: String,
    pub body: String,
    pub created_at: Instant,
}

#[derive(Clone, Debug)]
pub struct HoverPreview {
    pub target: String,            // wikilink target text
    pub anchor: Pos2,
    pub since: Instant,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum CentralOverlay {
    #[default]
    None,
    Tickets,
    TagExplorer,
    Timeline,
    Onboarding,
}

#[derive(Clone, Debug)]
pub struct OutlineCache {
    pub note_id: String,           // invalidate when active note changes
    pub headings: Vec<OutlineEntry>,
}

pub struct OutlineEntry {
    pub level: u8,                 // 1..6
    pub text: String,
    pub line: usize,
}

#[derive(Clone, Debug)]
pub struct BacklinksCache {
    pub note_id: String,
    pub entries: Vec<BacklinkEntry>,
}

pub struct BacklinkEntry {
    pub source_note_id: String,
    pub title: String,
    pub excerpt: String,           // 90-char window around the link
    pub note_type: NoteType,
}
```

### 4.3 State transition diagram

Notation: `Event → state delta`. Events grouped by source.

#### Vault-level
- `pick_vault()` → `vault = Some(_); active_note = None; outline_cache = None; backlinks_cache = None; tabs = []; tab_active = None`.
- `vault open → daily_auto_open == true && today's daily missing` → create daily, `active_note = Some(daily)`, `tabs = [daily.id]`, `tab_active = Some(0)`.
- `vault.reload_notes()` (from watcher) → `backlinks_cache = None; outline_cache = None` (invalidate). If active note's file changed and not dirty → refresh `active_note` content.

#### Active note lifecycle
- `select_note(id)` → `flush_active()` first, then `active_note = Some(_)`, `editing = false`, `dirty = false`, `outline_cache = None`, `backlinks_cache = None`. If `id` not already in `tabs` → append; `tab_active = Some(position_of(id))`.
- `note edit (TextEdit changed)` → `dirty = true`.
- `flush_active()` triggered every frame when `dirty && last_save.elapsed() > 600ms` → save to disk, `dirty = false; last_save = now()`.
- `Ctrl+E` → `editing = !editing` (only if active).
- `delete_note(id)` → if matches active, `active_note = None`. Remove from `tabs`; if active tab, switch to neighbor.

#### Right rail
- `Ctrl+\` → `right_rail_open = !right_rail_open`. Persist to `config.right_rail_open`.
- Click tab → `right_rail_tab = X`. Persist.
- `chat send` → `chat_in_flight = true`. Spawn LLM call. On response → push message, `chat_in_flight = false`. On error → push system message + toast.
- `chat_session.attached_notes` mutations from chip add/remove.

#### Command palette
- `Ctrl+P` → `palette_open = true; palette_query = ""; palette_scope = All; palette_cursor = 0; palette_results = compute(empty, All)`.
- `palette text changed` → `palette_results = compute(query, scope); palette_cursor = 0`.
- `palette Tab` → cycle scope, recompute.
- `palette ↑/↓` → adjust cursor (clamp).
- `palette ↵` → dispatch result + `palette_open = false`.
- `palette ⌘↵` → dispatch result in new tab.
- `palette Esc` → `palette_open = false`.

#### Quick capture
- `Ctrl+Shift+Space` → `quick_capture_open = true; quick_capture_buf = ""; quick_capture_type = last_used_type`. Autofocus textedit (no focus steal from underlying app per CAD-24 — but in v1.2 in-app shortcut, focus is fine).
- `quick capture submit` → append to `Inbox.md`, `quick_capture_open = false`, toast "Captured to Inbox.md", reload `vault.notes`.
- `Esc` → `quick_capture_open = false`.

#### Dictation
- `Ctrl+Shift+M` (toggle) → if `dictation == None`: start mic capture, `dictation = Some(DictationSession { Recording, ... })`. If `dictation == Some(Recording)`: stop mic, set `Transcribing`, spawn Whisper job. On Whisper response: create note + select it, `dictation = None`, toast.
- Pull mic peak each frame → `dictation.waveform.push(peak); waveform.pop_front() if > 20`.

#### Toasts
- Append: `toasts.push_back(Toast { ... })`.
- Frame tick: drop toasts where `created_at.elapsed() > kind.timeout()`.
- User dismiss: remove by id.

#### Central overlays (mutually exclusive)
- `Ctrl+Shift+J` → `central_overlay = if Tickets { None } else { Tickets }`.
- `Ctrl+Shift+T` → `central_overlay = if TagExplorer { None } else { TagExplorer }`.
- `Ctrl+Shift+H` → `central_overlay = if Timeline { None } else { Timeline }`.
- Open onboarding → `central_overlay = Onboarding`.

#### Hover preview
- `mouse hover wikilink for 500ms` → `hover_preview = Some(HoverPreview { target, anchor, since: now() })`.
- `mouse out (200ms grace)` → `hover_preview = None`.
- `Esc` → `hover_preview = None`.

#### Sidebar
- `search query change` → `query = text` (filters tree on next render).
- Click type chip → toggle `type_filter`.
- Click discipline file → `select_note(id)`.
- Click project folder → expand inline (no state change to active note).
- Click vault dropdown → `vault_switcher_open = true`.

#### Settings
- `Ctrl+,` → `show_settings = true`.
- Theme preset change → rebuild `Theme`, `apply_style(ctx)`, persist.
- Font/size/spacing change → `apply_style(ctx)`, persist.
- LLM provider change → may trigger `chat_session` reset prompt (toast warning if mid-conversation).
- Daily auto-open toggle → persist; takes effect next session.

### 4.4 Save / dirty semantics

Unchanged from v1.0:

- Any `note.*` mutation (title, content, frontmatter) sets `dirty = true`.
- `update()` runs every frame. If `dirty && elapsed > 600ms`, calls `flush_active()`.
- `flush_active()`: take ownership of `active_note`, persist to disk, return Note to `active_note`.
- `on_exit`: forced final `flush_active()` + save config.

New rules:
- Editing in raw mode of a discipline file → same dirty semantics.
- Quick-capture write to Inbox.md is **synchronous** (not via dirty queue) — append + close.
- AI Chat sending writes to `chat_session.messages` in memory only; persisted to `.omninote/chat_sessions/<uuid>.json` debounced 2 s after last activity.

### 4.5 Conflict / external change handling

Existing v1.0 behavior (in `ui_modals.rs::show_modal_external_change`): when watcher fires on the active note AND dirty=true, show conflict modal with "Reload" vs "Keep edits".

New: extend conflict modal with 3rd option "**Open diff**" — opens a side-by-side diff modal showing the in-memory version vs disk version. User picks per-line.

For non-active notes that change externally: silently `vault.reload_notes()`, emit `info` toast "Note X reloaded".

---

## 5. Egui code structure proposal

### 5.1 Workspace + crate layout (post CAD-21)

Per SPRINT v1.1 §0 rule 11, by Fase B the workspace is already split:

```
omninote/
├── Cargo.toml          (workspace)
├── crates/
│   ├── omninote-core/  (lib — vault, wikilinks, resolver, search,
│   │                    templates, daily, discipline, frontmatter)
│   ├── omninote-gui/   (current src/* minus core, plus all new ui_*.rs)
│   ├── omninote-cli/   (clap binary)
│   └── omninote-mcp/   (rmcp server)
```

UI work for CAD-25 Fase B lives entirely under `crates/omninote-gui/src/`.

### 5.2 GUI source files — proposed splits

| File | Status | Purpose |
|---|---|---|
| `app.rs` | extend | `OmniNoteApp` struct + `update()` orchestrator. Push field additions per §4.1. |
| `theme.rs` | NEW | `Theme` struct + `Theme::obsidian_dark/light/high_contrast/custom`. All color tokens. |
| `ui_titlebar.rs` | NEW | Top chrome — vault badge, nav, REC pill, dictation mic, palette button, settings. |
| `ui_statusbar.rs` | NEW | Bottom chrome — saved status, cursor pos, file type, sync indicator, LLM model, version. |
| `ui_sidebar.rs` | extend | Existing 280 px panel + new section helpers (`show_today_section`, etc) + vault switcher. |
| `ui_tabs.rs` | NEW | Tab strip above editor (single-tab in v1.2, multi-tab in v1.3). |
| `ui_breadcrumb.rs` | NEW | Editor breadcrumb row (path + heading anchor + view/edit toggle). |
| `ui_editor.rs` | extend | Existing edit + view panel logic. Add daily-view dispatch, hover preview hooks, inline embed cards. |
| `md_render.rs` | NEW | Custom markdown renderer (pulldown-cmark walker) replacing `egui_commonmark::CommonMarkViewer` for the OmniNote-specific tokens (wikilinks, embeds, hashtags). |
| `ui_right_rail.rs` | NEW | 320 px right panel + tab dispatch (Backlinks / Outline / AiChat). |
| `ui_chat.rs` | NEW | AI Chat tab sub-view (sub-call of `show_right_rail`). |
| `ui_palette.rs` | NEW | Command palette (Ctrl+P) + tag explorer overlay + quick-capture popup. |
| `ui_calendar.rs` | NEW | Calendar popover (daily picker). |
| `ui_discipline.rs` | NEW | Typed views: `show_sprint_view`, `show_diary_view`, `show_human_view`, `show_plan_view`, `show_tickets_panel`. |
| `ui_timeline.rs` | NEW | Snapshot diff view (git-aware). |
| `ui_dictation.rs` | NEW | Dictation overlay + mic peak meter. |
| `ui_toasts.rs` | NEW | Bottom-right toast queue. |
| `ui_modals.rs` | extend | Existing 4 modals + onboarding + conflict modal v2 + diff modal. |
| `actions.rs` | extend | Action handlers (existing post CAD-12 refactor). Add: `quick_capture_submit`, `chat_send`, `palette_dispatch`, `dictation_toggle`, `theme_apply_preset`. |

Total: 17 GUI source files (5 existing extended + 12 new). All `impl OmniNoteApp` blocks split across them.

### 5.3 Method signatures — new `show_*` methods

```rust
impl OmniNoteApp {
    // Chrome
    pub fn show_titlebar(&mut self, ctx: &egui::Context);
    pub fn show_statusbar(&mut self, ctx: &egui::Context);

    // Sidebar helpers (private)
    fn show_today_section(&mut self, ui: &mut egui::Ui);
    fn show_inbox_section(&mut self, ui: &mut egui::Ui);
    fn show_discipline_section(&mut self, ui: &mut egui::Ui);
    fn show_projects_section(&mut self, ui: &mut egui::Ui);
    fn show_vault_section(&mut self, ui: &mut egui::Ui); // existing tree
    fn show_vault_switcher(&mut self, ctx: &egui::Context, anchor: egui::Pos2);

    // Editor surfaces
    pub fn show_tab_strip(&mut self, ui: &mut egui::Ui);
    pub fn show_breadcrumb(&mut self, ui: &mut egui::Ui);
    pub fn show_daily_view(&mut self, ui: &mut egui::Ui, note: &Note);
    pub fn show_hover_preview(&mut self, ctx: &egui::Context);

    // Right rail
    pub fn show_right_rail(&mut self, ctx: &egui::Context);
    fn show_backlinks_tab(&mut self, ui: &mut egui::Ui);
    fn show_outline_tab(&mut self, ui: &mut egui::Ui);
    fn show_chat_tab(&mut self, ui: &mut egui::Ui);

    // Palette + capture
    pub fn show_command_palette(&mut self, ctx: &egui::Context);
    pub fn show_quick_capture(&mut self, ctx: &egui::Context);
    pub fn show_tag_explorer(&mut self, ui: &mut egui::Ui);

    // Discipline typed views
    pub fn show_sprint_view(&mut self, ui: &mut egui::Ui, note: &Note);
    pub fn show_diary_view(&mut self, ui: &mut egui::Ui, note: &Note);
    pub fn show_human_view(&mut self, ui: &mut egui::Ui, note: &Note);
    pub fn show_plan_view(&mut self, ui: &mut egui::Ui, note: &Note);
    pub fn show_tickets_panel(&mut self, ui: &mut egui::Ui);

    // Calendar + timeline + dictation
    pub fn show_calendar(&mut self, ctx: &egui::Context);
    pub fn show_timeline_view(&mut self, ui: &mut egui::Ui);
    pub fn show_dictation_overlay(&mut self, ctx: &egui::Context);

    // Toasts + onboarding + diff modal
    pub fn show_toasts(&mut self, ctx: &egui::Context);
    pub fn show_onboarding(&mut self, ctx: &egui::Context);
    pub fn show_diff_modal(&mut self, ctx: &egui::Context);

    // Custom markdown
    // (in md_render.rs as free fn, takes &mut self for navigation hooks)
    pub fn render_markdown(&mut self, ui: &mut egui::Ui, content: &str);
}
```

### 5.4 `update()` loop refactor

```rust
impl eframe::App for OmniNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0. Time-based housekeeping
        self.tick_autosave();          // existing 600ms flush
        self.poll_watcher();           // existing
        self.tick_toasts(ctx);         // drop expired
        self.tick_dictation();         // pull mic peaks if recording
        ctx.request_repaint_after(Duration::from_millis(500));

        // 1. Global shortcuts (consume keys before any TextEdit grabs them)
        self.handle_global_shortcuts(ctx);

        // 2. Cold-start fast path
        if self.vault.is_none() {
            self.show_cold_start(ctx);
            return;
        }

        // 3. Chrome
        self.show_titlebar(ctx);   // TopBottomPanel::top
        self.show_statusbar(ctx);  // TopBottomPanel::bottom

        // 4. Side panels (left = sidebar, right = right rail if open)
        self.show_sidebar(ctx);
        if self.right_rail_open {
            self.show_right_rail(ctx);
        }

        // 5. Central panel — dispatch on overlay state
        match self.central_overlay {
            CentralOverlay::None => self.show_editor(ctx),
            CentralOverlay::Tickets => egui::CentralPanel::default()
                .show(ctx, |ui| self.show_tickets_panel(ui)),
            CentralOverlay::TagExplorer => egui::CentralPanel::default()
                .show(ctx, |ui| self.show_tag_explorer(ui)),
            CentralOverlay::Timeline => egui::CentralPanel::default()
                .show(ctx, |ui| self.show_timeline_view(ui)),
            CentralOverlay::Onboarding => self.show_onboarding(ctx),
        }

        // 6. Floating layers (z-order: bottom → top)
        self.show_toasts(ctx);
        self.show_hover_preview(ctx);
        self.show_calendar(ctx);
        self.show_modals(ctx);             // existing — new, settings, confirm, import, external, diff
        self.show_quick_capture(ctx);
        self.show_dictation_overlay(ctx);
        self.show_command_palette(ctx);    // last so it's on top
        self.show_error_modal(ctx);        // existing error_msg banner
    }
}
```

### 5.5 Tab strip implementation note

For v1.2 ship-with-single-tab approach:
- `tabs` always has 0 or 1 entries (synced to `active_note.id`).
- Tab strip renders that one tab (or nothing) — chrome consistent with multi-tab future.
- `Ctrl+T` does nothing yet; `Ctrl+W` deselects active note.

For v1.3 multi-tab:
- `select_note(id)` checks `tabs.contains(id)`; if yes, just set `tab_active`. If no, push and activate.
- `palette ⌘↵` always pushes new tab.
- Middle-click sidebar row → push tab without switching.

### 5.6 Chat session module

`omninote-core` exposes `LlmProvider` trait + impls. `omninote-gui::ui_chat.rs` calls into it. Async strategy:

- Single tokio runtime in `OmniNoteApp::new()` (`tokio::runtime::Runtime::new()`, stored in `self.rt: Arc<Runtime>`).
- Chat send: spawn task on `rt`; task does HTTP call; on completion, sends `ChatMessage` via `mpsc::channel`.
- Each frame: drain channel into `chat_session.messages`. `chat_in_flight = false` when message arrives.

### 5.7 Watcher integration

Existing v0.6 watcher (`VaultWatcher::drain_md_changes`) already works. New:
- When watcher fires on a NOT-active note → push `Toast::info("Note <name> reloaded")`.
- When fires on active + clean → reload + toast.
- When fires on active + dirty → existing conflict modal (extended with diff button).

### 5.8 Theming + accent color picker

```rust
// theme.rs

pub struct Theme {
    pub bg: Color32, pub chrome: Color32, pub panel: Color32, pub panel2: Color32,
    pub inset: Color32, pub border: Color32, pub border_strong: Color32, pub divider: Color32,
    pub text: Color32, pub text_strong: Color32, pub dim: Color32, pub dimmer: Color32,
    pub accent: Color32, pub accent_dim: Color32, pub accent_bg: Color32, pub accent_bg_strong: Color32,
    pub green: Color32, pub amber: Color32, pub red: Color32, pub blue: Color32, pub pink: Color32,
}

impl Theme {
    pub fn obsidian_dark() -> Self { /* values from §1.1 */ }
    pub fn obsidian_light() -> Self { /* inverted */ }
    pub fn high_contrast() -> Self { /* §1.8 */ }
    pub fn custom(base: ThemePreset, accent: [u8;3]) -> Self { /* override accent + derived */ }
}
```

`AppConfig.accent_color: [u8;3]` rebuilds `accent`, `accent_dim` (multiply 0.65), `accent_bg` (alpha 30), `accent_bg_strong` (alpha 51).

`Theme` is loaded once in `OmniNoteApp::new()`, stored as `self.theme: Theme`. Apply to egui via `ctx.set_visuals(theme_to_visuals(&self.theme))` plus selective per-widget styling in `show_*` methods (egui doesn't have full theming via `Visuals` alone — sidebar/panel backgrounds need explicit `Frame::default().fill(theme.panel)`).

### 5.9 Markdown render replacement

Existing: `egui_commonmark::CommonMarkViewer::new().show(ui, &mut self.md_cache, &note.content)`.

Replacement plan (Fase B):

```rust
// md_render.rs

pub fn render_markdown(ctx: RenderCtx, ui: &mut egui::Ui, content: &str) {
    use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
    let parser = Parser::new(content);
    let mut state = RenderState::default();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => state.push_heading(level),
            Event::End(TagEnd::Heading(_)) => state.flush_heading(ui, ctx),
            Event::Text(text) => render_text_with_wikilinks(ui, ctx, text.as_ref(), &mut state),
            Event::Code(inline) => render_inline_code(ui, ctx, inline.as_ref()),
            Event::Start(Tag::CodeBlock(_)) => state.in_code_block = true,
            // ... full event handling
        }
    }
}

fn render_text_with_wikilinks(ui: &mut egui::Ui, ctx: RenderCtx, text: &str, state: &mut RenderState) {
    // Find [[...]], ![[...]], #tag inside `text` (skip if in code block)
    // Emit egui spans/widgets for each segment
    // For [[wikilink]] → ui.link(...)
    //                    On hover → set hover_preview
    //                    On click → ctx.navigate_to(...)
    // For ![[Note]] → embed card widget
    // For ![[image]] → egui::Image widget
    // For #tag → chip widget
}
```

`RenderCtx` carries `vault: &Vault`, `theme: &Theme`, and navigation callbacks (`navigate_to: &mut dyn FnMut(&str)`).

This is the biggest hunk of new code in Fase B. CAD-20 owns the parser side (`WikilinkRef` grammar); CAD-25 owns rendering it. Coordinate so CAD-20's parser API is compatible with how the renderer walks events.

### 5.10 Performance notes

- **Sidebar render:** 280 px wide. For 1000-note vaults, the tree could be slow. Cache `list_folders()` result; invalidate on `reload_notes()`.
- **Backlinks computation:** full vault scan. Cache in `backlinks_cache`, keyed on active note id. Invalidate on `reload_notes()`.
- **Outline:** scan current note for `^#+\s` lines. Cheap.
- **Command palette:** fuzzy-match across all notes' titles + N first words. Pre-build a `Vec<(id, indexable_text)>` on vault load.
- **Markdown render:** custom renderer must be O(n) over content. No per-frame regex.
- **AI Chat:** all LLM/embedding calls async via tokio. UI never blocks.

### 5.11 Testing strategy

Continue `#[cfg(test)]` inline pattern (SPRINT §0 rule 10). Per-module:

- `theme.rs` — unit tests for color derivations + preset toggles.
- `ui_palette.rs` — pure fuzzy-match scorer tested with proptest.
- `md_render.rs` — golden tests: given content → expected sequence of widget ops (use a fake `RenderCtx` that captures calls).
- `ui_discipline.rs` — SPRINT parser tested with real `discipline/SPRINT.md` samples.
- `actions.rs` — handler tests (CAD-12 pattern continued).

For the truly UI-render side, leverage `egui_kittest` when egui 0.30+ is adopted (deferred per existing HUMAN.md Q-06).

---

## 6. Keyboard shortcut table (consolidated)

All shortcuts are configurable in Settings → Atalhos (§3.14). Defaults below. Conflicts with reserved system shortcuts (Cmd-Q, Cmd-W on macOS) blocked at capture time.

### 6.1 Global

| Combo (mac / linux+win) | Action | Surface |
|---|---|---|
| `⌘N` / `Ctrl+N` | New note (modal) | Any |
| `⌘P` / `Ctrl+P` | Command palette | Any |
| `⌘K` / `Ctrl+K` | Focus sidebar search | Any |
| `⌘,` / `Ctrl+,` | Settings | Any |
| `⌘⇧D` / `Ctrl+Shift+D` | Toggle dark/light theme | Any |
| `⌘⇧Space` / `Ctrl+Shift+Space` | Quick capture popup | Any (global via daemon — CAD-24) |
| `⌘⇧M` / `Ctrl+Shift+M` | Toggle dictation | Any |
| `⌘\` / `Ctrl+\` | Toggle right rail | Any |
| `⌘⇧J` / `Ctrl+Shift+J` | Tickets panel | Any |
| `⌘⇧T` / `Ctrl+Shift+T` | Tag explorer | Any |
| `⌘⇧H` / `Ctrl+Shift+H` | Timeline view | Any |
| `⌘⇧.` / `Ctrl+Shift+.` | Toggle sidebar | Any |

### 6.2 Editor

| Combo | Action |
|---|---|
| `⌘E` / `Ctrl+E` | Toggle view/edit |
| `⌘=` / `Ctrl+=` | Math eval current line (existing v0.1) |
| `⌘L` / `Ctrl+L` | Insert wikilink picker (lightweight palette) |
| `⌘T` / `Ctrl+T` | New tab from template (v1.3) |
| `⌘W` / `Ctrl+W` | Close current tab |
| `⌘[` / `Ctrl+[` | Previous tab |
| `⌘]` / `Ctrl+]` | Next tab |
| `/` (start of line) | Slash menu |
| `[[` | Wikilink picker (auto-trigger inside textedit) |
| `Esc` | Close slash menu / hover preview / popup |

### 6.3 Right rail

| Combo | Action |
|---|---|
| `⌘1` / `Ctrl+1` | Backlinks tab |
| `⌘2` / `Ctrl+2` | Outline tab |
| `⌘3` / `Ctrl+3` | AI Chat tab |
| `⌘↵` / `Ctrl+Enter` | Send chat message |

### 6.4 Command palette

| Combo | Action |
|---|---|
| `↑ ↓` | Navigate results |
| `Tab` | Cycle scope chips |
| `↵` | Open result |
| `⌘↵` / `Ctrl+Enter` | Open in new tab |
| `Esc` | Close |

### 6.5 Daily / calendar

| Combo | Action |
|---|---|
| `⌘D` / `Ctrl+D` | Open today's daily |
| `⌘⇧→` / `Ctrl+Shift+→` | Next day's daily (creates if missing) |
| `⌘⇧←` / `Ctrl+Shift+←` | Previous day's daily |

### 6.6 Discipline files

| Combo | Action |
|---|---|
| (none default — accessible via palette) | Open SPRINT.md, DIARY.md, etc |

In Settings, user can bind any specific file to a shortcut.

### 6.7 Quick capture popup

| Combo | Action |
|---|---|
| `↵` | Capture |
| `⇧↵` | New line within capture |
| `Esc` | Cancel |
| `Tab` | Cycle type chips |

### 6.8 Dictation overlay

| Combo | Action |
|---|---|
| `⌘⇧M` | Stop recording |
| `Esc` | Cancel (discard) |

---

## 7. CLI output style guide

`omninote-cli` (CAD-21, Sprint v1.1) ships verbs that mirror the GUI surfaces. The CLI is itself a UX surface — its output formatting is part of the OmniNote design language.

### 7.1 Output modes

Every verb supports two modes:

- **Human (default):** ANSI color, single-line table borders, friendly date formatting (`2m ago`, `Yesterday`, `Mon Oct 7`), kebab-case status.
- **JSON (`--json` flag):** structured envelope, no ANSI, ISO-8601 dates, machine-friendly.

Auto-detect: if stdout is not a TTY (detected via `is_terminal::is_terminal(std::io::stdout())`), ANSI is suppressed even in human mode; JSON is NOT auto-selected — the user must opt in via `--json`.

### 7.2 ANSI palette (mirrors GUI theme)

| Role | Color | Use |
|---|---|---|
| accent (violet) | `38;5;141` | Headings, active row, verb name |
| green | `38;5;78` | Ok status, "saved", done counts |
| amber | `38;5;179` | In-progress, warning |
| red | `38;5;167` | Errors, broken links |
| blue | `38;5;75` | NoteType resumo, JIRA |
| pink | `38;5;176` | NoteType citacao, HUMAN |
| dim | `38;5;245` | Secondary text, paths |
| dimmer | `38;5;240` | Tertiary, ruler glyphs |

For pure 256-color terminals. For 24-bit terminals, use the hex tokens from §1.1 directly.

### 7.3 Human output formats per verb

#### `omninote vault info`

```
~/Projects/caderno   ClaudeBook
─────────────────────────────────────────
notes        142
folders       18
attachments   24    3.2MB
disciplines    6    SPRINT, DIARY, JIRA, HUMAN, PLAN, NOTION
last edit    2m ago   2026-05-20.md
```

#### `omninote note search "query"`

```
3 results for "motoboy" in 18ms

▤  Motoboys module                    Specs                  2m ago
   "...registra a entrega digitando o número do pedido e selecionando..."
   #motoboy #cfo-pocket #spec

⌘  iFood Linking                      cfo-pocket             4h ago
   "...vincula o pedido do motoboy ao pedido do iFood quando..."
   #integration #cfo-pocket

▤  Motoboy fee tiers                  Drafts                 2d ago
   "...estrutura de tiers definida pela Ju, ver Fee Tier Matrix..."
   #draft #motoboy
```

#### `omninote note new --type resumo --title "Test"`

```
✓ Created Resumo  Test
  ~/Projects/caderno/Test.md
  id: n_c2f...
```

#### `omninote link unresolved`

```
2 unresolved wikilinks across 142 notes

✗  [[Fee Tier Matrix]]
   referenced in:
     • 2026-05-20.md             (1 occurrence)
     • Motoboys module.md        (3 occurrences)
     • SPRINT.md                 (1 occurrence)

✗  [[OWASP-2021 checklist]]
   referenced in:
     • 2026-05-20.md             (1 occurrence)

Run  omninote note new --title "Fee Tier Matrix"  to resolve.
```

#### `omninote backlinks "Motoboys module"`

```
3 incoming links to "Motoboys module"

▤  2026-05-19.md             "...ver [[Motoboys module]] pra spec..."   1d ago
⌘  iFood Linking             "...depende de [[Motoboys module]]..."      4h ago
§  SPEC_V2 - NdA             "...conforme [[Motoboys module]] §3..."     6d ago
```

#### `omninote daily`

```
✓ Daily/2026-05-20.md  (created from Templates/daily.md)
  open: ~/Projects/caderno/Daily/2026-05-20.md
```

If exists:
```
↻ Daily/2026-05-20.md  (already exists, opening)
```

#### `omninote ask "where did I discuss escrow HMAC?"`

```
✦ claude-sonnet-4.5 · vault: ClaudeBook · 1.2s

Top 3 passages:

  ▤ SPEC_V2 - NdA       §3.2 Escrow HMAC handshake
    "O handshake usa HMAC-SHA256 com chave compartilhada..."
    relevance 0.91

  ⌘ p2p-desk/SPEC.md    §5.1 Cryptographic primitives
    "Escrow utiliza HMAC para validar a chave de sessão..."
    relevance 0.87

  ▤ HUMAN.md            Q-04: HMAC vs Ed25519
    "Resposta: HMAC suficiente, escrow não precisa de..."
    relevance 0.74

Synthesis:
  A discussão sobre escrow HMAC está concentrada em SPEC_V2 §3.2
  (handshake) e p2p-desk SPEC §5.1 (primitives). Q-04 do HUMAN
  registra a decisão de manter HMAC ao invés de Ed25519.
```

#### `omninote tag --auto path/to/note.md`

```
Reading: 2026-05-20.md (412 words)
✦ claude-sonnet-4.5 · 0.9s

Current frontmatter:
  tags: [#sprint, #daily, #cfo-pocket]

Suggested update:
  tags: [#sprint, #daily, #cfo-pocket, #motoboy, #owasp, #review]

  + #motoboy   (new — 4 references in body)
  + #owasp     (new — 1 reference + audit task)
  + #review    (new — feature in review state)

Apply? [y/N]
```

#### `omninote diff --since 1d`

```
~/Projects/caderno   since 2026-05-19 14:32

2026-05-20  Wed
  14:32  ✎ DIARY.md                            +47 lines
  14:08  ✎ SPRINT.md                           +5 lines, -3 lines
  11:42  ⊕ SPECS/CAD-25.md                     +52 lines (created)

2026-05-19  Tue
  23:14  ✎ NOTION.md                           +12 lines
  23:14  ✎ JIRA.md                             +8 lines
  17:08  ⊝ Notes/old-spec.md                   -412 lines (deleted)

6 changes total across 5 files.
```

#### `omninote capture "remember to ask Ju about fee tiers"`

```
✓ Inbox.md  (+1 line)
  - 2026-05-20 14:32 · ▤ remember to ask Ju about fee tiers
```

#### Errors

```
✗ Vault not found: /no/such/path
  Hint: pass --vault <PATH> or set OMNINOTE_VAULT
  Exit code: 3
```

```
✗ LLM provider not configured.
  Edit ~/.config/omninote/llm.toml or run omninote settings llm
  Exit code: 4
```

### 7.4 JSON output (envelope shape)

Every verb in `--json` mode emits a single JSON document on stdout:

```json
{
  "ok": true,
  "data": { ... },
  "error": null,
  "meta": {
    "verb": "note.search",
    "vault": "/Users/.../caderno",
    "duration_ms": 18,
    "version": "1.2.0"
  }
}
```

On error:
```json
{
  "ok": false,
  "data": null,
  "error": {
    "code": "vault_not_found",
    "message": "Vault not found: /no/such/path",
    "hint": "pass --vault <PATH> or set OMNINOTE_VAULT"
  },
  "meta": { "verb": "vault.info", "vault": null, "version": "1.2.0" }
}
```

Exit codes (same in both modes):
- `0` — success
- `1` — user error (bad args, file not found)
- `2` — internal error (bug)
- `3` — vault not found / not opened
- `4` — LLM provider error (not configured, API failure)
- `5` — git error (timeline / diff)

### 7.5 JSON `data` shapes per verb (excerpt — see §8 MCP for full table)

#### `vault.info`
```json
{
  "root": "/Users/.../caderno",
  "name": "ClaudeBook",
  "notes_count": 142,
  "folders_count": 18,
  "attachments_count": 24,
  "attachments_bytes": 3355443,
  "discipline_files": ["SPRINT.md", "DIARY.md", "JIRA.md", "HUMAN.md", "PLAN.md", "NOTION.md"],
  "last_edit": {
    "path": "Daily/2026-05-20.md",
    "title": "2026-05-20.md",
    "modified_at": "2026-05-20T14:32:00Z",
    "minutes_ago": 2
  }
}
```

#### `note.search`
```json
{
  "query": "motoboy",
  "results": [
    {
      "id": "n_abc...",
      "title": "Motoboys module",
      "path": "Specs/Motoboys module.md",
      "note_type": "resumo",
      "excerpt": "...registra a entrega digitando o número do pedido e selecionando...",
      "tags": ["motoboy", "cfo-pocket", "spec"],
      "modified_at": "2026-05-20T14:30:00Z",
      "score": 0.92
    }
  ],
  "total": 3,
  "duration_ms": 18
}
```

#### `link.unresolved`
```json
{
  "unresolved": [
    {
      "target": "Fee Tier Matrix",
      "occurrences": [
        { "path": "Daily/2026-05-20.md", "count": 1, "lines": [12] },
        { "path": "Specs/Motoboys module.md", "count": 3, "lines": [44, 88, 102] },
        { "path": "SPRINT.md", "count": 1, "lines": [56] }
      ]
    }
  ],
  "total_unresolved": 2,
  "total_occurrences": 6
}
```

#### `daily`
```json
{
  "path": "Daily/2026-05-20.md",
  "id": "n_def...",
  "created": true,
  "template_applied": "Templates/daily.md"
}
```

#### `ask`
```json
{
  "query": "where did I discuss escrow HMAC?",
  "provider": "claude",
  "model": "claude-sonnet-4.5",
  "passages": [
    {
      "note_id": "n_xyz...",
      "title": "SPEC_V2 - NdA",
      "heading": "§3.2 Escrow HMAC handshake",
      "excerpt": "O handshake usa HMAC-SHA256 com chave compartilhada...",
      "relevance": 0.91,
      "wikilink": "[[SPEC_V2 - NdA#3.2 Escrow HMAC handshake]]"
    }
  ],
  "synthesis": "A discussão sobre escrow HMAC está concentrada em...",
  "duration_ms": 1200
}
```

#### `diff`
```json
{
  "since": "2026-05-19T14:32:00Z",
  "until": "2026-05-20T14:32:00Z",
  "changes": [
    {
      "timestamp": "2026-05-20T14:32:00Z",
      "kind": "edit",
      "path": "DIARY.md",
      "added": 47,
      "removed": 0,
      "commit_hash": "abc123"
    }
  ],
  "total_changes": 6,
  "files_changed": 5
}
```

### 7.6 Pipelining recipes

Document in a new `docs/CLI_RECIPES.md` (CAD-24 scope). Examples:

```bash
# All unresolved wikilinks as a flat list
omninote link unresolved --json | jq -r '.data.unresolved[].target'

# Today's new notes
omninote diff --since 1d --json | jq -r '.data.changes[] | select(.kind=="create") | .path'

# Ask + extract just synthesis
omninote ask "summarize last week" --json | jq -r '.data.synthesis'

# Bulk auto-tag changed files
omninote diff --since 1d --json \
  | jq -r '.data.changes[] | select(.kind=="edit") | .path' \
  | xargs -I{} omninote tag --auto {} --yes
```

---

## 8. MCP tool registry

`omninote-mcp` (CAD-21) exposes verbs as MCP tools. Each tool declaration follows the MCP spec:

- `name` — kebab-case-namespaced (e.g. `vault_info`, `note_search`, `link_unresolved`).
- `description` — 1-line summary + when-to-use + side-effects + return shape. Read by the LLM at tool-call time, so must be precise.
- `inputSchema` — JSON Schema for parameters.
- Returns the same JSON envelope as `--json` CLI (`{ok, data, error, meta}`).

### 8.1 Tool table (full registry)

| Tool name | Side effects | When LLM should call |
|---|---|---|
| `vault_info` | none | At start of session to confirm vault context |
| `vault_list` | none | To discover other configured vaults |
| `vault_switch` | changes active vault for session | Only when user explicitly names another vault |
| `note_search` | none | To find notes by keyword / title |
| `note_open` | none (returns content) | To read a specific note's full content |
| `note_new` | creates `.md` file | To author a new note when user requests it |
| `note_append` | mutates `.md` | To append content to an existing note |
| `note_replace` | mutates `.md` | To rewrite a specific section (heading-scoped) |
| `note_rename` | renames `.md` | When user asks to rename a note |
| `note_delete` | deletes `.md` | Only with explicit user confirmation |
| `link_unresolved` | none | To audit vault for broken wikilinks |
| `link_backlinks` | none | To find what references a given note |
| `tag_list` | none | To enumerate all tags with counts |
| `tag_auto` | mutates frontmatter | To suggest tags for a note (LLM round-trip) |
| `daily_create_today` | creates `.md` in `Daily/` | At session start to ensure daily exists |
| `daily_open` | returns path | To navigate to a specific daily |
| `template_list` | none | Discover available templates |
| `template_apply` | mutates target note | To apply a template to an existing or new note |
| `discipline_sprint_show` | none | Read SPRINT.md structured |
| `discipline_sprint_append_task` | mutates SPRINT.md | Add new task with status/owner/points |
| `discipline_diary_append` | mutates DIARY.md | Append session log entry |
| `discipline_human_ask` | mutates HUMAN.md | Append open question to human |
| `discipline_human_resolve` | mutates HUMAN.md | Mark a question resolved with answer |
| `discipline_plan_show` | none | Read PLAN.md structured |
| `discipline_plan_append` | mutates PLAN.md | Append new plan block |
| `ticket_list` | none | Read NOTION.md + JIRA.md merged |
| `ticket_status` | mutates JIRA.md / NOTION.md row | Update ticket status locally |
| `ask` (semantic) | none (RAG lookup) | Answer questions over the vault corpus |
| `dictate_transcribe` | creates `.md` from audio | When user provides audio path |
| `ocr_pdf` | creates `<file>.ocr.md` | When user has scanned PDF |
| `diff` | none | Show recent vault changes |
| `capture` | mutates Inbox.md | Quick-capture a single line |

### 8.2 Detailed tool entries (excerpt)

#### `vault_info`
```json
{
  "name": "vault_info",
  "description": "Read-only. Returns metadata about the currently active OmniNote vault: root path, vault display name, total counts of notes/folders/attachments, list of discipline files present, and the most recently edited note. Call this at the start of a session to ground yourself in the vault state, or whenever you need to verify which vault is active. No side effects. Returns {ok, data: { root, name, notes_count, folders_count, attachments_count, attachments_bytes, discipline_files, last_edit }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "vault": {
        "type": "string",
        "description": "Optional path to a vault. If omitted, uses the currently active vault."
      }
    },
    "additionalProperties": false
  }
}
```

#### `note_search`
```json
{
  "name": "note_search",
  "description": "Read-only. Full-text search across all .md files in the vault. Matches against note titles and content body. Returns results ranked by relevance with excerpts. No side effects. Use to find notes by keyword, topic, or partial title before reading them in full. Returns {ok, data: { query, results: [{ id, title, path, note_type, excerpt, tags, modified_at, score }], total, duration_ms }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {"type": "string", "description": "Search query — substring or fuzzy match."},
      "type_filter": {
        "type": "string",
        "enum": ["resumo", "citacao", "codigo", "exercicio", "duvida", "definicao"],
        "description": "Optional: restrict to one NoteType."
      },
      "limit": {"type": "integer", "default": 20, "minimum": 1, "maximum": 100},
      "vault": {"type": "string", "description": "Optional vault path override."}
    },
    "required": ["query"],
    "additionalProperties": false
  }
}
```

#### `note_new`
```json
{
  "name": "note_new",
  "description": "Mutates the vault: creates a new .md file with YAML frontmatter. The note is placed in the specified folder (or vault root if omitted). On success returns the new note's id and path. Use when the user explicitly asks to create a note, or when the workflow requires a new note (e.g. resolving an unresolved wikilink). Side effect: writes one file to disk. Returns {ok, data: { id, path, title, note_type, frontmatter }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "title": {"type": "string", "description": "Note title (will become filename, sanitized)."},
      "type": {
        "type": "string",
        "enum": ["resumo", "citacao", "codigo", "exercicio", "duvida", "definicao"],
        "default": "resumo"
      },
      "folder": {"type": "string", "description": "Relative folder path within the vault. Omit for root."},
      "content": {"type": "string", "description": "Initial markdown content. Optional — empty body if omitted."},
      "tags": {"type": "array", "items": {"type": "string"}},
      "vault": {"type": "string", "description": "Optional vault path override."}
    },
    "required": ["title"],
    "additionalProperties": false
  }
}
```

#### `note_open`
```json
{
  "name": "note_open",
  "description": "Read-only. Returns the full content (frontmatter parsed + raw body markdown) of a single note identified by its id or by its title (case-insensitive). Use after note_search to fetch full content before reading or summarizing. No side effects. Returns {ok, data: { id, title, path, note_type, frontmatter, content, backlinks_count, wikilinks: [...] }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string", "description": "Note id (n_xxx). Mutually exclusive with title."},
      "title": {"type": "string", "description": "Note title (case-insensitive). Mutually exclusive with id."},
      "vault": {"type": "string", "description": "Optional vault path override."}
    },
    "additionalProperties": false
  }
}
```

#### `note_append`
```json
{
  "name": "note_append",
  "description": "Mutates an existing note by appending content to the end of its body, after a blank line. Frontmatter is preserved untouched. Use to add a new section, todo, or paragraph to a known note. Side effect: rewrites one .md file. Returns {ok, data: { id, path, lines_added }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "title": {"type": "string"},
      "content": {"type": "string", "description": "Markdown to append. A blank line is inserted before."},
      "vault": {"type": "string"}
    },
    "required": ["content"],
    "additionalProperties": false
  }
}
```

#### `link_unresolved`
```json
{
  "name": "link_unresolved",
  "description": "Read-only. Scans the entire vault for [[wikilinks]] whose target cannot be resolved by the resolver (filename, path, frontmatter alias, case-insensitive). Returns the list with their occurrence counts per source note. Use to audit vault health, or before suggesting a refactor. No side effects. Returns {ok, data: { unresolved: [{ target, occurrences: [{ path, count, lines }] }], total_unresolved, total_occurrences }}.",
  "inputSchema": {
    "type": "object",
    "properties": { "vault": {"type": "string"} },
    "additionalProperties": false
  }
}
```

#### `link_backlinks`
```json
{
  "name": "link_backlinks",
  "description": "Read-only. Returns all notes that reference a given target note, either by exact [[Title]] wikilink in their body or by frontmatter linked_note pointer. Each backlink includes a 90-char excerpt around the reference. Use to understand what depends on a note before editing or deleting it. No side effects. Returns {ok, data: { target, backlinks: [{ source_id, title, path, excerpt, occurrences: [{ line, kind }] }] }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "target": {"type": "string", "description": "Note title to look up backlinks for."},
      "vault": {"type": "string"}
    },
    "required": ["target"],
    "additionalProperties": false
  }
}
```

#### `daily_create_today`
```json
{
  "name": "daily_create_today",
  "description": "Mutates the vault if today's daily note does not exist: creates Daily/YYYY-MM-DD.md from Templates/daily.md (or a stub if no template). Idempotent — returns the existing file if already present. Use at session start to ensure today's daily exists before appending diary or session notes. Side effect: may write one .md file. Returns {ok, data: { path, id, created: bool, template_applied: string|null }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "date": {"type": "string", "format": "date", "description": "Optional: target a specific date instead of today. Format YYYY-MM-DD."},
      "vault": {"type": "string"}
    },
    "additionalProperties": false
  }
}
```

#### `discipline_diary_append`
```json
{
  "name": "discipline_diary_append",
  "description": "Mutates DIARY.md by appending a new entry under today's date heading (creates the heading if missing). Entry format follows the project's discipline convention: HH:MM timestamp + [label] chip + body. Use at the end of each working session to log what was done. Side effect: rewrites DIARY.md. Returns {ok, data: { path, entry_id, total_entries }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "body": {"type": "string", "description": "The entry body — short paragraph or bullet list."},
      "label": {"type": "string", "description": "Optional label chip like 'CAD-25 fase a', 'pre-merge-coverage', 'plan-mode-bypass'."},
      "vault": {"type": "string"}
    },
    "required": ["body"],
    "additionalProperties": false
  }
}
```

#### `discipline_human_ask`
```json
{
  "name": "discipline_human_ask",
  "description": "Mutates HUMAN.md by appending a new open question. Question is written in pt-BR per discipline rule. Use ONLY when a decision is genuinely high-stakes (irreversibility, contract, security, SPRINT conflict, design ambiguity that persists in code). Style/reversible choices should NOT use this — agent decides and logs to DIARY instead. Side effect: rewrites HUMAN.md. Returns {ok, data: { path, question_id, total_open }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "question": {"type": "string", "description": "Question in pt-BR. Lay out options and tradeoffs explicitly."},
      "category": {"type": "string", "enum": ["design", "security", "contract", "irreversibility", "sprint-conflict", "other"]},
      "vault": {"type": "string"}
    },
    "required": ["question"],
    "additionalProperties": false
  }
}
```

#### `ticket_list`
```json
{
  "name": "ticket_list",
  "description": "Read-only. Returns merged ticket index from NOTION.md and JIRA.md. Each ticket has provider, id, title, status, sprint, owner, points, tags. Filter by status if needed. Use to check what's in flight before starting work. No side effects. Returns {ok, data: { tickets: [{ provider, id, title, status, sprint, owner, points, tags, url, local_spec_path }], by_status: { in_progress: N, todo: N, done: N, blocked: N } }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "status": {"type": "string", "enum": ["in_progress", "todo", "done", "blocked"]},
      "provider": {"type": "string", "enum": ["notion", "jira"]},
      "vault": {"type": "string"}
    },
    "additionalProperties": false
  }
}
```

#### `ask` (semantic / RAG)
```json
{
  "name": "ask",
  "description": "Read-only. Runs a semantic (embedding-based) search over the vault: embeds the query with the configured local model, retrieves top-K passages, and synthesizes an answer with the configured LLM. Each cited passage is returned with a [[wikilink]] for navigation. Use for questions like 'where did I discuss X?' or 'summarize the decisions about Y'. No side effects on the vault, but may incur an LLM API call cost. Returns {ok, data: { query, provider, model, passages: [{ note_id, title, heading, excerpt, relevance, wikilink }], synthesis, duration_ms }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "query": {"type": "string"},
      "top_k": {"type": "integer", "default": 5, "minimum": 1, "maximum": 20},
      "synthesize": {"type": "boolean", "default": true, "description": "If false, returns only passages without LLM synthesis (faster, cheaper)."},
      "vault": {"type": "string"}
    },
    "required": ["query"],
    "additionalProperties": false
  }
}
```

#### `tag_auto`
```json
{
  "name": "tag_auto",
  "description": "Mutates the target note's frontmatter by adding LLM-suggested tags. Does NOT remove existing tags. Returns the diff so the user/caller can confirm before applying — pass dry_run=false to actually write. Use to enrich tag coverage on notes the user has flagged. Side effect (when dry_run=false): rewrites one .md file. Returns {ok, data: { id, path, current_tags, suggested_tags, added: [...], dry_run }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "id": {"type": "string"},
      "title": {"type": "string"},
      "dry_run": {"type": "boolean", "default": true},
      "vault": {"type": "string"}
    },
    "additionalProperties": false
  }
}
```

#### `capture`
```json
{
  "name": "capture",
  "description": "Mutates Inbox.md by appending a single bullet line with timestamp and type glyph. Use to quickly stash a thought, todo, or reference for later processing. Faster than creating a full note. Side effect: rewrites Inbox.md. Returns {ok, data: { path, line_appended, total_lines }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "text": {"type": "string"},
      "type": {
        "type": "string",
        "enum": ["resumo", "citacao", "codigo", "exercicio", "duvida", "definicao"],
        "default": "resumo"
      },
      "vault": {"type": "string"}
    },
    "required": ["text"],
    "additionalProperties": false
  }
}
```

#### `diff`
```json
{
  "name": "diff",
  "description": "Read-only. Returns git-aware snapshot diff of the vault: list of file changes within a time window. Requires the vault to be a git repo (returns ok=false with code=git_unavailable otherwise). Use to summarize recent work or to power a 'what changed' overview. No side effects. Returns {ok, data: { since, until, changes: [{ timestamp, kind, path, added, removed, commit_hash }], total_changes, files_changed }}.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "since": {"type": "string", "description": "Time window: '1d', '7d', '1mo', or ISO-8601 timestamp."},
      "paths": {"type": "array", "items": {"type": "string"}, "description": "Optional path filters."},
      "vault": {"type": "string"}
    },
    "additionalProperties": false
  }
}
```

### 8.3 Error codes (canonical)

| Code | Description | HTTP-equivalent intent |
|---|---|---|
| `vault_not_found` | Path doesn't exist or isn't a vault | 404 |
| `note_not_found` | id/title not in vault | 404 |
| `note_already_exists` | Filename collision on create/rename | 409 |
| `llm_not_configured` | `llm.toml` missing or `provider=Disabled` | 412 |
| `llm_api_error` | Upstream LLM call failed | 502 |
| `git_unavailable` | Vault is not a git repo | 412 |
| `validation_error` | Invalid input args | 400 |
| `internal_error` | Bug — please report | 500 |

### 8.4 MCP server packaging

```json
// Claude Desktop config snippet (after CAD-21 ships)
{
  "mcpServers": {
    "omninote": {
      "command": "/Users/peluche/.cargo/bin/omninote-mcp",
      "args": ["--vault", "/Users/peluche/Projects/caderno"]
    }
  }
}
```

Stdout = JSON-RPC framing per MCP spec. Stderr = logs (timestamped, color in TTY, plain in pipe). Logs respect `OMNINOTE_LOG=trace|debug|info|warn|error`.

---

## 9. Open questions for Fausto

These are decisions or ambiguities that surfaced while writing the design plan. Per discipline rule #9 (and to keep Fase B unblocked), they're flagged here so Fausto can decide in batch before implementation.

### Q-01 — Tabs in v1.2 or v1.3?
The mockup shows a 3-tab editor strip. Multi-tab is a meaningful state addition (`tabs: Vec<NoteId>`, persistence, drag-reorder, Ctrl+T/W/[/] keys). I propose **ship single-tab in v1.2** with the chrome rendered (so the UI is consistent) but Ctrl+T no-op. Multi-tab proper lands in v1.3 alongside the AI-native vault work. Confirm or override?

### Q-02 — Custom markdown renderer (pulldown-cmark) in v1.2?
Inline wikilink rendering with hover preview, inline `![[Note]]` embed cards, and inline `#tag` chips all require swapping `egui_commonmark::CommonMarkViewer` for a pulldown-cmark walker (SPEC.md §6 plan). That's the single largest chunk of new code in Fase B (~600 LOC + tests). Alternative: ship the right-rail/sidebar/palette/etc UI in v1.2 and keep `CommonMarkViewer` for body render, defer the custom renderer to v1.3. I lean toward **doing it now** because hover preview is one of the most-loved Obsidian features. Confirm?

### Q-03 — High-contrast preset shipping with v1.2?
CAD-25 constraint #4 demands a high-contrast preset. I sketched the palette (§1.8). Ship it ASAP or wait until an accessibility audit? I lean **ship now**, but it'll be visually rough (#000/#fff/#ff0 is brutal) and likely need iteration.

### Q-04 — Theme accent picker — color wheel or preset list?
Settings panel (§3.14) shows a color swatch + hex value for `accent_color`. Full color picker (HSV wheel + RGB sliders + hex input — egui has `egui::widgets::color_picker::color_picker_color32`) is ~30 LOC. Alternative: a preset list (violet/blue/green/amber/pink/red). I propose **full picker** since it's cheap. Confirm?

### Q-05 — Tab strip vs sidebar selection — what's the source of truth?
Currently `active_note` is the truth. In a multi-tab world, `tab_active: Option<usize>` indexes into `tabs`, and `active_note` is derived. Single-tab v1.2: keep `active_note` the truth, `tabs` is a 0-or-1 mirror. v1.3 multi-tab: invert — `tabs` is truth, `active_note` is `tabs.get(tab_active)`. Sound?

### Q-06 — Inbox.md format — bullet list or fenced sections?
§2.8 specifies `- YYYY-MM-DD HH:MM · <glyph> <text>` as the quick-capture line format. Obsidian + Logseq users tend to expect plain bullets. CFO Pocket discipline files use date-headed sections (`## 2026-05-20\n  14:32  ...`). I propose **plain bullets at top of file** (simpler, append-friendly, easy to triage). Section organization is a manual step at "Inbox processing" time. Confirm format?

### Q-07 — Quick-capture global hotkey in v1.2 GUI or v1.3 daemon?
Per CAD-24, global hotkey requires a separate `omninote-capture` binary (macOS NSEvent tap / Linux global-hotkey crate). v1.2 GUI alone can only register an **in-app** shortcut (Ctrl+Shift+Space when the app has focus). I propose **in-app only for v1.2**, daemon for v1.3 (CAD-24 phase 5). The UI is identical, just the trigger is in-app. OK?

### Q-08 — Templates folder location: `Templates/` at vault root or `.omninote/templates/`?
Obsidian convention: `Templates/` at vault root, plain `.md` files. CAD-22 ticket says `Templates/`. The cost: appears in sidebar Vault section unless suppressed. I propose **`Templates/` at root** (Obsidian-compat) and **explicitly suppress it from the Vault tree section** in the sidebar, render it instead as a sub-row under the Inbox section or as its own pinned entry under Disciplines. Confirm?

### Q-09 — Discipline-typed view edit affordances in v1.2?
§3.4 (SPRINT) and §3.5 (DIARY) specify typed views. v1.2 ships **read-only** typed views. Edit affordances (drag to reorder tasks, click checkbox to toggle status, "+ Append entry" dialog for DIARY) ship in v1.3 if there's demand. Alternative: ship at least the **DIARY append dialog** since it's the most-used write path. Lean toward **append dialog in v1.2, full edit in v1.3**. Confirm?

### Q-10 — AI Chat in v1.2 UI without the AI backend (CAD-23)?
The right-rail AI Chat tab needs the LLM backend (CAD-23 Sprint v1.3). Two options:
- **A:** ship the AI Chat tab in v1.2 with stub responses ("LLM not yet wired — see CAD-23"), so the UX flow is testable.
- **B:** hide the AI Chat tab in v1.2 entirely; right rail has only Backlinks + Outline tabs until CAD-23 lands.

I lean **B** (cleaner — no half-baked feature in front of the user). Backlinks + Outline alone are valuable enough to ship. Confirm?

### Q-11 — Toast queue position when sidebar is collapsed?
§2.15 anchors toasts bottom-right, 16 px from right edge. If sidebar/right-rail are toggled, the editor width changes — toasts always anchor to the screen edge, not the editor. Confirm that's the intended behavior (vs anchoring to the editor pane).

### Q-12 — Settings panel: modal or full-page takeover?
Existing settings is a `egui::Window` modal. With the new fields (§3.14), it's getting tall (~700 px). Two options:
- **A:** keep as modal, scrollable interior.
- **B:** promote to full-page takeover (CentralPanel) like Tickets/TagExplorer/Timeline, accessed via Ctrl+,.

I lean **A** (modal — settings is a focused detour, not part of the workspace). Confirm?

### Q-13 — Dictation UI in v1.2 without Whisper backend?
Same as Q-10 but for dictation. Whisper integration is CAD-23. v1.2 options:
- **A:** ship dictation UI with stub ("dictation not wired — see CAD-23"), creates an empty placeholder note.
- **B:** hide dictation UI entirely in v1.2 (no title-bar mic, no Ctrl+Shift+M binding).

Lean **B** for the same reason as Q-10. Confirm?

### Q-14 — Tickets panel pull/push affordances in v1.2?
§3.6 shows ⟲/⤴ icons for sync state. These require the Notion/Jira API integration. In v1.2 we can:
- **A:** render the icons as stubs ("Sync not wired — see CAD-22").
- **B:** hide them; show only the local merged view.

Lean **B**. Confirm?

### Q-15 — Onboarding flow — when does it trigger?
§3.15 onboarding triggers on "first vault open + empty vault". What about an existing-but-unfamiliar vault (user pointed OmniNote at their old Obsidian vault for the first time)? Should onboarding still show, maybe variant? Lean: **fire onboarding only when `vault.notes.is_empty()` AND `vault.config` has defaults**; for non-empty existing vaults, fire a 1-shot toast "Welcome to OmniNote — press ⌘P to explore". Confirm logic?

### Q-16 — Calendar widget date format — pt-BR or i18n later?
Sketch uses English day names ("Sun Mon Tue..."). Fausto's locale is pt-BR. Lean: **pt-BR by default** (Dom Seg Ter Qua Qui Sex Sáb), with `vault.config.locale` field for future i18n. Confirm?

### Q-17 — Frontmatter aliases storage — `Frontmatter.aliases: Vec<String>`?
CAD-20 needs the resolver to consult frontmatter aliases. Right now `Frontmatter` (in `src/types.rs`) has no `aliases` field. Add `pub aliases: Vec<String>` (`#[serde(default)]`). Confirm field name + type?

### Q-18 — Inline `#tag` parser — what counts?
Body-position `#word` becomes a clickable chip. What's the matching rule?
- **Strict:** `#[a-zA-Z][a-zA-Z0-9_/-]*` (Obsidian's rule)
- **Loose:** `#[^\s#]+`
- **Negative:** skip if inside code blocks (` `` `, fenced ```), inside wikilinks, inside URLs.

Lean **strict + negative**. Confirm? This affects both render and CLI search.

### Q-19 — Embed of code repo snapshots — `Projects/<repo>/`?
The vault model in CAD-25 mentions `Projects/<repo>/` as mirrored snapshots. Implementation is unspecified. Options:
- **A:** automatic — when vault is opened, scan `Projects/` for git repos and mirror their `.md` files into the vault on watcher events.
- **B:** manual — user copies/symlinks their code repos under `Projects/` themselves; OmniNote just renders them like any other folder.
- **C:** out-of-scope — drop the `Projects/` section in v1.2.

Lean **B** for v1.2 (zero implementation, just sidebar surface). Confirm?

### Q-20 — Daily section in sidebar: pinned forever or fade after N days?
§2.3 shows 3 dailies in the "Today" section. What's the rule?
- **A:** today + previous 2 days, always.
- **B:** today + last edited daily within 7 days.
- **C:** today only, plus "open calendar…" link.

Lean **A** (predictable). Confirm?

### Q-21 — Right-rail toggle persistence: per-vault or global?
`right_rail_open` could live in `AppConfig` (per-vault) or in `~/.config/omninote/global.toml` (cross-vault). Lean **per-vault** (different vaults have different shapes — a vault used for AI chat probably has the rail open; a draft vault doesn't). Confirm?

### Q-22 — Tab title format when title is empty / Untitled?
Edge case: new note with no title yet. Tab shows what? Lean: **"Untitled" + accent dot** (matches dirty indicator). Confirm?

### Q-23 — Hover preview latency — 500 ms vs 250 ms?
§3.2 specifies 500 ms hover delay. Obsidian uses ~300 ms. Faster = more responsive, but accidental triggers when scrolling. Lean **400 ms** as middle ground. Confirm?

### Q-24 — Markdown rendering of frontmatter block — show or hide by default?
In view mode the frontmatter shows as a styled callout (§2.4 mockup). Some users want it always hidden, only visible in edit mode. Lean: **collapsed by default in view mode, click-to-expand**. Confirm?

### Q-25 — Embed card "open full →" link — open in same tab or new?
Click an embed card → navigate to embedded note. Does that replace the current note view (single-tab v1.2) or open in new tab (v1.3)? In v1.2 it replaces. In v1.3 the affordance might be "click = same tab, ⌘-click = new tab". Confirm v1.2 single-tab behavior?

### Q-26 — Snapshot of OmniNote v1.2 dependencies bump?
Custom markdown render (Q-02) needs `pulldown-cmark` (~50 KB). Fuzzy match needs `fuzzy-matcher` (~30 KB). Git timeline needs `git2` (~2 MB shared lib). Async LLM calls need `tokio` (~1 MB). Total binary growth ~3 MB. From a 10 MB → 13 MB binary. Acceptable for v1.2 release? Or defer git timeline (Q-26b) to v1.3?

### Q-27 — Embedded fonts vs system fonts?
§1.5 plan embeds `Inter` + `JetBrains Mono`. Embed is +700 KB binary, but ensures consistent look across OSes. Alternative: rely on system fonts (`-apple-system` / `Segoe UI` / `Roboto`). Lean **embed** for v1.2 (visual consistency matters for the Obsidian-class look). Confirm?

### Q-28 — `_attachments/` flat vs subfolder per attachment type?
Existing v1.0 keeps `_attachments/` flat. Some users want `_attachments/images/`, `_attachments/pdfs/`, etc. Lean: **keep flat for v1.2** (less migration churn for existing vaults, simpler embedding). Confirm?

### Q-29 — Save-on-blur for sidebar vault switcher?
Click another vault in switcher → flush_active current note? Lean **yes** (current dirty changes persist before swap). Confirm?

### Q-30 — Accent color: 1 token or per-NoteType override?
§1.2 maps NoteType to fixed semantic colors (blue/pink/green/amber/red/violet). The accent token in §1.1 is the brand violet. What if user picks accent = green via picker (Q-04)? Two interpretations:
- **A:** accent affects only chrome (active row, palette, etc); NoteType colors stay fixed.
- **B:** accent replaces NoteType `definicao` color (since they share `violet` by default) and shifts the palette.

Lean **A** (decouple — semantic colors should be stable across themes). Confirm?

---

## Appendix A — Mapping JSX → egui

For implementers — quick translation table from the React/JSX primitives in `07-omninote-obsidian.jsx` to their egui equivalents.

| JSX | egui equivalent |
|---|---|
| `<div style={{ display: 'flex', flexDirection: 'column' }}>` | `ui.vertical(|ui| { ... })` |
| `<div style={{ display: 'flex', flexDirection: 'row' }}>` | `ui.horizontal(|ui| { ... })` |
| `<div style={{ display: 'flex', flexWrap: 'wrap' }}>` | `ui.horizontal_wrapped(|ui| { ... })` |
| `<div style={{ display: 'grid', gridTemplateColumns: '...' }}>` | `egui::Grid::new(id).num_columns(N).show(ui, |ui| { ... })` |
| `<button style={{...}}>` | `ui.add(egui::Button::new(label).fill(color))` or `ui.selectable_label(active, label)` |
| `<input>` / `<textarea>` | `egui::TextEdit::singleline / multiline` |
| `<span style={{ color: ... }}>{children}</span>` | `ui.label(RichText::new(text).color(color))` |
| `position: absolute; top, left` | `egui::Area::new(id).fixed_pos(Pos2::new(x, y)).show(ctx, |ui| { ... })` |
| `boxShadow` | `egui::Frame::default().shadow(Shadow { ... }).show(ui, |ui| { ... })` |
| `borderRadius` | `egui::Frame::default().rounding(N.0).show(ui, |ui| { ... })` |
| `border: '1px solid ...'` | `Frame::default().stroke(Stroke::new(1.0, color))` |
| `backdropFilter: blur(...)` | not supported in egui — omit or fake with semi-transparent overlay |
| `linear-gradient(...)` | render via `egui::Painter` with gradient mesh (use sparingly) |
| `cursor: pointer` | `ui.add(Button::new(...))` (cursor handled automatically) on `Response` |
| `@keyframes pulse` | per-frame interpolation against `Instant` + `ctx.request_repaint_after` |
| `<input type="checkbox">` | `ui.checkbox(&mut bool, label)` |
| `<select>` | `egui::ComboBox::from_id_salt(id).selected_text(...).show_ui(ui, |ui| { ... })` |
| ASCII border chars in mockups | for visual references only — actual UI uses `egui::Stroke` |

## Appendix B — File-touch matrix for Fase B

For project planning: which files Fase B touches, ordered by dependency depth.

| Layer | File | Op | Approx LOC |
|---|---|---|---|
| 0 | `types.rs` | extend with new enums + AppConfig fields | +120 |
| 0 | `theme.rs` | NEW — palette + presets | +250 |
| 1 | `app.rs` | extend struct + update() refactor | +180 |
| 1 | `actions.rs` | new handlers (palette, capture, chat, dictation, theme) | +400 |
| 2 | `ui_titlebar.rs` | NEW | +180 |
| 2 | `ui_statusbar.rs` | NEW | +120 |
| 2 | `ui_sidebar.rs` | extend with new section helpers | +400 |
| 2 | `ui_tabs.rs` | NEW (single-tab body for v1.2) | +120 |
| 2 | `ui_breadcrumb.rs` | NEW | +90 |
| 3 | `md_render.rs` | NEW — pulldown-cmark walker + wikilink/embed/tag widgets | +600 |
| 3 | `ui_editor.rs` | extend — daily-view dispatch, hover hooks | +250 |
| 3 | `ui_right_rail.rs` | NEW — tabs + Backlinks + Outline + AiChat stub | +350 |
| 3 | `ui_chat.rs` | NEW — chat session UI (deferred backend if Q-10=B) | +250 |
| 3 | `ui_palette.rs` | NEW — palette + quick capture + tag explorer | +500 |
| 3 | `ui_calendar.rs` | NEW | +200 |
| 3 | `ui_discipline.rs` | NEW — SPRINT/DIARY/HUMAN/PLAN/Tickets typed views | +600 |
| 3 | `ui_timeline.rs` | NEW (graceful if no git) | +200 |
| 3 | `ui_dictation.rs` | NEW (UI only if Q-13=A) | +150 |
| 3 | `ui_toasts.rs` | NEW | +120 |
| 4 | `ui_modals.rs` | extend — onboarding + conflict v2 + diff | +300 |
| 4 | Cargo deps | add pulldown-cmark, fuzzy-matcher, optionally git2/tokio | — |

Rough total: **~5500 LOC** of UI + handlers + theme + render. Plus tests (~30% of that). Estimate from the CAD-25 spec checklist suggests ~22h for Fase B — that aligns with ~5500 LOC at ~250 LOC/hour for an experienced Rust dev with tests.

---

## Appendix C — Glossary

- **Vault** — a directory on disk containing `.md` notes + `.omninote/` config + `_attachments/`.
- **Active note** — the note currently shown in the editor; held as a cloned `Note` on `OmniNoteApp.active_note` per the borrow-checker workaround in CLAUDE.md.
- **Sacred files** — `SPRINT.md`, `DIARY.md`, `JIRA.md`, `HUMAN.md`, `PLAN.md`, `NOTION.md`, `ETERNAL.md` — files with bespoke typed views.
- **Wikilink** — `[[Title]]` or `[[Title|Alias]]` or `[[Title#Heading]]` or `[[Title#^block-id]]` — link to another note.
- **Embed** — `![[…]]` — inline render of another note, image, or file.
- **Hashtag** — body-position `#tag` clickable chip.
- **Slash menu** — popup triggered by `/` at start of a line, lets user insert markdown blocks or AI/template/discipline actions.
- **Right rail** — 320 px right panel with Backlinks / Outline / AI Chat tabs.
- **Command palette** — Ctrl+P fuzzy palette for notes/commands/tags/disciplines.
- **Quick capture** — Ctrl+Shift+Space popup that appends a line to Inbox.md.
- **Discipline view** — typed widget rendering of a sacred file (SPRINT task list, DIARY entries, etc).
- **Theme preset** — `ObsidianDark` / `ObsidianLight` / `HighContrast` / `Custom` — bundle of color tokens.
- **NoteType** — one of `resumo` / `citacao` / `codigo` / `exercicio` / `duvida` / `definicao`; carries a glyph + color.
- **Toast** — non-modal bottom-right notification with auto-dismiss.
- **Fase A / Fase B** — analysis (this doc, CAD-25) / implementation (Sprint v1.2).

---

**End of UI_DESIGN_v2.md** — Fase A complete. Ready for Fase B kickoff once Q-01..Q-30 are batched-answered by Fausto.



