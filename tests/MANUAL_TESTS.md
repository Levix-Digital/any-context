# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.20.0)

### 📌 Cenário: Sincronização Unificada de Fontes (`/sync`) & Paridade de Famílias (`/folder`, `/web`, `/drive`)

- **Objetivo**: Comprovar que `/sync` orquestra todas as fontes cadastradas no workspace (Pastas Locais + Fontes Web + Drives na Nuvem), suportando flags granulares (`/sync --folder`, `/sync --web`, `/sync --drive`, `/sync --all`, `/sync --force`) e paridade de comandos individuais (`/folder`, `/web`, `/drive`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.20.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.20.0
   actx
   ```

2. **🔄 Sincronizador Mestre Unificado (`/sync`):**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - O AnyContext sincroniza simultaneamente todas as pastas locais, fontes web e cloud drives do workspace ativo.
     - Nenhuma fonte web é apagada indevidamente durante a checagem de pastas locais.

3. **🌐 Sincronização e Gerenciamento Específico de Web:**
   ```text
   /sync --web
   ```
   *(ou `/web --sync`)*
   - **Critério de Sucesso:**
     - Sincroniza exclusivamente as fontes web sem reprocessar pastas locais.

4. **📁 Gerenciamento Simétrico de Pastas Locais:**
   ```text
   /folder
   ```
   - **Critério de Sucesso:**
     - Lista as pastas locais do workspace.
     - Suporta `/folder --sync`, `/folder --add <path>` e `/folder --remove <path>`.

---

## 📚 Histórico de Cenários de Testes Anteriores

### 🔬 v0.19.0 - Motor Vetorial Paralelo LanceDB & Injeção de Presets de RAG
- **Validação:** Armazenamento colunar em Rust, buscas sub-5ms e injeção de dependência de `RetrievalConfig`.

### 🔬 v0.18.0 - Enriquecimento Semântico Contextual & Eliminação de Falsos Positivos
- **Validação:** Documentos recebem envelope com sumário rico e keywords, eliminando falsos positivos entre domínios diferentes.

### 🔬 v0.17.7 - Perguntas Compostas Multi-Tópico & Orçamento Proporcional
- **Validação:** Perguntas com múltiplos tópicos simultâneos preservam trechos de todos os tópicos no prompt sem estouro de 128k tokens.

### 🔬 v0.17.3 - Busca Paralela Multi-Fonte (`ThreadPoolExecutor`) & Presets de RAG
1. **Configuração de Presets:**
   - Execute `/config` ➔ Navegue até `Context & RAG Settings` ➔ Alterne entre **Turbo**, **Balanced** e **Deep Research**.
2. **Execução de Busca Paralela:**
   - No chat, digite uma pergunta abrangente: observe no log a mensagem `🔍 [Search] Parallel Multi-Source Scan: 'Workspace' (pool: 100 -> diversified top 20)...`.
   - Tempo de resposta: Fusão vetorial em sub-15ms na CPU.

### 📦 v0.17.1 - Scripts de Instalação e Desinstalação Automáticos
1. **Verificação de Assets no GitHub:**
   - Acessar `https://github.com/Levix-Digital/any-context/releases/latest` e validar que `install.ps1`, `install.sh`, `uninstall.ps1` e `uninstall.sh` estão anexados ao lado dos executáveis.
