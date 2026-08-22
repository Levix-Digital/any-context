# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**  
> _Nota: Este arquivo é acumulativo. Todos os cenários passo a passo permanecem completos até que o usuário execute e esvazie o arquivo manualmente._

---

## 🎯 Testes Pendentes de Validação Humana

### 📌 Cenário 1 (v0.22.1): Telemetria em Tempo Real & Micro-Barra de Progresso na Barra de Ferramentas Inferior

- **Objetivo**: Comprovar que durante a sincronização em segundo plano, a barra inferior (`bottom_toolbar`) exibe a micro-barra gráfica de blocos unicode `[████░░░░]` com percentual e contagem atualizada em tempo real (ex: `⚡ Syncing [████░░░░] 50% (15/30 files)`), transitando para `100%` e finalizando suavemente sem travar o teclado.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.22.1`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.22.1
   actx
   ```

2. **⚡ Disparar Sincronização em Background (`/sync`):**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - O prompt `👤 You:` permanece livre e interativo.
     - A barra inferior exibe a micro-barra de progresso com blocos unicode em tempo real:
       `⚡ Syncing [████░░░░] 50% (15/30 files)` (ou `[scanning...]` na fase de descoberta).

3. **💬 Conversar com a IA enquanto o Progresso Atualiza na Barra:**
   ```text
   Olá! Resuma o que você sabe sobre os documentos deste workspace.
   ```
   - **Critério de Sucesso:**
     - A IA responde sem atraso enquanto o percentual e a contagem da micro-barra sobem de forma fluida.
     - Ao término da sincronização (100%), o badge é removido da barra de status.

4. **🌐 Inspeção via REST API e MCP:**
   ```powershell
   # Em outro terminal com actx --serve ativo:
   curl http://127.0.0.1:8000/v1/workspaces/Default/sync/status
   ```
   - **Critério de Sucesso:**
     - Retorna campos estruturados `is_syncing`, `progress` (`pct`, `current`, `total`, `stage`) e `progress_bar`.

---

### 📌 Cenário 2 (v0.22.0): Sincronização Unificada em Background Não-Bloqueante & Status Dinâmico

