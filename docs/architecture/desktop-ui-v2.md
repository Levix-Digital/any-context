# Interface desktop (Tauri 2) — referência de layout

Guia do layout da interface desktop, a partir do mockup e dos blueprints de protótipo e de
tokens visuais (2026-10). **O que vale é o layout.** Valores, rótulos e opções do mockup
(ex.: Chunk Size 512, Score Threshold 0.75, Qdrant, top-K) são ilustrativos e serão
substituídos pelos reais, seguindo os designs de ingestão, recuperação e implantação.

## Estrutura em três colunas

| coluna | conteúdo | largura (mín · padrão · máx) | recolher |
|---|---|---|---|
| **Context Explorer** (esquerda) | árvore de workspaces e, dentro de cada um, suas fontes (pastas expansíveis, sites expansíveis em páginas, conectores), com status de cada fonte; botão "+ Novo"; rodapé com perfil do usuário e atalhos | 200 · 260 · 420 px | `Cmd/Ctrl + B` ou arrastar |
| **Canvas Hub** (centro) | barra de abas: uma aba de **chat por workspace** e uma aba por **fonte aberta** (leitor); conteúdo da aba ativa | 480 px · flexível | — |
| **Workspace Inspector** (direita) | configurações do contexto da aba ativa (workspace ou documento): seção básica exposta, seção avançada em accordion | 240 · 280 · 480 px | `Cmd/Ctrl + I` ou arrastar |

Barra superior fixa (38 px) com nome do app, busca rápida (`Cmd/Ctrl + K`) e menu global.

## Comportamento

- **Clicar num workspace** abre ou foca a aba de chat dele; **clicar numa fonte** abre ou
  foca uma aba de leitura. Abas não se repetem: abrir algo já aberto traz a aba para o
  foco.
- **Aba de chat:** histórico com streaming e citações clicáveis (abrem a fonte no ponto
  citado); composer multilinha com barra fixa de opções rápidas (modelo, modo de
  recuperação, busca web, sync, enviar), salvas por workspace.
- **Aba de leitura:** documento somente leitura (realce de sintaxe, numeração de linhas,
  metadados de indexação, reprocessar o documento). Abrir arquivos, navegar pastas e ver
  sites sem sair do AnyContext é escopo desta interface.
- **Inspector:** acompanha a aba ativa.

## Design system

Tokens visuais (cores semânticas, tipografia, espaçamentos, raios, elevações) definidos na
especificação de tokens: tema escuro (superfícies de `#0B0C0E` a `#1E2330`), acento
primário `#2563EB`, status de fonte (sincronizada, sincronizando, erro), fonte sans nativa
e mono para código, espaçamento base de 4 px.

## Regras herdadas das decisões de arquitetura

- Um chat por workspace, individual por usuário; nenhuma consulta atravessa workspaces.
- A documentação do AnyContext é a ajuda do sistema, não um workspace da árvore.
- Workspaces compartilhados e da organização aparecem na árvore com o papel do usuário
  (dono, editor, leitor).
