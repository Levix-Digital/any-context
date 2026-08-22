# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.21.0)

### 📌 Cenário: Unificação de Motor Vetorial 100% LanceDB & Comando `/inspect`

- **Objetivo**: Comprovar que o AnyContext opera exclusivamente sobre o **LanceDB (Apache Arrow / Rust)** com zero dependência de ChromaDB e zero travas SQLite, proporcionando máxima velocidade de busca (< 5ms), re-crawling de portais web e inspeção transparente de chunks via `/inspect`.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.21.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.21.0
   actx
   ```

2. **🔍 Inspecionar o Banco Vetorial Unificado (`/inspect` ou `/chunks`):**
   ```text
   /inspect
   ```
   - **Critério de Sucesso:**
     - Exibe o motor vetorial ativo: `⚡ LanceDB Columnar (Apache Arrow / Rust)`.
     - Exibe a contagem exata de chunks no workspace ativo e total no LanceDB.
     - Exibe a contagem de memórias de sessão no LanceDB.
     - Exibe amostras dos chunks com tipo, caminho e snippet de texto.

3. **🌐 Ingestão de Portal Web com Múltiplas Páginas:**
   ```text
   /web --add https://flyingsquirrelsports.ca/south-calgary-alberta/
   ```
   - **Critério de Sucesso:**
     - O crawler descobre e vetoriza as sub-páginas no LanceDB.
     - O `/inspect` passa a exibir as centenas de chunks gravados.

4. **🧠 Pergunta ao RAG no Modo Strict:**
   ```text
   👤 You: Quais atrações posso encontrar nesse lugar?
   ```
   - **Critério de Sucesso:**
     - Resposta instantânea e precisa detalhando as atrações do parque a partir dos chunks indexados no LanceDB.

---

## 📚 Histórico de Cenários de Testes Anteriores

### 🔬 v0.20.0 - Sincronização Unificada de Fontes (`/sync`) & Paridade de Famílias (`/folder`, `/web`, `/drive`)
- **Validação:** Sincronização unificada de pastas, portais e drives com flags granulares e comandos dedicados.

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
