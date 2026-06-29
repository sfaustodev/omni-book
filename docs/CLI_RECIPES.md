# CLI Recipes

Short, copy-paste recipes for the `omninote` CLI. Every verb accepts `--json`
for machine-readable output; the envelope is `{ok, data, meta?}` on success or
`{ok, error}` on failure.

Vault resolution order (shared by every verb): `--vault <PATH>` →
`OMNINOTE_VAULT` env → active entry in `~/.config/omninote/vaults.toml` →
legacy `~/.config/omninote/last_vault`.

## `omninote capture` — one line to the Inbox

Append a quick thought to `Inbox.md` without opening the app. The line is
prepended as a timestamped bullet (newest-first); the file is created with an
`# Inbox` heading on the first capture.

```bash
omninote capture "buy milk on the way home"
```

```text
✓ Inbox.md  (+1 line)
- 2026-06-27 14:03 · buy milk on the way home
```

Target a specific vault (otherwise the active one is used):

```bash
omninote --vault ~/notes/work capture "follow up with Dana re: Q3"
OMNINOTE_VAULT=~/notes/work omninote capture "same, via env"
```

Machine-readable form — the bullet and running line count come back in the
envelope:

```bash
omninote capture "ship the release" --json
```

```json
{"ok":true,"data":{"path":"/Users/me/notes/Inbox.md","line_appended":"- 2026-06-27 14:05 · ship the release"},"meta":{"total_lines":42}}
```

Notes:

- Empty or whitespace-only text is rejected (`linha vazia`) and nothing is
  written.
- Capture is **not** idempotent — every call adds a line, which is the point.
- The bullet format matches the in-app quick-capture, so terminal and GUI
  captures interleave cleanly in the same `Inbox.md`.

### Pipe from other tools

Because the text is a positional argument, wrap multi-word or shell-special
input in quotes:

```bash
omninote capture "$(pbpaste)"          # capture the clipboard (macOS)
omninote capture "TODO: $(date +%F) review PR backlog"
```
