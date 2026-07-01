# Córtex Experiment — Branch Report: `cortex/off-2` (Run #2)

- **Arm:** CÓRTEX **OFF** (baseline / control)
- **Run:** #2 of 3 (OFF)
- **Design identity:** **"Blueprint"** — cool navy/cyan drafting; mono headings; near-square corners; command bar on top + **bottom "References" dock** hosting wikilinks/backlinks (distinct from #1's warm parchment + right panel)
- **Branch / commit:** `cortex/off-2` @ `27fc9a6`
- **Worktree:** `.claude/worktrees/cortex-off-2`
- **Anchor:** rebuilt from `cortex-baseline` (`ea6a911`); engine crates frozen
- **Condition:** *primed* · **SAME session as #1** — context retained on purpose (Juan: "procedimento normal típico… nem vou limpar o contexto"). I **remembered** run #1's engine API + egui gotchas.

## The four metrics (Mycorrhiza #238)

| # | Metric | Result | vs #1 |
|---|--------|--------|-------|
| 1 ⭐ | **Feature-orphan rate** | **0 / 14** — all reachable (independent auditor) | = |
| 2 | **Engine green** (`test --workspace` + clippy + fmt) | **PASS** · engine byte-identical to baseline | = |
| 3 | **Continuity** (repeat / contradict / re-derive) | **0 hard events + 0 churn** | better (lean state from the start) |
| 4 | **Efficiency** (build tool-calls) | **11** · compiled **green on first attempt** (0 fix cycles) | **−78%** (was 50, ~5 fix cycles) |

GUI tests: 10 pass. Engine unchanged: core 252 · ai 126 · cli 12+38.

## ⚠️ Headline finding — the 50→11 gain is a CONFOUND, not a córtex win
The drop came **purely from retained context**: I remembered #1's engine API and every egui trap
(`Window::open` borrow, cursor-range path, dnd signatures, test-module order, lean state), so all 8
modules compiled right the first time. **But retained working-memory of lessons is exactly what the
córtex is meant to provide.** So a **same-context** baseline already contains the córtex's main effect —
the model's context window *is* the working memory here.

**Consequence for the experiment:** to isolate the córtex you must compare **fresh-context OFF vs
fresh-context ON** (#238's "contaminação zero"). This same-context #2 makes the OFF arm artificially
strong on continuity + efficiency (conservative *against* the córtex), and its **11 is not comparable**
to a fresh-context ON run. Attributing this gain to a córtex would be a falsified ruler (cf. #236
"caça-níquel com gráfico"). → this is why **Run #3 is run blind / fresh-context** as the clean sample.

## Cross-run continuity (lessons carried #1 → #2)
lean state from the start (killed #1's churn) · test module at end of `look.rs` (pre-empted the
`items-after-test-module` clippy fix) · `Window::open` cancel-flag pattern from the first write ·
all egui APIs correct first-try (zero re-derivation).

## Caveats (not orphans)
Same as #1: Flutter/Blade skills → principle-only; Serif/Dyslexic → Proportional; PDF embeds as links.
