# Córtex Experiment — OFF arm (baseline) summary

First experiment of "Livro II" (Mycorrhiza elos **#236 → #240**). **Hypothesis:** a working-memory
"córtex" layer improves Claude Code's *long-horizon* coding — measured by **continuity / orphan-wiring /
efficiency**, not raw correctness. **Cobaia:** rebuild the `omninote-gui` crate from scratch each round
against a fixed 14-feature checklist; the engine (`omninote-core/-ai/-cli/-mcp`) is **frozen = gabarito**.
Plan: **OFF ×3 → build the córtex → ON ×3**, only the córtex toggles. 3×3 = **pilot** (signal, not proof).

Anchor tag: `cortex-baseline` @ `ea6a911`. Each run lives in an isolated worktree; the main branch +
WIP are untouched; nothing here is a PR.

## Results so far

| Branch | Run | Design | ⭐ Orphans | Engine green | Continuity | Build tool-calls | Report |
|--------|-----|--------|-----------|--------------|------------|------------------|--------|
| `cortex/off-1` @ `676df03` | #1 | Almanac (warm editorial) | 0 / 14 | PASS · frozen | 0 hard + 1 churn | 50 | [branch-1.md](branch-1.md) |
| `cortex/off-2` @ `27fc9a6` | #2 | Blueprint (navy drafting) | 0 / 14 | PASS · frozen | 0 hard + 0 churn | 11 · first-compile-green | [branch-2.md](branch-2.md) |
| `cortex/off-3` @ `8a03a7d` | #3 | Almanac-redux (warm parchment) ⚠️≈#1 | **1/14 strict · 0/14 wiring** | PASS · frozen | 0 hard + 1 churn (self) | ≈32 · confounded ↓ | [branch-3.md](branch-3.md) |

## ⚠️ The one methodological caveat that matters
Runs #1 and #2 were done in the **same session** (context retained, on purpose). Run #2's 50→11
efficiency gain came from **retained context**, which is *exactly the mechanism a córtex provides* — so
a same-context baseline already captures the córtex's main effect. **#1/#2 are therefore *exploratory*
(primed + context-retained); #3 is the *clean/blind* sample** (fresh context, builder unaware of the
metrics). For a valid A/B, the eventual **ON runs should also be blind/fresh-context**, or they won't
compare to #3. Full reasoning in [branch-2.md](branch-2.md).

## Status
- OFF #1 — **done**, committed on `cortex/off-1`.
- OFF #2 — **done**, committed on `cortex/off-2`.
- OFF #3 — **done**, committed on `cortex/off-3` @ `8a03a7d` (blind/fresh-context). Two findings: (a) **blind convergence** — the builder reinvented #1's warm-parchment "Almanac" identity down to the name, unaware #1 existed; (b) the one strict orphan (a11y **letter-spacing** inert) is an **egui-0.29 limitation latent in #1/#2 too**, so the honest cross-run orphan rate is **0/14 wiring** for all three — re-audit #1/#2 at #3's strictness for parity.

## Handoff — "o resto"
Once `branch-3.md` exists: (1) fill the table row above; (2) treat **#3** as the OFF number that a
future ON run must beat *in the same fresh-context condition*; (3) the OFF arm is done → next stage is
**building the córtex**, then ON ×3 (blind). Note two soft spots to keep honest: continuity is
self-graded (needs an outside judge, unlike the independent orphan audit), and tool-calls/continuity for
#3 must come from that build session's transcript (see the prompt).

Internal detailed records also live in `../.cortex-experiment/results/off-{1,2}.md` and the project
DIARY entry (`discipline/DIARY.md`, 2026-06-30).
