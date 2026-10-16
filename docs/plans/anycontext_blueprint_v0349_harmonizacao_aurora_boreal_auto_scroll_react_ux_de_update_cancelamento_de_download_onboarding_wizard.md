---
plan_id: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_cancelamento_de_download_onboarding_wizard
project: any-context
title: 'Blueprint v0.34.9: Harmonizacao Aurora Boreal, Auto-Scroll ReAct, UX de Update, Cancelamento de Download & Onboarding Wizard'
version: v0.34.9
status: superseded
author: Levi Silveira
created_at: '2026-10-10T12:57:16'
updated_at: '2026-10-10T14:42:45'
session_id: a94494e0-08bb-4378-888d-a3bde8673537
tags:
- tui
- aurora_theme
- borderless_chat
- reasoning_autoscroll
- update_ux
- model_download_cancel
- guided_models_menu
- onboarding_wizard
supersedes: anycontext_blueprint_v0349_harmonizacao_aurora_boreal_auto_scroll_react_ux_de_update_download_guiado_de_modelos
superseded_by: anycontext_blueprint_v03410_standalone_onnx_models_download_resilience_404_fix
parent_plan_id: null
milestones:
- name: 'M1: Limpeza Cromatica Aurora'
  done: false
- name: 'M2: Conversation Borderless'
  done: false
- name: 'M3: Auto-Scroll ReAct'
  done: false
- name: 'M4: Mensagem Amigavel actx --update'
  done: false
- name: 'M5: Telemetria e Cancelamento Download ONNX'
  done: false
- name: 'M6: Submenus Guiados por Funcao'
  done: false
- name: 'M7: Wizard de Onboarding'
  done: false
- name: 'M8: Testes e Dual-Doc'
  done: false
status_log:
- at: '2026-10-10T13:28:52'
  note: 'v0.34.9 entregue com sucesso: Aurora Boreal harmonizada, chat borderless, auto-scroll ReAct, 404 amigavel no update, telemetria/cancelamento de download e onboarding neutro implementados e validados com 100% dos testes verdes.'
- at: '2026-10-10T14:42:45'
  note: Superseded by anycontext_blueprint_v03410_standalone_onnx_models_download_resilience_404_fix
- at: '2026-10-16T10:47:35'
  by: Levi Silveira
  note: Migrated from the local plan-memory database to docs/plans
---

# 📐 Blueprint v0.34.9: Harmonização Aurora Boreal, Auto-Scroll ReAct, UX de Update, Cancelamento de Download & Onboarding Wizard

**Projeto**: AnyContext (`actx`)  
**Versão Alvo**: `v0.34.9`  
**Data**: 2026-10-10  
**Status**: 📋 Planejamento Completo para Aprovação  

---

## 🎯 1. Escopo de Ajustes

1. **Harmonização Cromática Aurora Boreal na TUI**:
   - Eliminar amarelo (`Amber`) e vermelho (`Crimson`) de componentes informativos e de navegação (Thinking, Atalhos de Menu, Border de ReAct, Active Workspace, Fast RAG, Deep Search).
   - Substituir pelas cores correspondentes da paleta Aurora Boreal: **Glacial Cyan** (`#00F0FF`), **Brand Emerald** (`#00E5A3`), **Cosmic Violet** (`#A855F7`) e **Arctic Sky Blue** (`#38BDF8`).
   - Restringir estritamente amarelo e vermelho para:
     - **Vermelho (`#EF4444`)**: Atalho destrutivo `[Esc/Ctrl+C] Exit` (solicitado como exceção explícita), erros críticos de execução (`[Tool Error]`, `AppStatus::Error`) e pastas offline irrecuperáveis (`⚠️ [Sync: X folder(s) unreachable]`).
     - **Amarelo (`#F59E0B`)**: Avisos reais de sincronização pendente (`⚡ [Sync: Changes detected (/sync)]`).

2. **Caixa "Conversation" sem Bordas e sem Título (Canvas Minimalista)**:
   - Realizar o teste visual removendo a borda externa e o título `Conversation`.
   - Ganhar +2 linhas de altura útil e +2 colunas de largura útil para o fluxo de chat.
   - Manter a sinalização de scroll pausado de forma minimalista caso o usuário suba a conversa.

3. **Auto-Scroll Fluido na Caixa ReAct & Reasoning**:
   - Corrigir a ausência de cálculo de scroll em `render_accordion`.
   - Implementar cálculo de wrapping, limites de altura e auto-scroll automático para o final à medida que novos tokens e tool calls são injetados.

4. **Telemetria e Cancelamento de Download de Modelos ONNX**:
   - **Progresso na tela**: Streaming do download com porcentagem, MB baixados e velocidade na barra de status inferior da TUI (`📥 [Baixando Laya AI: 45% (94.5/210 MB) 4.2 MB/s]`).
   - **Cancelamento Elegante**: O usuário pode cancelar o download a qualquer momento via `Esc` (quando na tela principal) ou via `/menu -> Cancelar Download`.
   - **Zero Poluição de Disco**: O cancelamento aborta a requisição HTTP imediatamente, exclui arquivos temporários (`*.download.tmp`), posta mensagem explicativa no chat (`🛑 [Download Cancelado]`) e mantém o fallback ativo.
   - **Notificação de Conclusão / Falha**: Ao atingir 100%, emite confirmação de acoplamento no chat. Em caso de falha de rede, avisa de forma acolhedora.

