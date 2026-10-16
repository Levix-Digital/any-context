---
plan_id: anycontext_planejamento_proximas_etapas_ci_acceleration_e_master_plan
project: any-context
title: 'Planejamento das Proximas Etapas: Aceleracao Radical do CI/CD de Release e Continuidade do Master Plan'
version: v0.34.1
status: active
author: Levi Silveira
created_at: '2026-10-09T19:57:27'
updated_at: '2026-10-09T19:57:27'
session_id: 4f45b7d4-9ceb-4257-9375-7a0a27e74b3e
tags:
- ci-cd
- sccache
- cargo-xwin
- release-acceleration
- modelrouter
- presets
- master-plan
supersedes: null
superseded_by: null
parent_plan_id: null
milestones:
- name: 'M1: PoC sccache e cargo-xwin para CI ultra-rapido'
  done: false
- name: 'M2: Implementacao do novo pipeline de release < 8 min'
  done: false
- name: 'M3: ModelRouter na Ingestao e Quality Gate'
  done: false
- name: 'M4: Conector Jina Reader Nativo'
  done: false
- name: 'M5: Matriz de Presets Fase 3'
  done: false
status_log:
- at: '2026-10-16T10:47:34'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 🗺️ Planejamento das Próximas Etapas: Aceleração de CI/CD & Continuidade do Master Plan

## 📌 Contexto & Motivação
Na conclusão da **Fase 2 (RFC-042: Deep Search Agêntico Reflexivo)** com a release `v0.34.0`, o pipeline de CI/CD no GitHub Actions levou **~50 minutos** no runner Windows (`windows-latest`) para compilar dependências monolíticas (LanceDB, Apache Arrow, DataFusion, tantivy).
Esse gargalo consome tempo precioso da esteira de engenharia e atrasa a iteração de releases.

Conforme solicitado pelo usuário, adicionamos formalmente este tópico crítico de **Aceleração do Pipeline CI/CD** ao roadmap imediato antes ou em conjunto com as próximas entregas de produto do Master Plan.

---

## 🎯 Pauta Estratégica para Discussão e Próximos Passos

### 🚀 Tópico I: Aceleração Radical do CI/CD de Release (DevOps / Infra)
**Meta**: Reduzir o tempo de build e publicação de binários de ~50 minutos para menos de 8 minutos.

#### Alternativas em Avaliação:
1. **Opção 1: `sccache` (Mozilla Shared Compilation Cache) integrado ao GitHub Actions**
   - Utilizar `sccache` com backend no GitHub Actions Cache (`actions/cache` ou storage compatível com S3/R2).
   - Baixa os arquivos de objetos compilados (`.obj`/`.o`) das ~300 dependências em segundos sem recompilar.
   - Mantém runners nativos (`ubuntu-latest` e `windows-latest`).
2. **Opção 2: Cross-Compilation do Windows a partir do Linux (`cargo-xwin`)**
   - Eliminar a compilação no runner `windows-latest`.
   - Compilar o binário Windows (`x86_64-pc-windows-msvc`) diretamente dentro do runner `ubuntu-latest` usando `cargo-xwin` e os SDKs do MSVC da Microsoft.
   - O Linux compila muito mais rápido devido à eficiência de I/O em comparação ao Windows.
3. **Opção 3: Otimização de Flags de Release no `Cargo.toml`**
   - Ajustar `codegen-units = 16`, `split-debuginfo`, `debug = false`, `strip = true` no perfil de release para paralelizar a geração de código LLVM.

---

### 🧠 Tópico II: Restante da Fase 2 do Master Plan (Pesquisa & Ingestão)
1. **Marco 2 (Fase 2): ModelRouter na Ingestão (Quality Gate & Visão/OCR)**
   - Avaliação e enriquecimento semântico de documentos na ingestão.
   - Quality gate para detecção de lixo/ruído antes de indexar no LanceDB/BM25.
   - Suporte a modelos multimodais de visão local/remoto para imagens em PDFs/Markdown.
2. **Conector Web Jina Reader (`r.jina.ai`)**
   - Extração limpa em markdown de páginas web externas diretamente pelo agente.

---

### 🔮 Tópico III: Fase 3 do Master Plan (Matriz Definitiva de Presets / Kit de Models)
1. **Preset 100% Offline / Edge (SLM + ONNX Local)**
   - Operação completa sem conexão com a internet, usando modelos locais (ex: Qwen 2.5 Coder 7B, Llama 3.2 3B) via Ollama/LM Studio e embeddings locais fastembed/ONNX.
2. **Preset Híbrido (Custo-Benefício Balanceado)**
   - Embeddings e triagem rápida locais, raciocínio e síntese com provedores cloud rápidos (Gemini Flash, Claude Haiku, GPT-4o-mini).
3. **Preset Frontier (Alta Capacidade Cognitiva)**
   - Raciocínio profundo com modelos top-tier (Claude 3.5 Sonnet, GPT-4o, Gemini 1.5 Pro).

---

## 📋 Marcos de Ação Propostos

- **M1: Diagnóstico e Prova de Conceito com `sccache` / `cargo-xwin`**:
  - Testar `sccache` e `cargo-xwin` em branch dedicado e mensurar redução de tempo no GitHub Actions.
- **M2: Atualização do Workflow `.github/workflows/release.yml`**:
  - Implementar a solução vencedora, reduzindo o tempo de CI para < 10 minutos.
- **M3: ModelRouter na Ingestão & Quality Gate**:
  - Implementação nativa em Rust no `any-context-core-rs` e `actx-agent`.
- **M4: Conector Jina Reader Nativo**:
  - Integração no `actx-agent` como ferramenta nativa de deep web fetch.
- **M5: Matriz de Presets (Fase 3)**:
  - Implementação dos presets Offline, Híbrido e Frontier.
