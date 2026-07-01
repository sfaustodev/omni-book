# Córtex Experiment — Branch Report: `cortex/off-1` (Run #1)

- **Arm:** CÓRTEX **OFF** (baseline / control)
- **Run:** #1 of 3 (OFF)
- **Design identity:** **"Almanac"** — warm parchment/oxblood editorial; left folder tree + right connections panel; top toolbar + bottom status bar
- **Branch / commit:** `cortex/off-1` @ `676df03`
- **Worktree:** `.claude/worktrees/cortex-off-1`
- **Anchor:** rebuilt from `cortex-baseline` (`ea6a911`); engine crates (`omninote-core/-ai/-cli/-mcp`) frozen
- **Condition:** *primed* (experimenter was also subject; knew the metrics) · **first** build of the same-context chain (no build muscle-memory yet)

## The four metrics (as defined at experiment start — Mycorrhiza #238)

| # | Metric | Result |
|---|--------|--------|
| 1 ⭐ | **Feature-orphan rate** — checklist items with no reachable entry point | **0 / 14** — all reachable (independent auditor, file:line evidence) |
| 2 | **Engine green** — `cargo test --workspace` + `clippy --all-targets -D warnings` + `fmt --check` | **PASS** · engine **byte-identical** to baseline (`git diff cortex-baseline -- core/ai/cli/mcp` empty) |
| 3 | **Continuity** — repeated error / contradiction / re-derivation | **0 hard events** (0 reg · 0 contra · 0 re-deriv) **+ 1 churn** (removed speculative state fields mid-build) |
| 4 | **Efficiency** — build tool-calls, tag → CI-green | **50** |

GUI tests: 11 pass. Engine unchanged: core 252 · ai 126 · cli 12+38.

## 14-feature reachability — all REACHABLE
new note (`Cmd/Ctrl+N` + button) · edit/read (`Cmd/Ctrl+E` + toggle) · search (`Cmd/Ctrl+K` + button) · folder tree + type chips · wikilinks + backlinks (right panel) · inline img/PDF embeds (+ attach button) · import PDF / Claude-chat / artifact · FS watcher (+ reload button) · drag-drop (note → folder) · slash `/` menu · settings (`Cmd/Ctrl+,` + button) · theme toggle (`Cmd/Ctrl+Shift+D` + button) · eval-math (`Cmd/Ctrl+=` + button) · a11y (font family / size / line / letter)

## Caveats (not orphans)
- Kept skills (`button-remember`, `baseline-ui`) are Flutter/Blade-scoped → only their **principle** applied by hand. Orphans 0/14 is the **floor** of the star metric, expected under full-stack OFF.
- Quality caveats: Serif/Dyslexic fonts → Proportional fallback (typefaces not bundled); PDF embeds render as links; slash popup uses a fixed offset.
