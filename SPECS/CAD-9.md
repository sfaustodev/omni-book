# CAD-9 · 👁️ Watcher de filesystem com `notify`

**Notion:** https://app.notion.com/p/35373ac79ddb8139b759e88e9f3ef71d
**Phase:** v0.6 · **Status:** 🌱 Backlog · **Priority:** 🌿 Baixa · **Size:** 🐈 M (1-2 dias) · **Estimate:** 5h
**Type:** ✨ Feature · **Areas:** Watcher, Vault

---

## 🎯 Objetivo
Detectar mudanças externas no vault (Obsidian ou MCP do Claude Desktop edita um `.md`) e recarregar automaticamente.

## ✅ Critérios de aceite
- [ ] Watcher inicializado em `App::new()` apontando pra `vault.root` recursivo
- [ ] Mudanças drenadas no `update()` via `mpsc::channel`
- [ ] Em mudança detectada: `vault.reload_notes()` chamado
- [ ] Se a nota ativa foi modificada externamente: conflict resolution (vide [HUMAN.md Q-03](../discipline/HUMAN.md))
- [ ] Eventos `Create/Modify/Remove` tratados
- [ ] Self-write filter: nosso próprio save não dispara reload

## 🛠️ Especificação técnica
```rust
use notify::{Watcher, RecursiveMode, Event};
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(tx)?;
watcher.watch(&vault.root, RecursiveMode::Recursive)?;
```

**Cuidado:** o próprio OmniNote escrevendo arquivo gera evento. Filtrar com flag `self_write_until: Instant` durante save (janela de 100ms).

**De-bounce:** notify pode emitir vários eventos em rajada (Move = Remove + Create). 100ms de janela deve bastar.

## 💭 Decisão pendente
[HUMAN.md Q-03](../discipline/HUMAN.md) — como tratar conflito quando humano edita externamente nota ativa.

## 🧪 Como testar
1. Abrir OmniNote em vault X
2. Editar `.md` no Obsidian no mesmo vault → salvar
3. OmniNote detecta e recarrega → nota atualizada na sidebar
4. Stress: criar 100 notas via script externo → todas aparecem
5. Self-test: editar no OmniNote → não dispara reload-loop

## 📎 Referências
- [SPEC.md](../SPEC.md) §4
- notify crate: https://docs.rs/notify/
