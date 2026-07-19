# Prompt — generate the Run #3 branch report (`branch-3.md`)

Use this **after** the blind/fresh-context Run #3 build exists in the worktree
`.claude/worktrees/cortex-off-3` (branch `cortex/off-3`). It produces `branch-3.md` in the **same
format** as `branch-1.md` / `branch-2.md`, using the same rubric so the numbers are comparable.

Paste the block below to whoever generates the report (a Claude Code session with shell access is ideal;
Claude Desktop with filesystem access can do the orphan audit + write the file, and mark the
shell-dependent metrics as "pending" for a Code session to fill).

---

```
Gere o relatório de branch da Run #3 do experimento córtex e salve em
/Users/jf/Projects/ClaudeBook/caderno/cortex-reports/branch-3.md, no MESMO formato de
cortex-reports/branch-1.md e branch-2.md. Alvo (read-only p/ auditoria):
.claude/worktrees/cortex-off-3/crates/omninote-gui/src/ (branch cortex/off-3).

Contexto: Run #3 é o braço OFF **cego / contexto fresco** — o builder NÃO sabia das métricas nem que
era experimento (então o número de órfãs é o não-viciado). Engine (omninote-core/-ai/-cli/-mcp) era
congelada. Design deve ser distinto dos anteriores (Almanac=pergaminho, Blueprint=navy).

Meça as 4 métricas (defs do #238):

1) ⭐ ÓRFÃS — pra CADA uma das 14 features, achar um gatilho alcançável (consume_key de atalho E/OU
   widget clicável) que invoca o code path. Classifique REACHABLE / ORPHAN (implementado mas sem
   gatilho) / MISSING (não implementado), com evidência file:line. Órfã-rate = (ORPHAN+MISSING)/14.
   As 14: nova nota (Cmd/Ctrl+N) · editar/ler (Cmd/Ctrl+E) · busca (Cmd/Ctrl+K) · árvore de pastas +
   filtros de tipo · wikilinks + backlinks · embeds inline img/PDF · import (PDF/chat Claude/artefato) ·
   watcher FS · drag-and-drop · menu "/" · settings (Cmd/Ctrl+,) · tema (Cmd/Ctrl+Shift+D) ·
   avaliar matemática (Cmd/Ctrl+=) · acessibilidade (fonte/tamanho/espaçamento).

2) ENGINE VERDE (se tiver shell) — dentro do worktree cortex-off-3:
   cargo fmt --check ; cargo clippy --workspace --all-targets -- -D warnings ; cargo test --workspace
   PASS/FAIL. E prove o engine congelado:
   git -C .claude/worktrees/cortex-off-3 diff --stat cortex-baseline -- \
     crates/omninote-core crates/omninote-ai crates/omninote-cli crates/omninote-mcp   (tem que ser VAZIO)
   Sem shell: marque "pending (needs a Code session)".

3) CONTINUIDADE — do TRANSCRIPT da sessão de build do #3 (o Claude Code que construiu): conte eventos
   duros = {regressão (consertou→re-quebrou), contradição (reverteu decisão sem info nova), re-derivação
   (redescobriu o que já sabia)} + churn. Menor = melhor. Se não tiver o transcript, use o auto-report
   do builder e marque como self-graded. (Métrica mole — idealmente juiz externo.)

4) EFICIÊNCIA — tool-calls de build (do worktree-add até CI verde). Conte os "tool_use" no JSONL da
   sessão de build (~/.claude/projects/-Users-jf-Projects-ClaudeBook-caderno/<session>.jsonl) entre o
   marker de criação do worktree e o gate de CI; ou use o número auto-reportado pelo builder.

Escreva branch-3.md com: cabeçalho (arm OFF, run #3, design, branch cortex/off-3 @ <commit>, condição =
CEGO/contexto fresco), a tabela das 4 métricas, a lista de reachability das 14, e caveats. NÃO invente
número: o que não der pra medir, marque "pending". No fim, atualize a linha da Run #3 na tabela de
cortex-reports/SUMMARY.md.
```

---

### Note on the two hard-to-get metrics
- **Efficiency (tool-calls)** and **continuity** both need the **build session's transcript**. If Run #3
  was built in a separate Claude Code session, grab that session's JSONL (or its self-reported count).
  Claude Desktop alone can't see it → leave those "pending" and let a Code session fill them.
- **Orphan rate** and **engine-frozen** are computable from the code + git and are the load-bearing ones;
  don't block the report on the other two.
