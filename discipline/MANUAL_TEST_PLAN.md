# MANUAL_TEST_PLAN — OmniNote

> Checklists for UI-only surfaces that can't be covered by `cargo test` (eframe
> glue, OS-native integration, rfd dialogs, panic hook). Referenced from
> `discipline/SPRINT.md` §2 Definition of Done. Human runs these on macOS
> local and confirms in chat ("testado, pode fechar") — rule #13, never
> closed on green CI alone.

---

## CAD-25 Slice 7 — Theme gallery + native macOS menu bar (2026-07-10)

`cargo build/test/clippy/fmt` all green (see `discipline/DIARY.md` same date) —
this checklist covers only what those can't: the real `NSApp` menu bar and
live re-themeing, which need an actual macOS window.

1. `cargo run` (or the release binary) — confirm the macOS top menu bar shows
   **Arquivo / Editar / Tema** next to the default OmniNote app menu (About/Quit).
2. Click each of the 9 **Tema** entries (Terminal Escuro/Claro, Almanac/Almanac
   Noite, Blueprint/Blueprint Rascunho, Swiss, Alto Contraste, Personalizado) —
   the app re-themes live and the checkmark follows the selection.
3. Open a note in edit mode, select some text, click each **Editar** formatting
   item (Negrito/Itálico/Tachado/Código/Link/Bloco/H1-3/Lista/Lista numerada/
   Tarefa/Citação) — output matches what the same item already produces via
   right-click or the `/` slash menu.
4. Settings modal (⌘,) → the new "Tema" ComboBox lists and applies all 9
   presets; the accent color picker appears only under "Personalizado"; picking
   a preset there also updates the native Tema menu's checkmark.
5. ⌘B / ⌘I now also trigger Bold/Italic (new accelerators) without visibly
   colliding with any existing shortcut (⌘N/E/,/W/P/Shift+Space/Shift+D/
   Shift+J/Shift+H/\\/=).
6. Selecionar tudo / Copiar (Editar menu, no accelerator) act on the active
   note's content as expected.

**Known non-goal (documented in `native_menu.rs`):** Cut/Paste/Undo/Redo are
not native-menu items yet — they already work via keyboard (⌘X/C/V/Z).