5. **Navegação Guiada de Modelos por Função Específica no Menu**:
   - Subdividir o menu `Document AI & Modelos Locais` em categorias de propósito claro:
     1. 📂 **Classificação de Documentos (Ingestão)** (Heurística Instantânea vs Laya AI 210MB)
     2. 👁️ **Sentinela de Scans & PDFs Rasterizados** (Heurística 2D vs MobileNetV4 14MB)
     3. 🔍 **Roteamento de Perguntas (Fast vs Deep)** (Determinístico RFC-042 vs BGE-Small 35MB)
     4. 🖼️ **Visão de Documentos (OCR Visual Air-Gapped)** (Nuvem Multimodal vs SmolVLM 480MB vs Moondream2 1.1GB)
     5. 🛡️ **Status de Modelos em Disco & Fallbacks**
   - Guiar o usuário com clareza: explicar para que serve cada função e que nenhum download é obrigatório para o sistema operar perfeitamente.

6. **Onboarding Guiado na Primeira Execução (First-Run Wizard / `/onboarding`)**:
   - Se o AnyContext iniciar pela primeira vez ou se não houver chaves de API / provedores configurados:
     - **Passo 1: 🔑 Provedor de IA & Chaves de API**: Selecionar Gemini / OpenAI / Claude / Local e salvar a chave de forma segura no SQLite nativo.
     - **Passo 2: 🤖 Perfil de Document AI & Visão**:
       - *Perfil Recomendado (Leve & Nuvem)*: 0 MB de download, heurísticas Rust puras (<1µs / 0MB RAM) + visão multimodal da API configurada no Passo 1.
       - *Perfil Air-Gapped (Modelos Locais)*: Oferecer a opção de baixar Laya AI / MobileNetV4 para quem precisa de privacidade 100% offline.
     - **Passo 3: 📂 Pasta Inicial de Documentos (Opcional)**: Registrar a primeira pasta para indexação ou avançar para o chat.

7. **Mensagens Amigáveis e Profissionais no `actx --update` (404 em Releases)**:
   - Identificar 404 de assets recém-lançados causados pelo tempo de compilação do CI/CD (15-40 min).
   - Apresentar mensagem acolhedora e profissional inspirada no GitHub CLI / Rustup, informando que os pacotes estão sendo preparados e oferecendo o link da release para acompanhamento.

---

## 🗺️ 2. Fluxo do Onboarding Wizard

```mermaid
flowchart TD
    Start["actx iniciado pela 1ª vez ou /onboarding"] --> Step1["Passo 1: Provedor & Chave de API\n(Gemini, OpenAI, Claude, Local)"]
    Step1 --> Step2["Passo 2: Perfil Document AI\n(Leve/Nuvem 0MB vs Local Air-Gapped)"]
    Step2 --> Step3["Passo 3: Pasta Inicial\n(Registrar pasta ou pular)"]
    Step3 --> Finish["🎉 Pronto para uso!\nTUI abre com workspace configurado"]
```

---

## 🛡️ 3. Mecanismo de Cancelamento de Download

```mermaid
sequenceDiagram
    actor User as Usuário
    participant App as TUI App
    participant Downloader as Downloader Assíncrono
    participant Disk as Disco Local

    User->>App: Clica em Baixar Modelo (Laya AI)
    App->>Downloader: Inicia stream com CancellationToken
    loop Download em Progresso
        Downloader-->>App: Evento de Progresso (bytes, %)
        App-->>User: Exibe badge: [Baixando Laya: 34%]
    end
    User->>App: Pressiona Esc / Clica Cancelar
    App->>Downloader: Dispara CancellationToken.cancel()
    Downloader->>Disk: Remove laya_int8.onnx.download.tmp
    App-->>User: Chat: "🛑 Download cancelado. Fallback mantido."
```

---

## 📋 4. Plano de Marcos de Implementação

1. **Marco 1: Limpeza Cromática Aurora Boreal**:
   - Migrar `theme.warning` informativos para `theme.accent` (Cyan), `theme.reasoning` (Violet) e `theme.primary` (Emerald).
   - Manter vermelho apenas em `[Esc/Ctrl+C] Exit` e alertas críticos.
2. **Marco 2: Canvas Conversation Borderless**:
   - Remover bordas e título da caixa de chat; adicionar indicador minimalista de scroll pausado.
3. **Marco 3: Auto-Scroll Dinâmico ReAct & Reasoning**:
   - Adicionar controle de scroll em `render_accordion` para acompanhar raciocínio em tempo real.
4. **Marco 4: Mensagem Amigável no `actx --update`**:
   - Interceptar HTTP 404 e exibir explicação empática e profissional estilo GitHub CLI/Rustup.
5. **Marco 5: Telemetria e Cancelamento de Download ONNX**:
   - Stream com porcentagem e velocidade na status bar; suporte a cancelamento gracioso via `Esc` ou menu com deleção de arquivo temporário.
6. **Marco 6: Submenus Guiados por Função no Menu de Modelos**:
   - Submenus de Ingestão, Scans, Consultas e Visão com explicações contextuais de propósito.
7. **Marco 7: Wizard de Onboarding na Primeira Execução & `/onboarding`**:
   - Passo a passo integrado: Provedor/Chave de API -> Perfil de Document AI -> Pasta Inicial.
8. **Marco 8: Testes Automatizados, Dual-Doc & Release v0.34.9**:
   - ADR-117 em `TECDOC.md`, Cenário 32 em `MANUAL_TESTS.md` e compilação do release.
