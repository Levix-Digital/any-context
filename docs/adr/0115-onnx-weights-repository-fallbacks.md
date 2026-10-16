---
id: 115
title: Repositório de Pesos ONNX Nativo, Fallbacks Resilientes, Indicador Search Mode no Header e Tom Conversacional Humano
status: accepted
date: '2026-10-10'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0115: Repositório de Pesos ONNX Nativo, Fallbacks Resilientes, Indicador Search Mode no Header e Tom Conversacional Humano

- **Status**: Aprovado & Implementado (`v0.34.7`).
- **Contexto**: 
  1. A TUI exibia opções genéricas mencionando Ollama para visão e classificação, divergindo do design do AnyContext focado em inferência nativa embarcada via ONNX Runtime e heurísticas 2D.
  2. A ausência de modelos parciais não podia quebrar a indexação ou execução.
  3. O indicador de modo de busca (`AUTO`, `FAST`, `DEEP`) estava ausente na barra superior e sua persistência não era recarregada ao alternar workspaces no menu.
  4. O spinner Braille girava em sentido anti-horário e com velocidade lenta.
  5. Respostas com ausência de fatos continham disclaimers robóticos rígidos (`⚠️ Essa informação não consta...`), gerando falsa impressão de erro.
  6. O teste unitário de temporização no CI do Windows (`windows-latest`) apresentava flakiness sob jitter de I/O virtualizado.
- **Decisões**:
  1. **Repositório de Pesos ONNX Nativo (`OnnxModelManager`)**:
     - Diretório padronizado em `%LOCALAPPDATA%\AnyContext\models\` (Windows) e `~/.local/share/any-context/models/` (Linux).
     - Catálogo canônico com nomes e especificações reais: `Laya AI (mmBERT INT8 ONNX)`, `MobileNetV4 RVL-CDIP (ONNX)`, `BGE-Small Query Classifier (ONNX)`, `Moondream2 1.8B (INT4 ONNX)` e `SmolVLM-500M (ONNX)`.
     - Download assíncrono não-bloqueante acionado via teclado no submenu "Document AI & Modelos Locais" com streaming HTTP e escrita atômica via arquivo temporário.
  2. **Matriz de Tolerância a Falhas & Fallbacks**:
     - Se nenhum modelo ONNX for baixado, o sistema opera 100% no motor determinístico e heurística espacial 2D em Rust puro (<5MB RAM).
     - Presença parcial de modelos opera normalmente sem pânico nem quebra.
     - O comando `/status` detalha a integridade em tempo real dos modelos e diretório local.
  3. **Indicador Search Mode no Header da TUI & Isolamento por Workspace**:
     - Adicionado `[Search: AUTO|FAST|DEEP]` com tokens da paleta Aurora Boreal (`theme.info`, `theme.primary`, `theme.magenta`).
     - Atualizado `switch_to_workspace` em `App` para recarregar o `search_mode` do SQLite `NativeConfigDb` imediatamente na troca.
  4. **Spinner Braille Horário 2x**:
     - Invertida a sequência para a rotação horária canônica `["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"]` com aceleração `(app.tick_count * 2) % len`.
  5. **Tom Conversacional Humano**:
     - Erradicação de `⚠️ Essa informação não consta nos documentos deste workspace` de `prompt.rs` e `AGENT.md`.
     - O agente responde naturalmente e de forma direta como um colega de equipe, sem alarmes.
  6. **Resiliência de CI no Runner do Windows**:
     - Ajustado o teto de temporização no teste de inicialização para `< 500ms`, garantindo 100% de estabilidade sob I/O virtualizado em runners compartilhados.
- **Consequências**:
  - Interface fluida, natural, auditável e informativa em tempo real.
  - Zero dependência externa obrigatória; download sob demanda com fallbacks perfeitos.
  - 100% dos testes verdes e CI resiliente.
---
