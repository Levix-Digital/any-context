---
id: 125
title: Ativação Operacional do Gemma 4 Local ONNX no Motor CLI, Guardrails de Hardware Estilo LM Studio e Presets Seguros de Workspace
status: accepted
date: '2026-10-14'
version: v0.40.0
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: [130]
---

# ADR-0125: Ativação Operacional do Gemma 4 Local ONNX no Motor CLI, Guardrails de Hardware Estilo LM Studio e Presets Seguros de Workspace

- **Status**: Implemented & Verified
- **Date**: 2026-10-14
- **Context**:
  1. Na versão `v0.38.0`, a infraestrutura neural local do Gemma 4 ONNX (`OnnxGemmaEngine` e `GemmaOnnxProvider`) foi introduzida no monorepo, porém o despachante de inferência do CLI (`crates/actx-cli/src/engine.rs::resolve_lm_provider`) ainda não realizava o roteamento operacional para carregar os pesos locais correspondentes ao tier solicitado.
  2. Em modelos locais ONNX (Gemma 4 INT4), tentar carregar pesos que excedam a capacidade de RAM física do hospedeiro (ex: Gemma 4 32B em máquina com 16GB) desencadeia *swap thrashing* agressivo no sistema operacional, congelando a interface e derrubando o processo por OOM (*Out-Of-Memory*).
  3. A alternância arbitrária de modelo de inferência por turno quebra a previsibilidade e viola a soberania da escolha deliberada feita pelo operador.
- **Decisão**:
  1. **Fiação Operacional do Gemma 4 no CLI (`crates/actx-cli/src/engine.rs`)**:
     - Inclusão do ramo de reconhecimento para `gemma-4*` e `gemma*` em `resolve_lm_provider`.
     - Resolução automática do arquivo de pesos `.onnx` no diretório canônico `%LOCALAPPDATA%/actx/models/`.
     - Aplicação estrita da garantia ADR-110 (*Zero-Breakage Guarantee*): caso o usuário selecione um modelo local cujos pesos ainda não foram baixados pelo `actx-installer`, o provedor ativa graciosamente o fallback offline (`MockLmProvider`), garantindo zero vazamento de dados para a nuvem, custo $0.00 e zero interrupção ou panic.
  2. **Guardrails de Hardware Ativos Estilo LM Studio (`crates/any-context-core-rs/src/hardware/mod.rs`)**:
     - Modelos de nuvem (OpenAI, Claude, Gemini, DeepSeek, Groq) permanecem 100% desimpedidos para alternância imediata.
     - Modelos locais têm seus requisitos de RAM validados em tempo de execução ($<5\text{ms}$ via OS APIs):
       - 🟢 **Safe**: Aprovado com folga de memória física.
       - 🟡 **Tight**: Alerta explicativo de pressão de memória, permitindo execução.
       - 🔴 **Unsafe**: **Bloqueio preventivo defensivo contra OOM**. A seleção é rejeitada com diagnóstico detalhado (RAM exigida vs RAM total/disponível no sistema) e recomendação do modelo seguro ótimo. A execução só é liberada se o usuário passar a flag explícita `--force` (`/model gemma-4-32b --force`).
  3. **Badges de Semáforo na TUI (`crates/actx-cli/src/tui/menu.rs`)**:
     - O menu interativo de modelos (`[F1]` ou `/model`) exibe badges e ícones dinâmicos de semáforo (`🟢 [Compatível]`, `🟡 [Risco de Swap]`, `🔴 [Hardware Insuficiente - Exige X GB]`).
  4. **Presets Seguros de Workspace (`/preset` ou `/kit`)**:
     - `offline`: reconfigura atomicamente o workspace para o maior modelo local seguro para a máquina (Compact para 8GB, 12B para 16GB, 27B para 32GB, 32B para 64GB+), modo estrito, busca rápida e web search desligada (custo $0.00 e isolamento de rede).
     - `balanced`: modelo equilibrado em nuvem (`gpt-4o-mini`), modo híbrido, busca auto e web search ativa.
     - `frontier`: modelo frontier analítico (`claude-3-7-sonnet`), modo proativo, deep search e web search ativa.
     - `privacy`: modelo local seguro com isolamento de rede corporativo.
  5. **Soberania do Usuário**:
     - O modelo escolhido pelo usuário é rigorosamente preservado em cada turno. A triagem inteligente do `SearchMode::Auto` opera exclusivamente sobre a profundidade da busca RAG (`Fast RAG` vs `Deep Search`).
- **Consequências**:
  - Imunidade total contra travamentos de máquina e OOM causados por seleção inadvertida de modelos pesados demais para o hardware.
  - Ativação 100% funcional da inferência neural local do Gemma 4 via CLI headless ou interativo.
  - Ergonomia superior através de atalhos rápidos com `/preset`.
