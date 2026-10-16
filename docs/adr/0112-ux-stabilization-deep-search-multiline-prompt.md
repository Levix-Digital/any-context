---
id: 112
title: Estabilização de UX, Paridade Linguística Deep Search, Prompt Multilinhas e Auto-Conhecimento Transparente
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0112: Estabilização de UX, Paridade Linguística Deep Search, Prompt Multilinhas e Auto-Conhecimento Transparente

- **Status**: Aprovado & Implementado (`v0.34.4`).
- **Contexto**: A evolução do AnyContext para suporte a Deep Search e múltiplos modos exigia refinamento fino em detalhes de interação humana (acessibilidade de teclado, fidelidade de idioma, legibilidade de telemetria) e garantia de que manuais de auto-ajuda permaneçam indexados entre versões.
- **Decisões**:
  1. **Sanitização de Conclusão Deep Search**: Reforçar as instruções do `SUMMARY_PROMPT` em `actx-agent/src/deep_search.rs` e adicionar `sanitize_direct_conclusion` para podar metanarrativas reflexivas.
  2. **Persistência de `search_mode`**: Adicionar a coluna `search_mode` na tabela `workspaces` com migração automática transparente e suporte nos comandos `/search` e `/switch`.
  3. **Event Loop TUI a 125ms & 8-Dot Spinner**: Aumentar a cadência visual de sincronização em segundo plano reduzindo a latência do loop para 125ms com glifos `⣾⣽⣻⢿⡿⣟⣯⣷`.
  4. **Instrução de Atualização Segura**: Substituir chamada in-process de atualização por instrução segura para terminal do SO (`actx --update`), prevenindo colisões de I/O em raw-mode.
  5. **Input Multilinhas Flexível**: Adicionar suporte a múltiplas linhas no buffer de entrada (`\n`), captura de `Shift+Enter` / `Alt+Enter` / `Ctrl+J`, cálculo dinâmico de altura (`clamp(3, 8)`) e cursor 2D `(x, y)`.
  6. **Identidade Visual por Workspace**: Prefixar respostas com `[AI - <Workspace>]` na cor de destaque suave `Color::LightCyan` e renderizar texto digitado pelo usuário em `Color::Gray`.
  7. **Auto-Consciência Versão-Ciente**: Armazenar `global_knowledge_version` em `app_settings` no SQLite e disparar sincronização transparente do `Global` em segundo plano quando a versão for atualizada.
  8. **Document AI no Menu**: Incluir itens explicativos e de ação para modelos locais e visão espacial em `actx-cli/src/tui/menu.rs`.
- **Consequências**:
  - Eliminação completa de respostas em inglês para perguntas feitas em português no Deep Search.
  - O usuário visualiza com clareza em qual workspace a IA está atuando e o que o motor está executando nos bastidores.
  - Nenhuma perda de auto-consciência em relação aos comandos e funcionamento do sistema após updates.

---
