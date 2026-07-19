# Córtex Experiment — Branch Report: `cortex/off-3` (Run #3)

- **Arm:** CÓRTEX **OFF** (baseline / control)
- **Run:** #3 of 3 (OFF) — the **clean / blind** sample
- **Design identity:** **"Almanac (redux)"** — warm parchment + terracotta editorial; left folder-tree sidebar + central editor with **backlinks inline under the note**; Space Grotesk display + JetBrains Mono; bundled OpenDyslexic for a11y. ⚠️ **Collides with Run #1's "Almanac"** — blind-convergence finding below.
- **Branch / commit:** `cortex/off-3` @ `8a03a7d`
- **Worktree:** `.claude/worktrees/cortex-off-3`
- **Anchor:** rebuilt from `cortex-baseline` (`ea6a911`); engine crates (`omninote-core/-ai/-cli/-mcp`) frozen
- **Condition:** **BLIND / fresh-context** — the builder was told only "isolated engineering task," and did **not** know the 4 metrics, that it was an experiment, or that runs #1/#2 existed; it was explicitly told not to read `.cortex-experiment/` or `discipline/`. → the orphan number is **unvitiated**.

## The four metrics (Mycorrhiza #238)

| # | Metric | Result | vs #1 / #2 |
|---|--------|--------|-----------|
| 1 ⭐ | **Feature-orphan rate** (independent auditor) | **1 / 14 = 7.1%** strict (letter-spacing control inert) · **0 / 14 wiring** | #1/#2 logged 0/14 — same stub latent there |
| 2 | **Engine green** (`fmt --check` + `clippy --workspace --all-targets -D warnings` + `test --workspace`) | **PASS** · engine **byte-identical** to baseline (`git diff cortex-baseline -- core/ai/cli/mcp` empty) | = |
| 3 | **Continuity** (regression / contradiction / re-derivation + churn) | **0 hard + 1 churn** (self-graded) | ≈ #1 (0+1); **not** #2 (0+0) |
| 4 | **Efficiency** (build tool-calls, worktree-add → CI-green) | **≈32** substantive (self-counted) · ~3 fix cycles · **not** first-compile-green | between #1(50) & #2(11); **confounded** ↓ |

