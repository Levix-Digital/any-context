---
id: 117
title: Harmonização de Cores Aurora Boreal, Canvas Borderless, Auto-Scroll ReAct, Mensagens Amigáveis de Update, Cancelamento Gracioso de Downloads e Onboarding Wizard Neutro
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0117: Harmonização de Cores Aurora Boreal, Canvas Borderless, Auto-Scroll ReAct, Mensagens Amigáveis de Update, Cancelamento Gracioso de Downloads e Onboarding Wizard Neutro

- **Status**: Aprovado & Implementado (`v0.34.9`).
- **Contexto**:
  1. A TUI utilizava cores amarelas e vermelhas para elementos rotineiros e informativos (como borda de accordion, workspace ativo, atalhos de menu), poluindo visualmente a estética Aurora Boreal e gerando falsos sentidos de urgência/alerta no usuário.
  2. A caixa de conversa possuía bordas e título que limitavam a imersão moderna em tela cheia (estilo ferramentas de ponta como Claude Code ou Warp).
  3. A caixa de raciocínio ReAct (accordion `<think>`) permanecia estática após atingir a altura limite, sem auto-scroll dos passos de raciocínio.
  4. Ao rodar `actx --update` logo após o lançamento de uma nova tag, o usuário se deparava com um erro bruto `404 status code`, pois o CI/CD do GitHub Actions ainda estava compilando os binários para Windows/Linux/macOS.
  5. Downloads de modelos neurais ONNX eram iniciados sem visualização de progresso numérico contínuo (% e MBs) e sem possibilidade de cancelamento gracioso pelo usuário.
  6. O menu de modelos ONNX causava dúvidas sobre necessidade de downloads e funções de cada modelo.
  7. O fluxo inicial carecia de um Onboarding Wizard guiado e neutro (sem enviesamento de "[Recomendado]").
- **Decisões**:
  1. **Harmonização da Paleta Aurora Boreal**:
     - Substituição de amarelo (`theme.warning`) e vermelho (`theme.error`) por Glacial Cyan (`theme.accent`), Cosmic Violet (`theme.reasoning`) e Brand Emerald (`theme.primary`) em todos os elementos informativos.
     - Amarelo e vermelho reservados estritamente para alertas reais de erro (`AppStatus::Error`, erros de ferramentas, pastas desconectadas).
     - Exceção explícita mantida: atalho `[Esc/Ctrl+C] Exit` permanece em Polar Crimson (`theme.error`).
  2. **Canvas de Chat Borderless**:
     - Configuração de `Block::default().borders(Borders::NONE)` na área de conversa, maximizando o espaço útil da janela.
     - Indicador sutil `⏸ Scroll Pausado [End p/ tempo real]` exibido no rodapé apenas quando a rolagem automática for desacoplada pelo usuário.
  3. **Auto-Scroll no ReAct & Reasoning**:
     - Adicionados campos `reasoning_scroll_offset`, `reasoning_auto_scroll` e `reasoning_max_scroll` no `App`.
     - Cálculo de altura renderizada do accordion com auto-scroll ativo para o final do raciocínio durante a geração de pensamentos. Suporte a `Ctrl+PgUp/Ctrl+PgDn` para inspeção manual.
  4. **Tratamento Amigável e Empático de 404 no `actx-installer`**:
     - Detecção de erro HTTP 404 em `downloader.rs` retornando a mensagem sentinela `ASSET_NOT_YET_AVAILABLE`.
     - Exibição de mensagem informativa e acolhedora inspirada nos melhores padrões da indústria, esclarecendo que o CI/CD ainda está construindo os pacotes da release, recomendando aguardar alguns minutos e disponibilizando o link oficial do release.
  5. **Telemetria e Cancelamento de Download de Modelos**:
     - Implementação de `download_model_with_progress` com canal de progresso e sinal de cancelamento via `tokio::sync::watch`.
     - Exibição em tempo real no rodapé: `📥 Baixando ⠋ <Model> X.X/Y.Y MB (Z%) [Esc p/ cancelar]`.
     - Cancelamento gracioso via `Esc` ou menu, abortando o streaming e excluindo arquivos parciais `.download.tmp` sem corrupção de disco.
  6. **Menu Document AI Reorganizado por Função**:
     - Estruturado em 5 submenus claros: 1. Ingestão de Documentos, 2. Sentinela de Scans, 3. Roteamento de Perguntas, 4. Visão de Documentos, 5. Resumo de Armazenamento.
     - Explicações claras de que nenhum modelo é obrigatório para o funcionamento pleno do AnyContext.
  7. **Onboarding Wizard Interativo Neutro**:
     - Wizard modal em 3 passos acionável via `/onboarding` ou `/setup`:
       * Passo 1: Provedor de LLM e Chave de API.
       * Passo 2: Perfil Document AI (Visão LLM, Extração Nativa, Modelos Locais). Totalmente neutro, sem marcadores `[Recomendado]`.
       * Passo 3: Pasta Inicial de Documentos (opcional).
- **Consequências**:
  - Experiência visual coesa, moderna e elegante na TUI.
  - Controle completo e transparente do usuário sobre downloads, atualizações e configurações de onboarding.
  - Zero atrito e zero mensagens de erro intimidadoras.
