# Caderno

Caderno digital pessoal estilo Apple Notes/Obsidian em Rust nativo (egui).

Vault de arquivos `.md` em disco — 100% compatível com Obsidian e Claude Desktop (via MCP filesystem).

## Como rodar

```bash
cargo build --release
cargo run --release
```

Primeira build demora 5-10 min (compila egui inteiro). Depois é rápido.

## Estado

Veja [SPEC.md](./SPEC.md) — tem o que tá pronto, o que falta, e roteiro pra terminar.

## Stack

- `eframe` + `egui` 0.29 — UI imediata, single binary
- `egui_commonmark` — render de markdown
- `lopdf` — extração de texto de PDF
- `rfd` — file dialogs nativos
- `walkdir` + `notify` — varredura e watch do vault
- `serde_yaml` — frontmatter compatível com Obsidian

## Licença

MIT — Juan Fausto & Claude