GUI tests: **4 pass**. Engine unchanged (byte-identical) → core 252 · ai 126 · cli 12+38 (as #1/#2). Workspace total: 457 pass.

## ⭐ Orphan detail — the one strict orphan
Independent auditor (spawned blind to the builder's own notes) traced every trigger → code path:
- **13 / 14 REACHABLE**, each with file:line for both the trigger and the invoked path.
- **1 ORPHAN — accessibility letter-spacing:** the slider (`modals.rs:153`) is reachable and its value persists via `save_config()`, but `apply_style()` (`app.rs:~500`) applies `font_family` / `font_size` / `line_height` and **never reads `letter_spacing`** → the control has zero visual effect.
- **0 MISSING.**

**Parity note (load-bearing for the A/B).** This is an **egui-0.29 platform limitation** (no global letter-spacing in `Style`; it needs per-`LayoutJob` `extra_letter_spacing`), **not a wiring miss.** The identical inert stub exists in #1 and #2 (same egui, same `AppConfig.letter_spacing`), whose reports logged a11y as fully reachable. So **builder-wiring orphan rate = 0/14** (level with #1/#2); the strict number diverges only because **#3 received a stricter independent audit.** To keep the ruler honest, #1/#2 should be re-audited at this strictness (they'd likely also read 1/14), or #3 read as **0/14-wiring**. Do **not** attribute #3's 7.1% to worse building.

## 14-feature reachability (independent audit, file:line)
1 new note — `Cmd/Ctrl+N` `app.rs:369` + "+ nota" `sidebar.rs:153` (+ empty-state btn) → **REACHABLE**
2 edit/read — `Cmd/Ctrl+E` `app.rs:373` + toggle `editor.rs:52` → **REACHABLE**
3 search — `Cmd/Ctrl+K` `app.rs:376` + 🔍 `sidebar.rs:105` → palette `palette.rs:19` (fuzzy title + full-text) → **REACHABLE**
4 tree + type filters — chips `sidebar.rs:175`, folder toggle `sidebar.rs:242` → `row_visible` `sidebar.rs:193` → **REACHABLE**
5 wikilinks + backlinks — link click `md_render.rs:123` → `navigate_wikilink` `md_render.rs:12`; backlink click `editor.rs:183` → **REACHABLE**
6 img/PDF embeds — inline image `md_render.rs:136`; PDF open-external `md_render.rs:153` → **REACHABLE**
7 import PDF/chat/artifact — "importar" `sidebar.rs:164` + modal `modals.rs:184-192` → `modals.rs:204-241` → **REACHABLE**
8 FS watcher — init `app.rs:164`, polled every frame `app.rs:426` → `watcher.rs:35` → **REACHABLE** (automatic)
9 drag-drop — drag source `sidebar.rs:279` → drop zones `sidebar.rs:217/238` → `move_note` `app.rs:282` → **REACHABLE**
10 slash menu — detect `/` `editor.rs:237`, pick item `editor.rs:277` → **REACHABLE**
11 settings — `Cmd/Ctrl+,` `app.rs:381` + ⚙ `sidebar.rs:84` → `modals.rs:111` → **REACHABLE**
12 theme — `Cmd/Ctrl+Shift+D` `app.rs:384` + ☀/🌙 `sidebar.rs:92` → `toggle_theme` `app.rs:317` → **REACHABLE**
13 math — `Cmd/Ctrl+=` `editor.rs:16` → `try_math_substitute` `editor.rs:199` → **REACHABLE**
14 a11y — font family / size / line-height apply; **letter-spacing ORPHAN** (`modals.rs:153` → not read by `apply_style`)

## Cross-run finding — blind convergence on "Almanac"
Blind to #1, the builder independently reinvented **Run #1's warm-parchment editorial identity — down to the name "Almanac"** (parchment `#f4ecdd` + terracotta `#bf4d26`; Space Grotesk / JetBrains Mono). It differs from #1 only in layout (backlinks **inline under the note** vs #1's dedicated right connections panel). Evidence that **without retained memory the model returns to the same aesthetic attractor** for "distinct, non-generic notebook." Note the experimenter's "make #3 distinct from #1/#2" intent went unmet **because the builder was blind to it** — a cost of the clean condition, not a builder error. (Same mechanism, inverted, as #2's headline: retained context is what would have *avoided* the collision.)

## Efficiency / continuity caveats (honesty)
- **Front-loaded-planning confound.** #3 spent **4 research subagents in the planning phase** (3× Explore mapping the frozen `omninote-core` API + 1× Plan confirming egui-0.29 dnd / cursor-index / commonmark signatures). That moved API-discovery **out of the build window**, cutting in-build re-derivation to 0 and lowering the build-window count — so **≈32 is not clean-comparable** to #1(50)/#2(11), which did not fan out planning agents. Total session tool_use so far ≈ 86 (planning + build + commit + this report).
- **Not first-compile-green** (unlike #2): ~3 fix cycles — sidebar borrow-checker (collect-before-mutate), `md_cache`/`egui_commonmark` speculative dead-code removal, 3 clippy lints. Consistent with fresh-context: #2's zero-fix run came from **retained** egui muscle-memory that #3 lacked.
- **Continuity + efficiency are self-graded** (builder = grader, from this session's transcript). Metrics 1 (independent auditor) and 2 (git diff) are the externally-checkable, load-bearing ones.

## Caveats (not orphans)
- Kept skills (`button-remember`, `baseline-ui`) are Flutter/Blade-scoped → principle-only (full-stack Rust OFF).
- Quality: Serif→Proportional / Dyslexic→OpenDyslexic fallbacks; PDF embeds open externally (not inline-rendered); slash popup uses a fixed screen offset; image embeds via `file://` + `egui_extras` loaders (launch-smoke verified, not pixel-verified).
- The crate manifest was trimmed to the 8 deps actually used (dropped egui_commonmark/serde/serde_json/serde_yaml/chrono/dirs/uuid + unused dev-deps); `Cargo.lock` is gitignored, so CI regenerates it.
