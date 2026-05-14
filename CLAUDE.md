# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

OmniNote — a native Rust desktop notebook app (previously called Caderno). Filesystem vault of `.md` files with YAML frontmatter, compatible with Obsidian and Claude Desktop MCP filesystem.

## Commands

```bash
cargo build                  # debug build
cargo build --release        # optimized release (~10MB stripped binary)
cargo run                    # run the app
cargo test                   # run all unit tests (vault, autoformat, import, wikilinks, pdf, types, actions)
cargo test vault::           # run only vault tests
cargo test autoformat::      # run only autoformat tests
cargo test import::          # run only import tests

# Coverage gate (pure modules ≥90% — CAD-12).
# One-time install: cargo install cargo-llvm-cov --locked
cargo llvm-cov --html \
  --include-files src/vault.rs --include-files src/wikilinks.rs \
  --include-files src/autoformat.rs --include-files src/import.rs \
  --include-files src/pdf.rs --include-files src/types.rs --include-files src/app.rs
open target/llvm-cov/html/index.html      # visual report

cargo llvm-cov --fail-under-lines 90 \
  --include-files 'src/vault.rs' --include-files 'src/wikilinks.rs' \
  --include-files 'src/autoformat.rs' --include-files 'src/import.rs' \
  --include-files 'src/pdf.rs' --include-files 'src/types.rs' --include-files 'src/app.rs'
```

UI render layer (`ui_*.rs`) is intentionally excluded — covered by [discipline/MANUAL_TEST_PLAN.md](discipline/MANUAL_TEST_PLAN.md) human checklist.

For macOS `.app` bundle: `cargo install cargo-bundle && cargo bundle --release`

## Architecture

**egui immediate-mode pattern:** `OmniNoteApp::update()` runs every frame. UI is rendered by calling methods — `show_sidebar()`, `show_editor()`, `show_modals()` — which are `impl OmniNoteApp` blocks split across source files. No retained UI components; all state lives in the struct.

**Active note as owned clone:** `active_note: Option<Note>` holds a cloned copy of the currently edited note. When saving, `flush_active()` takes ownership via `.take()`, saves to disk, syncs back into `vault.notes`, and restores `active_note`. This sidesteps the borrow checker conflict between `&mut vault` and `&mut active_note`. Never hold an index into `vault.notes` — indices invalidate on create/delete.

**Vault:** `Vault` is the single source of truth for on-disk state. All `.md` files under `vault.root` (excluding `.omninote/` and `_attachments/`) are loaded into `vault.notes` on open and after mutations. Call `vault.reload_notes()` after any external filesystem change. Config persists at `<vault>/.omninote/config.json`. Last-used vault path at `~/.config/omninote/last_vault`.

**Module layout:**
- `vault.rs` — CRUD for notes/folders, frontmatter parse, attachment import
- `types.rs` — `Note`, `Frontmatter`, `NoteType` (6 variants), `AppConfig`, `ConfirmAction`
- `app.rs` — `OmniNoteApp` struct, `flush_active`, `select_note`, `update()` main loop
- `ui_sidebar.rs` — sidebar panel (280px): search, type chips, folder tree, footer
- `ui_editor.rs` — central panel: edit mode (TextEdit) + view mode (CommonMarkViewer + backlinks)
- `ui_modals.rs` — 4 modal windows (new note, settings, confirm, import) + import helpers
- `autoformat.rs` — safe arithmetic on the current line (`Ctrl+=` shortcut)
- `import.rs` — Claude chat JSON and artifact import
- `pdf.rs` — PDF text extraction via `lopdf`

## Key egui constraints

- `TextEdit::show(ui)` returns `TextEditOutput` with `.cursor_range`. Use this (not `ui.add(TextEdit)`) when cursor position is needed. Drop the `TextEditOutput` before mutating the string it borrowed.
- Use `id_salt` not `from_id_source` (renamed in egui 0.29+).
- `egui_commonmark 0.18` requires egui 0.29 — keep versions pinned together.
- `rfd::FileDialog` is blocking (intentional; fine for a desktop app).
- `CommonMarkCache` lives in `OmniNoteApp.md_cache`. Never recreate it per frame.

## CI Commands

Run locally before pushing — mirrors the pipeline exactly:

```bash
cargo fmt --check                    # formatting (lint job)
cargo clippy --all-targets -- -D warnings  # lint (fails on warnings)
cargo test                           # all unit tests (test job)
cargo build --release                # release build check
cargo audit                          # security audit (install: cargo install cargo-audit)
```

On Linux, install display deps first (required by eframe/rfd):
```bash
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

## Next planned phases (from SPEC.md)

- **v0.4** — Wikilinks clickable (`[[Title]]` → `ui.link` using `pulldown-cmark`)
- **v0.5** — Accessibility: font family/size config, dark mode
- **v0.6** — Filesystem watcher (`notify` crate) to sync Obsidian edits
- **v0.7** — Drag-and-drop note reordering between folders
- **v0.8** — Slash menu in editor
- **v1.0** — Release builds, polish