- **Objetivo**: Comprovar que os comandos de sincronização (`/sync`, `/folder --sync`, `/web --sync`, `/drive --sync`) executam em segundo plano via `BackgroundSyncManager` de forma totalmente desacoplada e não-bloqueante, liberando o prompt de digitação `👤 You:` instantaneamente e exibindo o indicador `⚡ Syncing...` em tempo real na barra de ferramentas inferior (`bottom_toolbar`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.22.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.22.0
   actx
   ```

2. **⚡ Disparar Sincronização em Background (`/sync`):**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - O terminal exibe a mensagem de inicialização imediata:
       `⚡ Background synchronization started for workspace '...' (all sources). You can continue chatting!`
     - O prompt `👤 You:` reaparece **imediatamente** sem travar o teclado.
     - A barra inferior exibe o badge dinâmico: `⚡ Syncing...`.

3. **💬 Conversar com a IA enquanto a Sincronização Ocorre em Paralelo:**
   ```text
   Olá! Quais ferramentas você possui disponíveis?
   ```
   - **Critério de Sucesso:**
     - A IA responde normalmente via streaming enquanto a thread de sincronização trabalha em background.
     - Ao concluir a sincronização, o badge `⚡ Syncing...` é removido suavemente na renderização da barra.

4. **🔍 Sincronização Granular de Fontes Web / Pastas:**
   ```text
   /folder --sync
   /web --sync
   ```
   - **Critério de Sucesso:**
     - Ambas as operações despacham os workers em background sem bloquear o terminal.

---

### 📌 Cenário 2 (v0.21.2): Unificação de Motor Vetorial 100% LanceDB, Sincronização e `/inspect`

- **Objetivo**: Comprovar que o AnyContext opera exclusivamente sobre o **LanceDB (Apache Arrow / Rust)** com zero dependência de ChromaDB e zero travas SQLite, proporcionando máxima velocidade de busca (< 5ms), re-crawling e sincronização de portais web (`/web --sync` e `/sync --web`) e inspeção transparente de chunks via `/inspect`.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.21.2`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**

   ```powershell
   actx --update@0.21.2
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
     - O crawler descobre e vetoriza as centenas de sub-páginas no LanceDB.
     - O `/inspect` passa a exibir as centenas de chunks gravados.

4. **🧠 Pergunta ao RAG no Modo Strict com Web-Search Desligado:**
   ```text
   /mode
   # Selecione: Strict (Audit & Legal)
   /web-search --off
   Quais atrações posso encontrar nesse lugar?
   ```

   - **Critério de Sucesso:**
     - Resposta instantânea e precisa detalhando as atrações do parque a partir dos chunks indexados no LanceDB.

---

### 📌 Cenário 2 (v0.20.0): Sincronização Unificada de Fontes (`/sync`) & Paridade de Famílias (`/folder`, `/web`, `/drive`)

- **Objetivo**: Comprovar que `/sync` orquestra todas as fontes cadastradas no workspace (Pastas Locais + Fontes Web + Drives na Nuvem), suportando flags granulares (`/sync --folder`, `/sync --web`, `/sync --drive`, `/sync --all`, `/sync --force`) e paridade de comandos individuais (`/folder`, `/web`, `/drive`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.20.0` ou superior.

#### 📋 Passo a Passo de Execução:

1. **🔄 Sincronizador Mestre Unificado (`/sync`):**

   ```text
   /sync
   ```

   - **Critério de Sucesso:**
     - O AnyContext sincroniza simultaneamente todas as pastas locais, fontes web e cloud drives do workspace ativo.
     - Nenhuma fonte web é apagada indevidamente durante a checagem de pastas locais.

2. **🌐 Sincronização Específica de Web:**

   ```text
   /sync --web
   ```

   _(ou `/web --sync`)_
   - **Critério de Sucesso:**
     - Sincroniza exclusivamente as fontes web sem reprocessar pastas locais.

3. **📁 Gerenciamento Simétrico de Pastas Locais (`/folder`):**

   ```text
   /folder
   /folder --add C:\Caminho\Para\Documentos
   /folder --sync
   /folder --remove C:\Caminho\Para\Documentos
   ```

   - **Critério de Sucesso:**
     - Lista as pastas locais do workspace.
     - Adiciona a pasta, sincroniza e remove com purga dos vetores correspondentes.

4. **☁️ Gerenciamento Simétrico de Cloud Drives (`/drive`):**
   ```text
   /drive
   /drive --sync
   ```

   - **Critério de Sucesso:**
     - Lista e sincroniza provedores de nuvem configurados.

---

### 📌 Cenário 3 (v0.19.0): Presets de RAG & Injeção de `RetrievalConfig`

- **Objetivo**: Comprovar a alternância dinâmica entre os presets de busca e densidade (**Turbo**, **Balanced**, **Deep Research**).
- **Pré-requisito**: Binário executável do AnyContext com documentos indexados.

#### 📋 Passo a Passo de Execução:

1. **⚙️ Configurar Presets via Menu Interativo:**
   ```text
   /config
   ```

   - Navegue até `Context & RAG Settings` ➔ `Retrieval & Context Density Presets`.
   - Selecione **⚡ Turbo** (Pool 50 | Top 10 | ~5k tokens).
   - Faça uma pergunta no chat e observe resposta rápida e direta.
2. **🔬 Alternar para Deep Research:**
   ```text
   /config
   ```

   - Selecione **🔬 Deep Research** (Pool 150 | Top 40 | ~35k tokens).
   - Faça uma pergunta de comparação entre múltiplos arquivos.
   - **Critério de Sucesso:**
     - O log exibe a busca de 40 chunks e a resposta sintetiza múltiplos documentos simultaneamente.

---

### 📌 Cenário 4 (v0.18.0): Enriquecimento Semântico Contextual & Eliminação de Falsos Positivos

- **Objetivo**: Validar que perguntas sobre termos genéricos em um domínio (ex: _diretrizes para autorização de menores_) não trazem documentos não relacionados (ex: formulários de TI ou relatórios fiscais).
- **Pré-requisito**: Workspace contendo documentos de áreas diferentes.

#### 📋 Passo a Passo de Execução:

1. **📄 Indexar Arquivos de Áreas Diferentes:**
   - Adicione um arquivo sobre imigração/família e outro sobre permissões de rede/TI.
2. **❓ Fazer Pergunta Específica de Domínio:**
   ```text
   Quais são as diretrizes para autorização de menores?
   ```

   - **Critério de Sucesso:**
     - O RAG injeta apenas chunks do documento de menores com o envelope contextual `[Context: Documento | Scope | Keywords]`.
     - Zero trechos de permissões de rede/TI são injetados no prompt.

---

### 📌 Cenário 5 (v0.17.7): Perguntas Compostas Multi-Tópico & Orçamento Proporcional

- **Objetivo**: Comprovar que o algoritmo de orçamento proporcional preserva múltiplos sub-tópicos pesquisados na mesma pergunta sem estourar o limite de tokens da OpenAI.

#### 📋 Passo a Passo de Execução:

1. **❓ Fazer Pergunta Multi-Tópico:**
   ```text
   Quem deve assinar autorizações para crianças e o que as leis do Canadá dizem sobre custódia?
   ```

   - **Critério de Sucesso:**
     - O agente executa buscas para os dois sub-tópicos.
     - Ambos os tópicos permanecem sintetizados na resposta final.
     - Zero erro de estouro de tokens (128k context overflow).
