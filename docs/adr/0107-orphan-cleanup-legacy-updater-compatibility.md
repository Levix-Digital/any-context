---
id: 107
title: Erradicação de Arquivos Órfãos e Unificação de Release Assets para Atualizadores Legados
status: accepted
date: '2026-10-09'
version: null
subsystems: []
supersedes: []
superseded_by: []
amends: []
amended_by: []
---

# ADR-0107: Erradicação de Arquivos Órfãos e Unificação de Release Assets para Atualizadores Legados

- **Status**: Aprovado & Implementado (`v0.33.1`).
- **Contexto**: A transição de um ecossistema híbrido Python/Rust para 100% Rust exige que a base de clientes já instalada consiga transitar de versão sem intervenção manual e sem quebras em scripts de atualização.
- **Decisão**:
  1. Manter temporariamente nos pacotes de distribuição uma duplicata de compatibilidade (`actx-core.exe` / `actx-core`) idêntica ao binário principal nativo `actx`.
  2. Implementar no `crates/actx-installer` lógica de limpeza silenciosa para remover `actx-core.exe` e diretórios `_internal` quando executado por versões 0.33.x+.
  3. Purgar os últimos arquivos órfãos em Python e consolidar o padrão Dual-Doc.
- **Consequências**:
  - Atualização automática transparente para 100% dos usuários legados com um simples `actx --update`.
  - Repositório limpo, sem dependências ou arquivos mortos.
  - Zero Python em produção e zero overhead de empacotamento.

---
