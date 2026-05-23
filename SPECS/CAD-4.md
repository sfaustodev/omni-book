# CAD-4 · 💾 Auto-save com debounce de 600ms

**Notion:** https://app.notion.com/p/35373ac79ddb8161a571d490e0c615f4
**Phase:** v0.1 · **Status:** 🎯 Pronta · **Priority:** 📌 Média · **Size:** 🐭 S (≤meio dia) · **Estimate:** 3h
**Type:** ✨ Feature · **Areas:** Vault

---

## 🎯 Objetivo
Nota é salva automaticamente 600ms após a última edição. Em `on_exit()`, força save final + `vault.save_config()`.

## ✅ Critérios de aceite
- [x] Flag `dirty: bool` setada em qualquer mudança no buffer da nota
- [x] No `update()`: se `dirty && last_save.elapsed() > 600ms`, chama `flush_active()`
- [x] Após save, `dirty = false` e `last_save = Instant::now()`
- [x] `on_exit()` força save mesmo se dentro do debounce
- [x] Title rename: se `note.title` mudou, renomeia o `.md` no disco
- [ ] Indicador visual sutil de "salvando…" → "salvo" no header (não implementado, opcional)

## 🛠️ Especificação técnica
`std::time::Instant` em `OmniNoteApp.last_save`. `flush_active()` faz: `take()` da `active_note`, salva, sincroniza com `vault.notes`, restaura.

**Borrow checker:** `Option::take()` é necessário pra liberar `&mut active_note` antes de chamar `&mut vault.save_note(...)`. Sem o `take`, conflita.

## 🧪 Como testar
1. Editar nota → esperar 600ms → conferir mtime do arquivo no disco
2. Editar e fechar app rapidamente → reabrir → mudanças preservadas
3. Stress: edição contínua → save acontece a cada ~600ms de pausa, não a cada keystroke
4. Renomear título → arquivo `.md` renomeado, frontmatter intacto

## 📎 Referências
- [SPEC.md](../SPEC.md) §1.3
- Implementação: [src/app.rs::flush_active](../src/app.rs)
