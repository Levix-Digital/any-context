# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.18.0)

### 📌 Cenário: Enriquecimento Semântico Contextual & Eliminação de Falsos Positivos

- **Objetivo**: Comprovar que o `ContextualEnricher` ancora os chunks ao tema macro e palavras-chave de cada documento/URL, impedindo que documentos irrelevantes de outros domínios (ex: finanças, TI) apareçam em buscas de imigração ou autorizações.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.18.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.18.0
   actx
   ```

2. **🔄 Sincronizar Workspace com Enriquecimento Contextual:**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - A sincronização conclui com sucesso.
     - Cada arquivo e página web recebe seu envelope com **Sumário Rico** e **Top-N Palavras-Chave** (armazenados em cache SQLite persistente).

3. **💬 Turno 1 (Busca Semântica Específica com Termos Genéricos):**
   ```text
   👤 You: Quem deve assinar autorizações para crianças? O que as leis do Canadá dizem sobre isso?
   ```
   - **Critério de Sucesso:**
     - **100% de Precisão Temática**: Todos os chunks recuperados e citados pertencem estritamente aos documentos de imigração/menores (`Regras_Menores`, `Canada.ca`).
     - **Zero Falsos Positivos**: Nenhum arquivo financeiro, de RH ou de TI aparece nos chunks recuperados.
     - Os cabeçalhos de busca exibem `Keywords: ...` e o tema associado.

4. **💬 Turno 2 (Verificação de Sub-30ms em Arquivos Inalterados):**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - O cache SHA-256 é acionado instantaneamente sem reprocessar sumários para arquivos inalterados.

---

## 📚 Histórico de Cenários de Testes Anteriores

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
