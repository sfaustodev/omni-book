# CAD-10 · 🔬 Spike: avaliar abordagem pra wikilinks clicáveis

**Notion:** https://app.notion.com/p/35373ac79ddb8144a057d5206e37fcbc
**Phase:** v0.4 · **Status:** 🌱 Backlog · **Priority:** 📌 Média · **Size:** 🐭 S (≤meio dia) · **Estimate:** 4h
**Type:** 🔬 Spike · **Areas:** Wikilinks, UI

---

## 🎯 Objetivo
Decidir como renderizar `[[Wikilinks]]` clicáveis dentro do CommonMark, já que `egui_commonmark` não dá hook em links.

## 🔬 Investigar
- [ ] **Opção A:** trocar `egui_commonmark` por parser custom usando `pulldown-cmark` + `ui.link`
- [ ] **Opção B:** fork de `egui_commonmark` adicionando o hook
- [ ] **Opção C:** post-render — renderiza commonmark normal e desenha overlay clicável sobre wikilinks
- [ ] Comparar esforço, performance e capacidade de embedar `![[arquivo.pdf]]` e `![[imagem.png]]` em cada uma

## 📊 Saída esperada
Recomendação (1 das 3 opções) com esboço de implementação e estimativa pro ticket de execução. Resultado vira novo CAD-XX (Feature) na Notion.

## 💭 Notas
A Opção A parece a mais limpa mas perde o trabalho do `egui_commonmark` (highlighting, tabelas, etc). Embedar `![[imagem.png]]` requer `egui::Image` — fica fácil em qualquer opção.

## 📎 Referências
- [SPEC.md](../SPEC.md) §6
- pulldown-cmark: https://docs.rs/pulldown-cmark/
- egui_commonmark source: https://github.com/lampsitter/egui_commonmark
