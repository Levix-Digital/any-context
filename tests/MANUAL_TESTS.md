# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.19.0)

### 📌 Cenário: Motor Vetorial Paralelo LanceDB & Injeção de Presets de RAG

- **Objetivo**: Comprovar que o novo motor vetorial `LanceDBStore` e `ParallelRetriever` realizam buscas colunares em sub-milissegundos (< 5ms), com filtragem pura desacoplada (`RelevanceFilter`) e injeção de dependência de regras de RAG (`RetrievalConfig`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.19.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.19.0
   actx
   ```

2. **🔄 Sincronização e Indexação Colunar:**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - A sincronização grava os vetores em formato colunar Apache Arrow no LanceDB.
     - Velocidade de processamento e persistência visivelmente instantânea.

3. **💬 Turno 1 (Busca Vetorial Ultrarrápida no LanceDB com Filtro de Relevância):**
   ```text
   👤 You: Quais são as diretrizes de viagem para menores no Canadá?
   ```
   - **Critério de Sucesso:**
     - Resposta sai em menos de 2 segundos.
     - `RelevanceFilter` corta qualquer ruído com similaridade fraca e balanceia as fontes via round-robin.

4. **🎛️ Turno 2 (Alternância Dinâmica de Presets via `/rag` ou `/config`):**
   ```text
   /rag turbo
   ```
   - **Critério de Sucesso:**
     - O sistema injeta o `RetrievalConfig` do preset **Turbo** (pool=50, top_k=10).
     - A próxima pergunta executa com orçamento super conciso de ~5.000 tokens.

---

## 📚 Histórico de Cenários de Testes Anteriores

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
