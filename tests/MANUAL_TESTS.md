# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**  
> _Nota: Este arquivo é acumulativo. Todos os cenários passo a passo permanecem completos até que o usuário execute e esvazie o arquivo manualmente._

---

## 🎯 Testes Pendentes de Validação Humana

### 📌 Cenário 1 (v0.28.90): Blindagem Dual-Binary no Linux/WSL, Prevenção do Destroyer Bug no Update e Isolamento de Artefatos no CI/CD

- **Objetivo**: Comprovar que na release `v0.28.90`:
  1. No Linux/WSL, a atualização via `actx --update` ou `/update` preserva rigorosamente o motor central `actx-core` (45MB) em `~/.local/bin/actx-core` e o launcher nativo compilado `actx` (16KB) em `~/.local/bin/actx`, eliminando por completo o "Destroyer Bug" em que o motor era sobrescrito pelo wrapper bash.
  2. O comando `actx` no Linux/WSL abre diretamente o OpenTUI (ou executa os comandos solicitados) sem jamais sair silenciosamente com código 0.
  3. O launcher nativo ELF (`actx`) é reconhecido pela assinatura de magic bytes `\x7fELF` e preservado contra sobrescrita com scripts bash.
  4. Caso o motor `actx-core` esteja ausente no disco, o wrapper bash emite diagnóstico explícito no `stderr` (`❌ AnyContext core engine not found at $BIN_DIR/actx-core`) e encerra com código 1, em vez de sair silenciosamente com 0.
  5. No GitHub Actions (`release.yml`), os artefatos de Linux e Windows são baixados em pastas temporárias isoladas (`dist-linux` e `dist-windows`), garantindo que o asset de release `actx` publicado no GitHub Releases seja o executável nativo ELF de Linux, sem ser corrompido pelo wrapper de Git Bash do Windows.
- **Pré-requisito**: Versão `v0.28.90` instalada no Linux/WSL (`actx -v` exibindo `v0.28.90`).

#### 📋 Passo a Passo de Execução:

1. **🐧 Verificação de Binários Dual-Binary no Linux/WSL:**
   - No terminal Linux ou WSL (`levi@Home-Desktop`), executar:
     ```bash
     ls -la ~/.local/bin/actx*
     ```
   - **Critério de Aceitação:**
     - O arquivo `~/.local/bin/actx-core` existe e possui tamanho > 40MB com permissão de execução (`-rwxr-xr-x`).
     - O arquivo `~/.local/bin/actx` existe e possui permissão de execução.
     - A pasta `~/.local/bin/_internal` existe com os pacotes e bibliotecas compartilhadas do Python.

2. **⚡ Execução de Comandos Rápidos e Diagnósticos no Linux:**
   - No terminal Linux/WSL, executar:
     ```bash
     actx -v
     actx --diagnostics
     ```
   - **Critério de Aceitação:**
     - `actx -v` retorna imediatamente `v0.28.90`.
     - `actx --diagnostics` imprime o relatório completo de saúde do sistema, indicando SO Linux, Executable Binary `/home/.../.local/bin/actx-core` e Bun runtime detectado com sucesso.
     - Nenhum comando sai em branco ou silencioso.

3. **🖥️ Execução Interativa do OpenTUI no Linux/WSL:**
   - No terminal Linux/WSL, digitar:
     ```bash
     actx
     ```
   - **Critério de Aceitação:**
     - A interface reativa do OpenTUI abre em tela cheia diretamente no terminal Linux.
     - Comandos como `/sources`, `/mode` e `/help` respondem normalmente via Stdio RPC Bridge.
     - Ao sair com `/exit`, a tela é restaurada perfeitamente ao shell.

4. **🔄 Validação do Mecanismo de Atualização Sem Perda de `actx-core`:**
   - Disparar a verificação de atualização:
     ```bash
     actx --update
     ```
   - **Critério de Aceitação:**
     - O serviço de update identifica corretamente a versão mais recente.
     - Caso force a reinstalação, todos os arquivos dual-binary são roteados adequadamente: `actx-core` para `~/.local/bin/actx-core` e o shim para `~/.local/bin/actx`.
     - O executável `actx-core` permanece intacto e funcional após o ciclo de atualização.

---

### 📌 Cenário 1 (v0.28.89): Validação do OpenTUI como Interface Padrão do `actx`, TUI 100% Thin Client via RPC e Remoção do Legado `chat_loop.py`

- **Objetivo**: Comprovar que na release `v0.28.89`:
  1. O comando `actx` disparado no terminal interativo abre imediatamente o **OpenTUI** como interface padrão oficial sem necessidade de passar a flag `--tui`.
  2. O arranque é limpo e contínuo: não há double clears, flashes de banner ANSI antigo ou atrasos antes da renderização reativa do OpenTUI.
  3. O front-end OpenTUI funciona como um cliente 100% "burro" (Thin Client): todo comando executado (`/mode`, `/switch`, `/model`, `/update`, `/menu`, `/onboarding`, etc.) é despachado diretamente ao Python Core via RPC Bridge (`client.executeCommand`), retornando ações declarativas para abertura de modais com os mesmos payloads e comportamento que serão consumidos pelo futuro Tauri Desktop.
  4. Execuções headless (`actx "pergunta"`, `actx -p "..."`, stdin pipes `cat doc.txt | actx "resuma"`, `actx -v`, `actx --rpc`, `actx --diagnostics`) continuam ultrarrápidas em sub-segundo sem instanciar o Bun ou a TUI.
  5. O arquivo legado `chat_loop.py` foi 100% removido do repositório com zero regressões no sistema e todas as utilidades necessárias migradas para `src/any_context/cli/utils.py`.
- **Pré-requisito**: Versão `v0.28.89` instalada (`actx -v` exibindo `v0.28.89`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Inicialização Padrão Direta do OpenTUI (`actx`):**
   - No terminal (PowerShell, CMD, Windows Terminal ou Bash), digitar simplesmente:
     ```bash
     actx
     ```
   - **Critério de Aceitação:**
     - O OpenTUI inicializa diretamente em tela cheia com sua barra de status inferior de 1 linha, banner ASCII Art clássico e árvore de telemetria progressiva (`┌─ ⚡ Engine Startup Telemetry`).
     - Nenhum banner ANSI duplicado ou tela preta piscando antes do OpenTUI.
     - Não foi necessário digitar `--tui`.

2. **⚡ Validação da Arquitetura Thin Client (RPC Despacho de Modais):**
   - Dentro do OpenTUI, digitar e pressionar Enter para cada um dos seguintes comandos:
     - `/mode`: O modal interativo de seleção de Grounding Mode deve abrir imediatamente.
     - `/switch`: O modal interativo de troca/criação de workspaces deve abrir.
     - `/model`: O modal de seleção de provedores e modelos deve abrir.
     - `/menu`: O modal interativo de configuração deve abrir.
     - `/onboarding`: O modal wizard de setup inicial deve abrir.
   - **Critério de Aceitação:**
     - Todos os modais abrem com resposta instantânea.
     - Ao inspecionar os logs (`actx --logs`), comprovar que os eventos foram recebidos e processados pelo `CommandDispatcher` no Python Core, retornando ações declarativas (`open_mode_modal`, `open_switch_modal`, etc.) para o front-end.

3. **🤖 Validação de Paridade de Mensagens Conversacionais:**
   - Enviar uma mensagem conversacional qualquer (ex: `"Qual a versão do AnyContext?"`).
   - **Critério de Aceitação:**
     - O agente LangGraph processa a mensagem via Stdio RPC Bridge com streaming reativo em tempo real token por token no OpenTUI.

4. **⚡ Validação de Execução Headless e Unix Pipes:**
   - Sair do OpenTUI digitando `/exit` ou pressionando `Ctrl+C`.
   - Testar o comando one-shot no terminal:
     ```bash
     actx "Diga apenas: 'Headless OK'"
     ```
   - **Critério de Aceitação:**
     - A resposta é impressa diretamente no stdout do terminal sem abrir o OpenTUI.
   - Testar pipe de entrada:
     ```bash
     echo "Linha de teste para sumário" | actx "Quantas linhas existem neste texto?"
     ```
   - **Critério de Aceitação:**
     - O streaming responde no stdout e finaliza com código 0.

5. **🔍 Validação de Versão e Ausência do `chat_loop.py`:**
   - Verificar versão:
     ```bash
     actx -v
     ```
   - **Critério de Aceitação:** Exibe estritamente `v0.28.89` em `< 50ms`.

---

### 📌 Cenário 2 (v0.28.88): Validação de Inicialização Sub-Segundo (--onedir) e Normalização de Versão sem BOM no Git Bash/PowerShell

- **Objetivo**: Comprovar que na release `v0.28.88`:
  1. O tempo de inicialização a frio do runtime Python (`⚡ Python Core runtime linked`) cai de ~2.8s para `< 0.2s`, eliminando a descompressão repetitiva de 250MB em `%TEMP%` graças à arquitetura `--onedir` com runtime pré-extraído em `%LOCALAPPDATA%\actx\bin\_internal`.
  2. O comando `actx -v` executado no Git Bash / MSYS2, CMD e PowerShell exibe exatamente `v0.28.88` com apenas um `v` na frente, comprovando a erradicação do UTF-8 BOM (`\xef\xbb\xbf`) e a higienização em todas as camadas de shim (C#, C e wrapper Bash).
  3. O instalador (`install.ps1` e `install.sh`) e o atualizador interativo (`/update` e `actx --update`) descarregam o pacote distribuído (`.zip` no Windows, `.tar.gz` no Linux), realizam a extração atômica via `actx_staging/` e atualizam `_internal/` e binários sem locks de arquivo.
- **Pré-requisito**: Versão `v0.28.88` instalada (`actx -v` exibindo `v0.28.88`).

#### 📋 Passo a Passo de Execução:

1. **🔍 Validação de Versão sem Duplo 'v' no Git Bash e PowerShell:**
   - Abrir o terminal **Git Bash** (MinTTY / MINGW64) e digitar:
     ```bash
     actx -v
     ```
   - **Critério de Aceitação:** A saída no terminal deve ser estritamente:
     ```text
     v0.28.88
     ```
     (Não deve exibir `vv0.28.88` sob hipótese alguma).
   - Repetir a consulta no PowerShell ou CMD do Windows:
     ```powershell
     actx -v
     ```
   - **Critério de Aceitação:** Exibe idêntico `v0.28.88`.

2. **⚡ Validação da Telemetria de Inicialização Instantânea (Sub-Segundo):**
   - No terminal, iniciar uma sessão do AnyContext:
     ```bash
     actx
     ```
   - Ou na interface desktop OpenTUI:
     ```bash
     actx --tui
     ```
   - Observar a árvore inicial `┌─ ⚡ Engine Startup Telemetry`:
     ```text
       ┌─ ⚡ Engine Startup Telemetry
       │ ├─ [ 48.3ms] ⚡ Python Core runtime linked
       │ ├─ [ 12.1ms] 🔌 SQLite Configuration Store active
       │ ├─ [ 82.2ms] 🤖 AI Model engine linked (gpt-4o-mini - OPENAI)
       │ ├─ [ 82.2ms] 📂 Workspace connected (Default)
       │ ├─ [152.8ms] 📦 Context state verified (Up to date - 3 files)
       │ └─ [152.8ms] 🚀 AnyContext ready in 0.15s
     ```
   - **Critério de Aceitação:**
     - O primeiro marco `⚡ Python Core runtime linked` conclui em **menos de 200ms** (comparado aos 2.77s - 3.20s anteriores com `--onefile`).
     - A inicialização total do AnyContext ocorre em fração de segundo (`< 0.5s`).

3. **📁 Validação da Estrutura de Arquivos em Disco (`_internal/`):**
   - Verificar no PowerShell o diretório de instalação do AnyContext:
     ```powershell
     Test-Path "$env:LOCALAPPDATA\actx\bin\_internal"
     Get-Item "$env:LOCALAPPDATA\actx\bin\actx-core.exe"
     ```
   - **Critério de Aceitação:** O diretório `_internal` existe no disco, contendo as bibliotecas e dependências Python pré-extraídas, permitindo que o sistema operacional as carregue diretamente sem sobrecarregar o diretório `%TEMP%`.

---

### 📌 Cenário 1 (v0.28.87): Validação do Diálogo Interativo Unificado de Atualização (/update) com Tríade Consistente e Encerramento Limpo

- **Objetivo**: Comprovar que na release `v0.28.87`:
  1. O comando `/update` no OpenTUI Desktop (`actx --tui` ou `actx`) e no Terminal CLI apresenta **sempre as 3 opções universais**, sem suprimir a opção de fechar a sessão quando apenas 1 instância estiver em execução.
  2. Com apenas a sessão atual em primeiro plano (`count == 0`), a opção de encerramento exibe o título adaptado no singular:
     `⏹️ Close session and update now`
     E descrição:
     `Terminates this session and exits cleanly to terminal with vX.Y.Z.`
  3. A opção de atualização em segundo plano exibe com transparência:
     `⚡ Update in background (Recommended)`
     `Active session(s) continue undisturbed; applies on your next launch.`
  4. A opção de cancelamento `🔙 Cancel update` aborta o processo e retorna ao chat imediatamente.
  5. Ao escolher a opção `⏹️ Close session and update now`:
     - O sistema realiza o download do binário atômico.
     - Notifica o encerramento da sessão com a contagem regressiva de 800ms.
     - Restaura o terminal para o modo canônico (cooked mode), limpa buffers e devolve o prompt do sistema operacional pronto para executar `actx` na nova versão.
- **Pré-requisito**: Versão `v0.28.87` instalada (`actx -v` exibindo `v0.28.87`).

#### 📋 Passo a Passo de Execução:

1. **🚀 Abertura do AnyContext e Disparo do `/update`:**
   - No terminal, inicie o AnyContext:
     ```bash
     actx
     ```
   - No campo de entrada de mensagens (`👤 You:`), digite o comando e pressione `Enter`:
     ```text
     /update
     ```
   - **Critérios de Aceitação:**
     - Uma janela modal de seleção interativa é renderizada no centro da tela.
     - O cabeçalho exibe: `🚀 AnyContext Update Available: v0.28.87 → ...` ou a versão detectada.
     - São listadas **exatamente 3 opções**:
       1. `⚡ Update in background (Recommended)  [Recommended]`  
          `Active session(s) continue undisturbed; applies on your next launch.`
       2. `⏹️ Close session and update now` (ou `Close all AnyContext sessions and update now` se houver outros processos)  
          `Terminates this session and exits cleanly to terminal with vX.Y.Z.`
       3. `🔙 Cancel update`  
          `Aborts the update process and returns to chat.`
     - A navegação pelas opções funciona suavemente via setas `↑` e `↓`.

2. **🔙 Teste de Cancelamento:**
   - Com as setas, selecione a opção `🔙 Cancel update` e pressione `Enter`.
   - **Critérios de Aceitação:**
     - O modal fecha instantaneamente.
     - Uma mensagem informativa surge no chat indicando que a atualização foi cancelada pelo usuário.
     - O prompt `👤 You:` permanece 100% responsivo para novos comandos.

3. **⏹️ Teste de Fechamento Limpo (Opcional quando houver nova versão disponível):**
   - Ao executar `/update` e escolher `⏹️ Close session and update now`, confirme que a aplicação exibe a mensagem de finalização, encerra o TUI em 800ms e retorna ao prompt do terminal sem artefatos visuais ou travamentos de cursor.
   - Digite `/exit` para finalizar o teste.

---

### 📌 Cenário 2 (v0.28.86): Telemetria de Inicialização em Tempo Real na TUI com Efeito Cascata Suave (Abordagem B) e Paridade com Terminal CLI

- **Objetivo**: Comprovar que na release `v0.28.86`:
  1. Abertura do AnyContext na interface interativa TUI (`actx` ou `actx --tui`) exibe **imediatamente desde o primeiro milissegundo** a árvore `┌─ ⚡ Engine Startup Telemetry` com animação de spinner braille (`⠋`, `⠙`, `⠹`, etc.).
  2. A caixa estática amarela de espera (`⏳ Initializing AnyContext AI Core...`) foi **completamente eliminada**.
  3. Cada subsistema (Runtime Python, SQLite Store, Motor de Modelo de IA, Conexão do Workspace e Verificação do Índice Vetorial) progride visualmente com efeito cascata fluido (~70ms de cadência mínima visual - Abordagem B), estampando o tempo real medido em milissegundos (`[ 42.0ms]`, `[ 64.7ms]`, etc.).
  4. Ao finalizar a inicialização, a última linha `└─ [ X.XXs ] 🚀 AnyContext ready in X.XXs` fecha a árvore com destaque verde e o card `💬 Chat started! Type '/' for quick commands...` surge imediatamente abaixo.
  5. A árvore de telemetria concluída permanece elegantemente fixada no topo do histórico de mensagens da conversa (atingindo 100% de paridade estética com o banner do Terminal CLI).
  6. O prompt `👤 You:` torna-se 100% responsivo para digitação e comandos slash (`/`).
- **Pré-requisito**: Versão `v0.28.86` instalada (`actx -v` exibindo `v0.28.86`).

#### 📋 Passo a Passo de Execução:

1. **🚀 Inicialização da TUI e Observação do Boot:**
   - No terminal (PowerShell ou Windows Terminal), inicie o AnyContext:
     ```bash
     actx
     ```
   - **Critérios de Aceitação:**
     - A tela abre imediatamente. Em vez de uma caixa amarela estática, surge logo abaixo do banner ASCII a árvore:
       ```text
         ┌─ ⚡ Engine Startup Telemetry
         │ ├─ [ ⠋ ] ⚡ Initializing Python Core runtime & dependencies...
       ```
     - O spinner braille gira ativamente enquanto o processo filho em segundo plano carrega as bibliotecas de IA.
     - Assim que o motor conecta, a árvore desce em cascata suave, exibindo cada subsistema em verde com seus milissegundos:
       ```text
         ┌─ ⚡ Engine Startup Telemetry
         │ ├─ [  3.20s] ⚡ Python Core runtime linked
         │ ├─ [ 42.0ms] 🔌 SQLite Configuration Store active
         │ ├─ [ 64.7ms] 🤖 AI Model engine linked (gpt-4o-mini - OPENAI)
         │ ├─ [ 64.8ms] 📂 Workspace connected (Default)
         │ ├─ [134.1ms] 📦 Context state verified (Up to date - 3 files)
         │ └─ [  3.50s] 🚀 AnyContext ready in 3.50s
       ```

2. **💬 Confirmação de Prontidão e Persistência no Chat:**
   - Observe a área logo abaixo da telemetria.
   - **Critérios de Aceitação:**
     - O box arredondado `💬 Chat started! Type '/' for quick commands...` aparece logo abaixo da telemetria de inicialização.
     - A árvore de telemetria **não desaparece**: ela permanece no topo da lista de mensagens como histórico de inicialização do motor.
     - O prompt `👤 You:` está ativo e com cursor piscando, pronto para receber perguntas ou comandos como `/switch`, `/model`, `/sync` e `/menu`.
     - Digite `/exit` para sair limpo.

---

### 📌 Cenário 3 (v0.28.85): Validação de Imunidade Absoluta do Banco de Produção, Barreira de Segurança e Cascade Purge de Workspaces de Teste

- **Objetivo**: Comprovar que na release `v0.28.85`:
  1. A execução de testes unitários diretamente (ex: `python -m unittest tests/unit/core/test_workspace_sources.py` ou `test_grounding_modes.py`) NUNCA polui o banco de produção do usuário (`%LOCALAPPDATA%\AnyContext\config\settings.db`).
  2. Qualquer workspace residual de testes passados (`Unit_Dispatch_WS`, `TestWS`, `RpcUnitTestWS`, `NewRPCWS`, `RenderTestWS`) é automaticamente purgado em cascata na inicialização em modo de produção (incluindo todas as tabelas filhas `workspace_web_urls`, `workspace_folders`, etc.).
  3. Workspaces criados pelo usuário (`created_by = 'user'`) e do sistema (`Default`, `Shared Sources`) são 100% imunes a qualquer limpeza.
  4. Na interface TUI ou CLI, o modal de troca de workspaces exibe apenas os workspaces reais do usuário e do sistema, sem nenhum workspace fantasma ou temporário de testes.
- **Pré-requisito**: Versão `v0.28.85` instalada (`actx -v` exibindo `v0.28.85`).

#### 📋 Passo a Passo de Execução:

1. **🚀 Execução Direta de Testes Unitários de Fontes e Despacho:**
   - Execute o teste unitário que manipulava pastas temporárias e portais web:
     ```bash
     python -m unittest tests/unit/core/test_workspace_sources.py
     python -m unittest tests/unit/core/test_grounding_modes.py
     ```
   - **Critério de Aceitação:** Ambos os testes passam com `OK`. A barreira de segurança em `paths.py` e o isolamento de `ACTX_SETTINGS_DB` garantem que o banco de produção nunca seja tocado.

2. **🔍 Auditoria Forense no Banco de Produção do Usuário:**
   - No terminal (PowerShell ou Bash), execute a checagem rápida de integridade do banco:
     ```bash
     python -c "import sqlite3, os; db = os.path.expandvars(r'%LOCALAPPDATA%\AnyContext\config\settings.db'); conn = sqlite3.connect(db); print([r[1] for r in conn.cursor().execute('SELECT id, name, created_by FROM workspaces').fetchall()])"
     ```
   - **Critério de Aceitação:** A lista de workspaces exibe estritamente `['Default', 'Shared Sources', ...]` (e workspaces reais criados por você, como `RustBook`). Workspaces de teste como `Unit_Dispatch_WS` e `TestWS` **não existem**.

3. **🖥️ Verificação Visual na Interface OpenTUI (`/switch`):**
   - Inicie o AnyContext na interface interativa:
     ```bash
     actx --tui
     ```
   - Digite `/switch` e pressione `[Enter]` para abrir o modal de workspaces:
     ```text
     /switch
     ```
   - **Critério de Aceitação:**
     - O modal lista apenas `Default`, `Shared Sources` e os workspaces criados pelo usuário.
     - Nenhum workspace com pastas temporárias em `Temp/tmpxjkelzz9` ou `Unit_Dispatch_WS` ou `TestWS` aparece na lista.
     - Pressione `[Esc]` para fechar o modal.

---

### 📌 Cenário 4 (v0.28.83): Atualização com Fechamento Seguro de Todas as Sessões, Teardown Limpo de Terminal e Prevenção de Prompt Fantasma

- **Objetivo**: Comprovar que na release `v0.28.83`:
  1. No comando `/update` (ou `/update@<versão>`), a opção de encerramento fecha com segurança **todas** as instâncias AnyContext ativas no sistema operacional.
  2. O download do novo binário é realizado e validado **antes** do encerramento de qualquer sessão, prevenindo fechamento indevido caso haja falha de rede.
  3. No OpenTUI, o encerramento realiza o teardown completo do terminal (`renderer.destroy()`, desativação de Raw Mode, restauração do buffer canônico de entrada e cursor visível).
  4. O terminal não apresenta colisão visual ("TUI fantasma" com prompt vazado sobre a interface gráfica).
  5. Ao encerrar a sessão de update, o controle retorna 100% responsivo para o shell com a mensagem informativa orientando o usuário a executar `actx`.
  6. Comandos e atalhos de terminal (Enter, Ctrl+C, digitação normal) permanecem 100% funcionais imediatamente após a saída do update.
- **Pré-requisito**: Versão `v0.28.83` instalada (`actx -v` exibindo `v0.28.83`).

#### 📋 Passo a Passo de Execução:

1. **🚀 Início de Sessão Interativa e Abertura do Modal de Update:**
   - Inicie o AnyContext na interface OpenTUI:
     ```bash
     actx --tui
     ```
   - No prompt de chat, digite `/update`:
     ```text
     /update
     ```
   - **Critério de Aceitação:** O modal de opções de atualização é exibido com o título e opções atualizadas:
     - `⚡ Update in background (Recommended)`
     - `⏹️ Close all AnyContext sessions and update now`
     - `🔙 Cancel update`

2. **⏹️ Execução de Update com Fechamento de Sessões:**
   - Selecione a opção `⏹️ Close all AnyContext sessions and update now` e pressione `[Enter]`.
   - **Critério de Aceitação:**
     - O download do binário é concluído e verificado.
     - A interface exibe: `🎉 AnyContext updated! Closing session and returning to terminal...`.
     - O OpenTUI faz o teardown completo do console e fecha suavemente em menos de 1 segundo.
     - O terminal volta para o prompt do seu shell (Git Bash, PowerShell ou Zsh) de forma totalmente limpa.
     - É exibida a mensagem final:
       ```text
       🎉 AnyContext foi atualizado com sucesso para v0.28.83!
       👉 Execute 'actx' para iniciar a nova versão.
       ```

3. **⌨️ Validação da Responsividade do Terminal (Eliminação do Freeze / Raw Mode Lock):**
   - No prompt do shell imediatamente após a saída, digite comandos normais:
     ```bash
     echo "teste de terminal responsivo"
     actx -v
     ```
   - Teste também pressionar `Ctrl+C` no prompt.
   - **Critério de Aceitação:**
     - O terminal ecoa os caracteres normalmente.
     - `actx -v` responde exibindo a versão atualizada instantaneamente (< 10ms).
     - O terminal responde a `Ctrl+C` e a novas linhas sem travar, confirmando que o modo Raw foi desativado e o modo canônico foi 100% restaurado.

---

### 📌 Cenário 2 (v0.28.82): Normalização Automática de Redirecionamento Canônico HTTP (Trailing Slash) e Crawling Completo de Portais Web

- **Objetivo**: Comprovar que na release `v0.28.82`:
  1. A adição ou sincronização de portais de documentação sem a barra final (ex: `https://doc.rust-lang.org/stable/book`) detecta dinamicamente o redirecionamento canônico HTTP (`301 Moved Permanently` -> `.../stable/book/`).
  2. O crawler utiliza a URL efetiva pós-redirect como `base_url` para o `HTMLLinkExtractor`, garantindo que links relativos (como `ch01-00-getting-started.html`) resolvam corretamente para o subdiretório `/stable/book/...` em vez de truncar para `/stable/...` (o que causava HTTP 404 e indexação de apenas 1 página no Linux/WSL).
  3. No Linux/WSL e no Windows, a sincronização do workspace indexa todas as 44 páginas da documentação no LanceDB.
  4. A conciliação no SQLite (`workspace_web_urls`) atualiza o registro semente com a contagem total de páginas (44 páginas) e data de varredura sem criar duplicatas no banco de dados.
  5. Perguntas feitas no chat sobre tópicos da documentação (ex: *"O que são variáveis mutáveis?"*) respondem com precisão e citam as fontes legítimas indexadas.
- **Pré-requisito**: Versão `v0.28.82` instalada (`actx -v` exibindo `v0.28.82`).

#### 📋 Passo a Passo de Execução:

1. **🌐 Sincronização Completa do Portal Web (Linux/WSL ou Windows):**
   - No terminal, execute a sincronização do workspace `RustDoc` (ou adicione a URL sem barra final em um novo workspace):
     ```bash
     actx /sync
     ```
   - **Critério de Aceitação:** O crawler executa a descoberta, identifica as páginas filhas da seção e realiza o download e indexação vetorial completa. A contagem de páginas salvas atinge 44 páginas.

2. **🔍 Verificação da Quantidade de Páginas e Chunks no LanceDB:**
   - Inspecione a contagem de páginas e chunks indexados:
     ```bash
     python -c "from any_context.vector_engine.store import LanceDBStore; lance = LanceDBStore(); print('Total de chunks:', lance.count_records('RustDoc'))"
     ```
   - **Critério de Aceitação:** O total de chunks é superior a 1.000 (ex: ~1.148 chunks correspondentes aos 44 capítulos), comprovando que todas as páginas filhas foram descarregadas e vetorizadas.

3. **💬 Validação de Pergunta e Citação em Linguagem Natural:**
   - Abra a CLI interativa no workspace `RustDoc`:
     ```bash
     actx --tui
     ```
   - Envie a seguinte pergunta:
     ```text
     O que são variáveis mutáveis em Rust?
     ```
   - **Critério de Aceitação:** O assistente responde explicando o conceito de mutabilidade em Rust (ex: uso da palavra-chave `mut`, imutabilidade por padrão), citando explicitamente o capítulo correspondente (*Chapter 3 - Variables and Mutability*).

---

### 📌 Cenário 2 (v0.28.81): Blindagem Hermética de Testes e Imunidade da Base Vetorial de Produção

- **Objetivo**: Comprovar que na release `v0.28.81`:
  1. A execução de qualquer suíte de testes automatizados (`python tests/run_all.py` ou unit tests) opera sob sandbox estrito e nunca apaga, contamina ou purga a base vetorial real do usuário (`%LOCALAPPDATA%\AnyContext\data\context_db\lancedb`).
  2. As variáveis `ACTX_CONTEXT_DB` e `ACTX_MEMORY_DB` isolam completamente os dados de contexto e memória durante a execução dos testes.
  3. A função `clear_context_vector_db()` possui trava de segurança nativa que aborta imediatamente caso detecte ambiente de testes tentando limpar a pasta de produção.
  4. Workspaces existentes com fontes web ou pastas (ex: `RustBook` com 44 páginas do Rust Book) mantêm todos os seus vetores intactos após qualquer execução de testes ou atualização do sistema.
  5. Consultas de RAG respondem perfeitamente com citações legítimas das fontes indexadas.
- **Pré-requisito**: Versão `v0.28.81` instalada (`actx -v` exibindo `v0.28.81`).

#### 📋 Passo a Passo de Execução:

1. **🛡️ Validação da Integridade e Imunidade dos Chunks de Produção:**
   - Abra um terminal e consulte a contagem de chunks vetoriais do workspace `RustBook`:
     ```powershell
     python -c "from any_context.vector_engine.store import LanceDBStore; lance = LanceDBStore(); print('Chunks no RustBook:', lance.count_records('RustBook'))"
     ```
   - **Critério de Aceitação:** O comando exibe `Chunks no RustBook: 1148` (ou o número de chunks das suas fontes), comprovando que os vetores estão 100% preservados.

2. **🧪 Validação da Execução de Testes com Sandbox Hermético:**
   - Execute o teste unitário de imunidade de produção:
     ```powershell
     python -m unittest tests/unit/core/test_production_immunity.py
     ```
   - **Critério de Aceitação:** Todos os 3 testes passam com sucesso (`OK`), confirmando que qualquer tentativa de acessar `%LOCALAPPDATA%` em modo de teste é bloqueada e desviada para diretórios temporários.

3. **💬 Validação de Pergunta e Citação em Linguagem Natural:**
   - Inicie a TUI ou CLI no workspace `RustBook`:
     ```bash
     actx --tui
     ```
   - Pergunte:
     ```text
     Me fale um pouco sobre o que é Rust e me dê um código mínimo de exemplo
     ```
   - **Critério de Aceitação:** O assistente responde com precisão técnica e cita as fontes oficiais consultadas (ex: `The Rust Programming Language`, `Hello, World!`).

---

### 📌 Cenário 2 (v0.28.80): LanceDB Single Source of Truth para Fontes Web, Resync Rápido e Eliminação de Split-Brain

- **Objetivo**: Comprovar que na release `v0.28.80`:
  1. O banco de dados vetorial LanceDB é 100% a única fonte de verdade (Single Source of Truth) para o estado de páginas web indexadas, contagens e metadados.
  2. A tabela redundante `workspace_indexed_web_pages` do SQLite foi completamente removida e descartada via migração automática (`DROP TABLE IF EXISTS`).
  3. O bug de Split-Brain (onde o SQLite reportava páginas indexadas enquanto o LanceDB estava vazio, impedindo o crawler de ingerir os vetores) foi definitivamente erradicado.
  4. O re-sync de páginas já indexadas ocorre em sub-10ms utilizando projeções colunares zero-copy do Apache Arrow (`select(['file_path', 'content_hash', 'last_modified'])`), sem carregar nem deserializar vetores de embedding.
  5. As respostas do RAG utilizam os chunks reais indexados no LanceDB, citando fontes legítimas e precisas.
- **Pré-requisito**: Versão `v0.28.80` instalada (`actx -v` exibindo `v0.28.80`).

#### 📋 Passo a Passo de Execução:

1. **🗑️ Validação da Remoção da Tabela SQLite Legada:**
   - Abra um terminal e confirme que a tabela `workspace_indexed_web_pages` não existe mais no SQLite:
     ```powershell
     python -c "import sqlite3, os; conn = sqlite3.connect(os.path.expandvars(r'%LOCALAPPDATA%\AnyContext\config\settings.db')); print('Existe:', bool(conn.execute(\"SELECT name FROM sqlite_master WHERE type='table' AND name='workspace_indexed_web_pages'\").fetchone()))"
     ```
   - **Critério de Aceitação:** O comando imprime `Existe: False`.

2. **⚡ Validação da Leitura de Páginas Indexadas Direto do LanceDB (Zero-Copy Columnar Scan):**
   - Execute a inspeção das páginas indexadas pelo LanceDB:
     ```powershell
     python -c "from any_context.vector_engine.store import LanceDBStore; lance = LanceDBStore(); print('Páginas indexadas no LanceDB:', len(lance.get_indexed_pages_map('TestWorkspace')))"
     ```
   - **Critério de Aceitação:** O retorno lista com precisão todas as páginas registradas (ex: 44 para o Rust Book) diretamente da tabela vetorial, em menos de 10 milissegundos.

3. **🔄 Validação de Resync com Custo $0 (Skip Inteligente via Content Hash):**
   - Inicie a TUI ou execute o sync no workspace `TestWorkspace`:
     ```bash
     actx --tui
     ```
     No chat ou via comando `/sync`:
     ```bash
     /switch TestWorkspace
     /sync
     ```
   - **Critério de Aceitação:** O crawler detecta que as 44 páginas já estão indexadas e inalteradas no LanceDB, pulando todas em ~0.00s sem gerar novos embeddings nem custos de tokens ($0.00).

4. **🎯 Validação de RAG e Citações com Grounding Preciso:**
   - Faça uma pergunta técnica baseada na fonte indexada (ex: Rust Book):
     ```text
     O que é Rust e me dê um código mínimo de exemplo
     ```
   - **Critério de Aceitação:** O assistente responde com precisão conceitual e citações explícitas às fontes indexadas no LanceDB (ex: `[Fonte: https://doc.rust-lang.org/book/...]`), comprovando que a desincronização de split-brain foi totalmente superada.

---

### 📌 Cenário 2 (v0.28.79): Imunidade de Workspaces por Proveniência (created_by), Snapshots de Pré-Migração e Logs Persistentes

- **Objetivo**: Comprovar que a release `v0.28.79`:
  1. O workspace do usuário (ex: `TestWorkspace` ou qualquer outro nome) e todas as suas fontes, páginas web indexadas e vetores permanecem 100% intactos após a atualização.
  2. A autoria de workspaces é rastreada na coluna `created_by` (`'user'` para workspaces manuais, `'system'` para `Default` e `Shared Sources`, e `'test'` para fixtures de teste).
  3. Workspaces criados pelo usuário (`created_by = 'user'`) possuem imunidade absoluta contra qualquer rotina automática de limpeza ou migração, independente do seu nome.
  4. Um snapshot de segurança da base de dados (`settings.db.bak`) é gerado automaticamente antes de qualquer alteração estrutural no SQLite.
  5. Os eventos do ciclo de vida de atualização, instalação e migração são persistidos de forma estruturada em `%LOCALAPPDATA%\AnyContext\logs\` (`install.log`, `update.log` e `migration.log`).
- **Pré-requisito**: Versão `v0.28.79` instalada (`actx -v` exibindo `v0.28.79`).

#### 📋 Passo a Passo de Execução:

1. **📂 Validação de Imunidade e Integridade do Workspace do Usuário:**
   - Abra a TUI:
     ```bash
     actx --tui
     ```
   - Liste os workspaces com `/switch` ou verifique as fontes ativas:
     ```bash
     /switch
     ```
   - **Critério de Aceitação:** O workspace `TestWorkspace` (ou qualquer workspace criado pelo usuário) está presente na lista com todas as suas fontes indexadas (ex: 44 páginas da documentação do Rust Book) e vetores operacionais.

2. **🏷️ Validação de Proveniência (`created_by`) no Banco de Dados:**
   - Abra um terminal e consulte a proveniência dos workspaces via SQLite:
     ```powershell
     python -c "import sqlite3, os; conn = sqlite3.connect(os.path.expandvars(r'%LOCALAPPDATA%\AnyContext\config\settings.db')); print(conn.execute('SELECT name, created_by FROM workspaces').fetchall())"
     ```
   - **Critério de Aceitação:** Os workspaces `Default` e `Shared Sources` exibem `system`, enquanto `TestWorkspace` e workspaces criados pelo usuário exibem `user`.

3. **🛡️ Validação do Snapshot de Pré-Migração (`settings.db.bak`):**
   - Verifique o diretório de configuração do AnyContext:
     ```powershell
     Test-Path "$env:LOCALAPPDATA\AnyContext\config\settings.db.bak"
     ```
   - **Critério de Aceitação:** O arquivo `settings.db.bak` existe como uma cópia de segurança íntegra pré-migração.

4. **📝 Validação dos Logs Persistentes (`update.log`, `install.log`, `migration.log`):**
   - Inspecione a pasta de logs:
     ```powershell
     Get-ChildItem "$env:LOCALAPPDATA\AnyContext\logs"
     ```
   - Verifique os registros gravados:
     ```powershell
     Get-Content "$env:LOCALAPPDATA\AnyContext\logs\migration.log" -Tail 10
     ```
   - **Critério de Aceitação:** Os arquivos de log registram com timestamp detalhado cada operação realizada (migração de schema, snapshot e histórico de atualizações).

---

### 📌 Cenário 2 (v0.28.78): Modelo Padrão de Fábrica (GPT-4o Mini), Auto-Expurgo de Workspaces de Teste e Sandbox de Isolamento

- **Objetivo**: Comprovar que a release `v0.28.78`:
  1. A interface OpenTUI (`actx --tui`) e a CLI iniciam rigorosamente com `🤖 GPT-4o Mini` no workspace `Default` (`📂 Default │ 🤖 GPT-4o Mini │ 🛡️ Strict │ 🌐 Search: OFF │ 💡 /menu │ ✔ Up to date`), eliminando em 100% qualquer contaminação acidental do Claude Sonnet.
  2. Nenhum workspace residual de testes passados (`RpcUnitTestWS`, `NewRPCWS`, `Unit_Dispatch_WS`, `TestWorkspace`, `E2E_Empty_Workspace`) permanece visível na lista de workspaces do usuário.
  3. A execução da suíte de testes locais não contamina, modifica ou adiciona qualquer dado ao banco canônico do usuário (`%LOCALAPPDATA%\AnyContext\config\settings.db`), operando sob sandbox temporário hermético (`ACTX_SETTINGS_DB`).
  4. A troca de modelo via `/model` ou RPC é persistida e isolada por workspace com autoridade do Core via `ModelService`.
- **Pré-requisito**: Versão `v0.28.78` instalada (`actx -v` exibindo `v0.28.78`).

#### 📋 Passo a Passo de Execução:

1. **🤖 Validação do Modelo Padrão de Fábrica na Barra de Status:**
   - Abra um terminal e inicie a TUI:
     ```bash
     actx --tui
     ```
   - **Critério de Aceitação:** A barra inferior exibe imediatamente:
     `📂 Default  │  🤖 GPT-4o Mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu  │  ✔ Up to date`
     O modelo ativo exibido deve ser **GPT-4o Mini** e o modo de grounding deve ser **Strict**.

2. **🧹 Validação de Purga Automática de Workspaces Residuais de Teste:**
   - No chat da TUI, digite o comando para listar workspaces:
     ```bash
     /switch
     ```
   - **Critério de Aceitação:** O modal exibe apenas o workspace `Default` (e workspaces reais criados pelo próprio usuário). Nenhum workspace de teste antigo (`RpcUnitTestWS`, `NewRPCWS`, `Unit_Dispatch_WS`, `TestWorkspace`, `E2E_Empty_Workspace`) está presente. Pressione `[Esc]` para fechar o modal.

3. **🔄 Validação de Troca de Modelo Isolada por Workspace:**
   - Crie um novo workspace ou use um workspace secundário:
     ```bash
     /workspace add ProjetoBeta
     /switch ProjetoBeta
     ```
   - Altere o modelo neste workspace para outro provedor configurado ou modelo local:
     ```bash
     /model deepseek-chat
     ```
   - Retorne para o workspace padrão:
     ```bash
     /switch Default
     ```
   - **Critério de Aceitação:** O workspace `Default` permanece ativo com `🤖 GPT-4o Mini`, enquanto `ProjetoBeta` mantém seu modelo `🤖 DeepSeek V3`.

4. **🛡️ Validação de Imunidade da Base de Dados contra Testes:**
   - Em outro terminal, execute a suíte de testes automatizados:
     ```bash
     python tests/run_all.py
     ```
   - Após a conclusão bem-sucedida dos testes, volte para a TUI ou abra um novo terminal com `actx --tui`:
   - **Critério de Aceitação:** O workspace `Default` continua intacto com `🤖 GPT-4o Mini`, nenhum workspace fantasma foi adicionado à lista e as configurações de grounding continuam inalteradas.

---

### 📌 Cenário 2 (v0.28.77): Inicialização Nativa Git Bash, Handshake Instantâneo, Paridade de Comandos e UI Burra

- **Objetivo**: Comprovar que a release `v0.28.77`:
  1. O comando `actx -v` e `actx --tui` funcionam no Git Bash (`MINGW64`) e PowerShell sem erros de `No such file or directory`.
  2. O handshake do RPC bridge responde em menos de 100ms via `ping` com a versão `0.28.77` dinâmica.
  3. A paleta de comandos slash exibe os 31 comandos canônicos sincronizados dinamicamente do Core, sem duplicatas ou avisos de chaves no React.
  4. O Onboarding não possui bifurcações manuais no TypeScript e respeita metadados declarativos enviados pelo Core.
  5. A Header Bar renderiza o Tier ativo (`🌿 Community Edition`, `👥 Team Edition`, etc.) diretamente sem filtros ou substituições via regex.
- **Pré-requisito**: Versão `v0.28.77` instalada (`actx -v` exibindo `v0.28.77`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Verificação do Launcher Nativo no Git Bash & PowerShell:**
   - Abra um terminal Git Bash e execute:
     ```bash
     actx -v
     ```
   - **Critério de Aceitação:** Retorna instantaneamente `AnyContext v0.28.77 (actx)` sem qualquer erro de caminho de arquivo.

2. **⚡ Verificação da Inicialização da TUI e Handshake Instantâneo:**
   - No terminal, inicie a interface:
     ```bash
     actx --tui
     ```
   - **Critério de Aceitação:** A interface TUI abre com o banner exibindo `v0.28.77`. A barra superior (Header Bar) exibe `🌿 Community Edition` (ou o plano ativo) sem distorção no emoji.

3. **📚 Verificação da Paleta de Comandos Dinâmica e Sem Duplicatas:**
   - Dentro da TUI, digite `/` no prompt de chat:
   - **Critério de Aceitação:** A paleta de autocompletar exibe exatamente os comandos canônicos disponíveis. Não há duplicação de `/sources` ou `/sync`, e a navegação `[↑/↓]` e seleção `[Tab]` operam com fluidez.

4. **🔄 Verificação do Streaming do Web Crawler com Mini-Batches:**
   - Execute a sincronização forçada de uma documentação com muitas páginas:
     ```
     /sync --force
     ```
   - **Critério de Aceitação:** O crawler faz streaming em lotes de 25 páginas, gravando o progresso de forma atômica no banco sem travar a aplicação.

---

### 📌 Cenário 2 (v0.28.76): Persistência Integral de Configurações, Chaves de API e Imunidade de Onboarding Entre Versões

- **Objetivo**: Comprovar que ao atualizar o AnyContext para uma nova versão (`actx --update` ou nova instalação de binário):
  1. Todas as configurações do usuário (Workspaces, modelos configurados, grounding mode, web search, e chaves de API) são integralmente preservadas na base canônica `%LOCALAPPDATA%\AnyContext\config\settings.db`.
  2. O modal de onboarding inicial ("🤖 Welcome to AnyContext AI Setup!") NÃO é acionado caso o usuário já tenha configurado sua chave de API ou provedor local em uma versão anterior, iniciando o sistema imediatamente pronto para uso (`stage = "ready"`).
  3. Atualizações de parâmetros de contexto (`/preset turbo`, alterações de chunk size ou top_k) e salvamentos globais (`save_app_settings`) nunca apagam o sinalizador `onboarding_completed` ou deletam workspaces configurados.
  4. A nova tabela `system_config` garante isolamento persistente e tripla camada de auto-cura para as preferências do sistema.
- **Pré-requisito**: Versão `v0.28.76` instalada (`actx -v` exibindo `v0.28.76`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Verificação de Inicialização Direta sem Onboarding:**
   - Inicie a interface OpenTUI ou execute o actx:
     ```powershell
     actx --tui
     ```
   - **Critério de Aceitação:** A interface TUI abre diretamente na tela principal de chat com o modelo previamente configurado (`gpt-4o-mini` ou personalizado) e a lista de workspaces preservada. Nenhum modal pedindo chave de API da OpenAI deve ser exibido.

2. **🔑 Verificação de Chaves de API e Provedor Preservados:**
   - Digite no chat:
     ```
     /model
     ```
   - **Critério de Aceitação:** O menu exibe o provedor ativo configurado sem solicitar nova digitação de chave. O status de onboarding exibe "Pronto" / "Ready".

3. **⚙️ Verificação de Imunidade a Alterações de Configuração:**
   - Alterne o preset de recuperação:
     ```
     /preset turbo
     ```
   - Feche a aplicação (`/exit` ou `Ctrl+C`) e reabra `actx --tui`.
   - **Critério de Aceitação:** A aplicação inicializa normalmente sem disparar o onboarding. O preset `turbo` permanece ativo.

---

### 📌 Cenário 2 (v0.28.75): Auto-Cura de Sessão Interrompida & Sanitização de Tool Calls Órfãos (OpenAI Error 400)

- **Objetivo**: Comprovar que quando um turno conversacional é interrompido ou sofre cancelamento durante a execução de ferramentas (`search_db` / `live_web_search`), a mensagem assistente com `tool_calls` é automaticamente sanitizada com respostas sintéticas válidas no histórico de checkpoints (`ResilientSqliteSaver` & `_prune_messages_for_llm`), impedindo que o modelo OpenAI retorne o erro fatal `Error code: 400 - An assistant message with 'tool_calls' must be followed by tool messages responding to each 'tool_call_id'`.
- **Pré-requisito**: Versão `v0.28.75` instalada (`actx -v` exibindo `v0.28.75`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Abertura da Interface OpenTUI ou CLI:**
   - Inicie a interface TUI:
     ```powershell
     actx --tui
     ```
   - Alterne para o workspace de testes:
     ```
     /switch TestWorkspace
     ```

2. **💬 Execução de Consultas Consecutivas com Busca Contextual:**
   - No chat, faça uma pergunta que dispara consulta ao banco vetorial:
     ```
     Em poucas palavras, o que é Rust?
     ```
   - **Critério de Aceitação:** A IA consulta o banco vetorial e responde factualmente sobre Rust com as citações das fontes web consultadas (`The Rust Programming Language`), sem qualquer mensagem de erro 400.

3. **🔄 Execução de Nova Pergunta na Mesma Sessão:**
   - Imediatamente após a resposta, envie:
     ```
     O que instalar para começar a programar em Rust?
     ```
   - **Critério de Aceitação:** A IA processa o histórico com as ferramentas anteriores, executa a nova busca vetorial e responde detalhadamente sobre o `rustup`, `cargo` e o compilador, citando os capítulos de instalação do livro sem falhas de `tool_call_id`.

---

### 📌 Cenário 3 (v0.28.74): Validação de Resiliência do Crawler Web, Launcher Shim Dual-Binary e Blindagem de Sessão no `/update`

- **Objetivo**: Comprovar que:
  1. A adição de um portal de documentação como o Rust Book (`https://doc.rust-lang.org/stable/book/`) realiza o rastreamento recursivo completo em segundo plano mesmo se o workspace tiver acabado de ser criado/alternado, indexando dezenas de páginas e permitindo responder perguntas contextuais no chat.
  2. O comando `actx -v` responde instantaneamente (< 60ms) via Launcher Shim nativo C# sem sofrer degradação de performance por sobreposição de binários.
  3. Durante o processo de `/update`, a árvore de processos da sessão ativa é imune ao fechamento forçado, impedindo que o prompt do terminal (`bash.exe` / `cmd.exe`) vaze ou corrompa a tela da interface TUI.
- **Pré-requisito**: Versão `v0.28.74` instalada (`actx -v` exibindo `v0.28.74`).

#### 📋 Passo a Passo de Execução:

1. **⚡ Validação de Latência Instantânea do `actx -v`:**
   - Abra um terminal e execute:
     ```powershell
     actx -v
     ```
   - **Critério de Aceitação:** A resposta deve ser imediata (< 100ms), exibindo exatamente `v0.28.74` sem aguardar 6 a 8 segundos.

2. **🌐 Validação de Rastreamento Recursivo de Fontes Web (`/web --add`):**
   - Inicie a OpenTUI:
     ```powershell
     actx --tui
     ```
   - Crie e alterne para um novo workspace limpo:
     ```
     /switch RustDocs
     ```
   - Imediatamente adicione a fonte de documentação:
     ```
     /web --add https://doc.rust-lang.org/stable/book/
     ```
   - Aguarde a notificação de sincronização ser emitida:
     ```
     ✔ Workspace 'RustDocs' synchronization completed (XX web page(s) crawled and indexed).
     ```
   - Verifique a lista de fontes:
     ```
     /sources
     ```
   - **Critério de Aceitação:** A fonte deve registrar múltiplas páginas (ex: `• 40+ pages (https://doc.rust-lang.org/stable/book/)`), e não apenas 1 página.

3. **🤖 Validação de RAG e Recuperação Semântica:**
   - No chat do workspace `RustDocs`, pergunte:
     ```
     Em poucas palavras, o que é o Rust?
     ```
   - **Critério de Aceitação:** A IA deve responder com embasamento factual nos documentos do livro de Rust recém-indexados, sem informar que a informação não consta nos documentos.

4. **🛡️ Validação de Blindagem no `/update`:**
   - Execute o comando `/update`.
   - **Critério de Aceitação:** Se nenhuma outra janela externa estiver aberta, a interface NÃO deve exibir a opção "Close other instances". Ao prosseguir com a atualização, o terminal Git Bash / PowerShell não deve piscar nem vazar prompt de comando sobre a TUI.

---

### 📌 Cenário 4 (v0.28.73): Validação de Renderização Limpa do `/logs` na Interface OpenTUI (`actx --tui`)

- **Objetivo**: Comprovar que o comando `/logs`, `/spans` e `/diagnostics` na interface gráfica OpenTUI (`actx --tui`) renderiza as mensagens de observabilidade em caixas de código Markdown limpas e monospaçadas, sem nenhuma sujeira, caracteres flutuantes ou quebra do grid do terminal.
- **Pré-requisito**: Versão `v0.28.73` instalada (`actx -v` exibindo `v0.28.73`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Inicialização da OpenTUI:**
   ```powershell
   actx --tui
   ```

2. **📜 Execução do Comando de Logs:**
   - Na barra de input, digite:
     ```
     /logs 10
     ```
   - Pressione Enter.
   - **Critério de Aceitação:**
     - O bloco de logs deve aparecer formatado dentro de uma caixa de código monospaçada limpa (`### 📜 AnyContext System Logs`).
     - A tela NÃO deve apresentar nenhuma "sujeira", caracteres soltos no fundo preto ou desalinhamento de colunas.
     - A barra inferior, o prompt `👤 You:` e o scroll continuam perfeitamente intactos e responsivos.

3. **⏱️ Execução do Comando de Performance (`/spans`):**
   - Digite `/spans 5` e pressione Enter.
   - **Critério de Aceitação:** As métricas de latência devem ser exibidas em bloco formatado limpo sem erros visuais.

4. **📊 Execução de Diagnóstico (`/diagnostics`):**
   - Digite `/diagnostics` e pressione Enter.
   - **Critério de Aceitação:** O relatório de saúde do AnyContext deve ser renderizado em Markdown estruturado.

---

### 📌 Cenário 5 (v0.28.72): Adição Resiliente de Fontes Web com Descompressão Tolerante a Falhas (`/web --add <url>`)

- **Objetivo**: Comprovar que a adição de fontes web complexas (portais com compactação Akamai/Cloudflare como `canada.ca`) é executada com sucesso e auto-cura de interrupções de stream zlib/gzip, sem erros de descompressão truncada (`Error -5`).
- **Pré-requisito**: Versão `v0.28.72` instalada (`actx -v` exibindo `v0.28.72`).

#### 📋 Passo a Passo de Execução:

1. **🌐 Adição de Portal Web via CLI ou OpenTUI:**
   - Inicie o AnyContext ou abra o OpenTUI (`actx --tui`).
   - Execute o comando:
     ```
     /web --add https://www.canada.ca/en/immigration-refugees-citizenship.html
     ```
   - **Critério de Aceitação:** O comando deve responder com confirmação positiva:
     ```
     ✅ Added web source 'https://www.canada.ca/en/immigration-refugees-citizenship.html' to workspace 'Default'.
     ⚡ Crawler started in background.
     ```
     E NÃO deve exibir `❌ Error adding web source: Error -5 while decompressing data: incomplete or truncated stream`.

2. **📜 Validação de Rastreamento Estruturado de Logs:**
   - Execute no terminal:
     ```
     /logs 10
     ```
   - **Critério de Aceitação:** Os logs do sistema devem registrar a execução do comando `/web` com telemetria limpa e sem erros não tratados.

3. **🔍 Listagem de Fontes Web Cadastradas:**
   - Execute:
     ```
     /web --list
     ```
   - **Critério de Aceitação:** A URL deve constar na lista de portais cadastrados no workspace ativo.

---

### 📌 Cenário 5 (v0.28.71): Validação de Resposta Instantânea do Launcher Shim (`actx -v` / `actx --version`)

- **Objetivo**: Comprovar que o comando `actx -v` e `actx --version` responde de forma instantânea (< 50ms) imprimindo a versão de maneira simples, limpa e direta (`v0.28.71`), sem carregar módulos pesados ou descompactar o runtime no `%TEMP%`.
- **Pré-requisito**: Versão `v0.28.71` instalada via `.\scripts\install.ps1` ou `install.sh`.

#### 📋 Passo a Passo de Execução:

1. **⚡ Execução do Fast-Path de Versão Curta (`-v`):**
   ```powershell
   actx -v
   ```
   - **Critério de Aceitação:** O retorno deve ser instantâneo (mesma sensação de `node -v`), imprimindo exatamente:
     ```
     v0.28.71
     ```

2. **⚡ Execução do Fast-Path de Versão Longa (`--version`):**
   ```powershell
   actx --version
   ```
   - **Critério de Aceitação:** Retorno idêntico e instantâneo imprimindo `v0.28.71`.

3. **🚀 Execução Transparente do Chat Interativo:**
   ```powershell
   actx
   ```
   - **Critério de Aceitação:** O Launcher Shim delega a chamada transparentemente para o `actx-core.exe`, abrindo o chat interativo do AnyContext normalmente. Digite `/exit` para sair.

---

### 📌 Cenário 5 (v0.28.70): Validação de Inicialização Estável da Interface OpenTUI (`actx --tui`)

- **Objetivo**: Comprovar que o comando `actx --tui` abre a interface gráfica interativa do terminal sem nenhum erro de sintaxe do Bun (`Expected "]" but found ";"`) e renderiza o HeaderBar, prompt de entrada e barra de status.
- **Pré-requisito**: Versão `v0.28.70` instalada (`actx -v` deve exibir `v0.28.70`).

#### 📋 Passo a Passo de Execução:

1. **🖥️ Inicialização da Interface OpenTUI:**
   ```powershell
   actx --tui
   ```
   - **Critério de Aceitação:** A interface visual deve abrir imediatamente sem exibir exceções do Bun.
   - Deve exibir a barra superior (HeaderBar), a área de conversação com o status de sincronização e a barra de input na parte inferior.

2. **⌨️ Envio de Mensagem ou Comando Slash:**
   - Na barra de input, digitar:
     ```powershell
     /help
     ```
   - Pressionar Enter.
   - **Critério de Aceitação:** A lista de comandos ou a mensagem deve ser processada e exibida na tela.
   - Pressionar `Esc` ou digitar `/exit` para sair limpo do terminal.

---

### 📌 Cenário 5 (v0.28.69): Validação da Suíte de Testes 100% Nativa e Aprovação no GitHub Actions CI/CD

- **Objetivo**: Comprovar que o AnyContext executa toda a sua suíte de testes de forma 100% autônoma via Python standard library `unittest` sem necessidade de bibliotecas externas (pytest), atingindo 200 testes aprovados localmente e no pipeline de CI/CD do GitHub Actions.
- **Pré-requisito**: Versão `v0.28.69` ou branch `dev` atualizada.

#### 📋 Passo a Passo de Execução:

1. **🧪 Execução Isolada do Teste de Modelo Padrão por Workspace:**
   ```powershell
   python -m unittest tests.unit.core.test_workspace_default_model
   ```
   - **Critério de Aceitação:** O comando deve rodar os 3 testes unitários e finalizar com `OK` (Ran 3 tests).

2. **🚀 Execução da Suíte Master Modular de Testes:**
   ```powershell
   python tests/run_all_e2e.py
   ```
   - **Critério de Aceitação:** Todos os 200 testes (Core, CLI UI, Server, Ingestion e E2E) devem ser descobertos e aprovados com a mensagem `ALL 200 TESTS PASSED SUCCESSFULLY!`.

3. **☁️ Validação do Workflow no GitHub Actions:**
   - Acessar a aba **Actions** no repositório GitHub (`Levix-Digital/any-context`).
   - Validar que a execução do workflow **End-to-End (E2E) Modular Test Suite** para o push da versão `v0.28.69` finaliza com status verde (Checkmark ✔️) sem nenhum erro de `ModuleNotFoundError: No module named 'pytest'`.

---

### 📌 Cenário 5 (v0.28.68): Validação de Progresso em Tempo Real do Web Crawler e Notificação de Conclusão

- **Objetivo**: Comprovar que, ao adicionar uma fonte web ou sincronizar um portal em segundo plano, a barra inferior exibe a animação/barra de progresso do crawler (`⚡ Crawling [████░░░░] 50% (15/30 pages)`) em vez do badge prematuro `✔ Up to date`, e que ao finalizar o crawling uma notificação de sistema é exibida informando o total de páginas indexadas.
- **Pré-requisito**: Versão `v0.28.68` ou superior.

#### 📋 Passo a Passo de Execução:

1. **🌐 Adicionar Fonte Web no Chat (CLI ou OpenTUI):**
   ```powershell
   actx
   ```
   - No chat interativo, digitar:
     ```powershell
     /web --add https://docs.python.org/3/tutorial/
     ```
   - Observar a resposta imediata informando o início do crawler em segundo plano.

2. **⚡ Observar Barra de Status Durante o Crawling:**
   - Observar o rodapé/barra de status inferior:
     - Deve exibir `⚡ Crawling [crawling...]` ou `⚡ Crawling [████░░░░] ... (X/Y pages)`.
     - **Critério de Aceitação:** Não deve exibir `✔ Up to date` enquanto o crawler estiver em execução.

3. **🔔 Validar Notificação de Sistema ao Finalizar:**
   - Aguardar o término do crawler.
   - Validar que a notificação de sistema aparece com o sumário:
     `✅ System Notification:`
     `✔ Workspace '...' synchronization completed (X web page(s) crawled and indexed).`
   - Validar que após a conclusão a barra inferior transiciona para `✔ Up to date`.

---

### 📌 Cenário 5 (v0.28.67): Validação de Modelo Padrão de Fábrica `gpt-4o-mini` para Todo Novo Workspace e Isolamento entre Espaços


- **Objetivo**: Comprovar que todo novo workspace criado (seja no primeiro boot, via `/switch`, REST API ou MCP Server) inicia estritamente com o modelo de IA `gpt-4o-mini`, e que trocar o modelo no Workspace A não contamina o Workspace B recém-criado.
- **Pré-requisito**: Versão `v0.28.67` ou superior.

#### 📋 Passo a Passo de Execução:

1. **🤖 Iniciar o Chat e Verificar Modelo no Workspace Atual:**
   ```powershell
   actx
   ```
   - Observar na telemetria de boot ou na barra inferior que o modelo ativo é `gpt-4o-mini`.

2. **🔄 Alterar o Modelo no Workspace Atual:**
   - No chat do AnyContext, digitar:
     ```powershell
     /model gpt-4o
     ```
     - Comprovar que o modelo muda para `gpt-4o`.

3. **🆕 Criar um Novo Workspace e Validar que Permanece em `gpt-4o-mini`:**
   - No chat, digitar:
     ```powershell
     /switch TesteNovoWorkspace
     ```
     - Comprovar que o novo workspace `TesteNovoWorkspace` é criado e o modelo ativo exibido na barra inferior é rigorosamente `gpt-4o-mini` (sem contaminar com o `gpt-4o` do workspace anterior).

---

### 📌 Cenário 5 (v0.28.66): Validação de Fast-Path Instantâneo de Flags (`actx -v`) e Inicialização Desacoplada de Rede


- **Objetivo**: Comprovar que a flag de versão `actx -v` / `actx --version` responde de forma imediata em sub-milissegundos (< 5ms) sem inicializar bancos de dados, e que o startup do chat interativo não sofre nenhum travamento por chamadas síncronas de rede do verificador de atualizações.
- **Pré-requisito**: Versão `v0.28.66` ou superior.

#### 📋 Passo a Passo de Execução:

1. **⚡ Testar Fast-Path de Versão Imediato:**
   ```powershell
   actx -v
   ```
   - Validar que a linha `AnyContext (actx) v0.28.66 - Levix Digital` aparece instantaneamente no terminal sem qualquer delay perceptível.

2. **🚀 Testar Startup sem Bloqueio de Rede:**
   ```powershell
   actx
   ```
   - Validar que o chat abre imediatamente com o banner e a telemetria de boot, sem travar na checagem de atualizações.

---

### 📌 Cenário 5 (v0.28.65): Validação de Startup Instantâneo (< 100ms), Telemetria Visual de Boot e Time Watching de Observabilidade (`actx --diag`, `/logs`, `/diagnostics`, `/spans`)


- **Objetivo**: Comprovar que a inicialização do AnyContext ocorre de forma quase instantânea com carregamento lazy, exibindo a telemetria visual de micro-etapas de boot (`Engine Startup Telemetry`) com tempos em milissegundos logo abaixo do banner, e que o módulo de observabilidade (*time watching*) registra a latência de todas as operações críticas e disponibiliza relatórios de diagnóstico detalhados.
- **Pré-requisito**: Versão `v0.28.65` ou superior.

#### 📋 Passo a Passo de Execução:

1. **⚡ Testar Inicialização Rápida e Telemetria Visual de Boot:**
   ```powershell
   actx
   ```
   - Validar que o CLI abre instantaneamente sem congelamento de tela.
   - Observar o bloco de telemetria de inicialização impresso logo abaixo do banner ASCII:
     ```text
       ┌─ ⚡ Engine Startup Telemetry
       │ ├─ [ 8.2ms] 🔌 SQLite Configuration Store active
       │ ├─ [19.4ms] 🤖 AI Model engine linked (gpt-4o-mini - OPENAI)
       │ ├─ [28.1ms] 📂 Workspace connected (Default)
       │ ├─ [34.7ms] 📦 Context state verified (Up to date - 42 files)
       │ └─ [41.9ms] 🚀 AnyContext ready in 0.04s
     ```

2. **⏱️ Executar Comandos de Time Watching no Chat Interativo:**
   - No chat do AnyContext, digitar:
     ```powershell
     /spans
     ```
     - Comprovar que a lista de operações recentes é exibida com duração em milissegundos e status `[ok]`.
   - Digitar:
     ```powershell
     /diagnostics
     ```
     - Comprovar que a seção `⏱️ Performance & Latency Metrics (Recent Spans)` consolida métricas como `rag:retrieval`, `cmd:<nome>`, `ingestion:local_folder`, `sync:check_changes` com médias (`avg_ms`), contagem (`xN`), `min` e `max`.
   - Digitar:
     ```powershell
     /logs
     ```
     - Comprovar que os logs de sistema estruturados são renderizados com formatação clara.

3. **🔍 Validar Diagnóstico via Flag CLI:**
   - Sair do chat (`/exit`) e executar no terminal:
     ```powershell
     actx --diag
     ```
     - Validar que o relatório completo de saúde, motor SQLite, Bun runtime e a matriz de latência dos spans recentes é gerado instantaneamente no terminal com cores ANSI.

---

### 📌 Cenário 5 (v0.28.64): Validação de Integridade do Modelo Limpo de 2 Camadas (Sem Workspace Global e com Isolamento Estrito de RAG)


- **Objetivo**: Comprovar que a remoção do workspace `Global` unificou a arquitetura em um modelo de 2 camadas limpo (`Workspaces de Projeto` + `Shared Sources` vinculável sob demanda via `/link`), garantindo que apenas `Default` e `Shared Sources` sejam protegidos pelo sistema e que o RAG nunca puxe dados não-linkados de outros escopos.
- **Pré-requisito**: Versão `v0.28.64` ou superior.

#### 📋 Passo a Passo de Execução:

1. **📁 Verificar Workspaces Ativos e Protegidos:**
   ```powershell
   actx --workspaces
   ```
   - Comprovar que os workspaces de sistema presentes são estritamente `Default` e `Shared Sources` (sem o workspace `Global`).

2. **🗑️ Tentar Deletar ou Renomear Workspaces Protegidos:**
   ```powershell
   # No Chat:
   /workspace delete Default
   /workspace delete "Shared Sources"
   /rename Default MeuNovoNome
   ```
   - Comprovar que o sistema bloqueia com mensagem de proteção de workspace de sistema (`protected system workspace`).

3. **🔗 Testar Vinculação Explícita com `/link`:**
   - Adicionar uma fonte ao `Shared Sources`:
     ```powershell
     /folder --add C:\MinhaBiblioteca --workspace "Shared Sources"
     ```
   - Criar e alternar para um novo workspace `ProjetoBeta`:
     ```powershell
     /switch ProjetoBeta
     ```
   - Fazer uma pergunta no `ProjetoBeta` antes de linkar:
     - Comprovar que no modo `Strict`, o AnyContext informa que a informação não consta no workspace (sem poluição fantasma de outros contextos).
   - Vincular a fonte via `/link`:
     ```powershell
     /link MinhaBiblioteca
     ```
   - Repetir a pergunta e comprovar que agora a resposta é sintetizada perfeitamente a partir dos documentos da fonte compartilhada.

---

### 📌 Cenário 5 (v0.28.63): Validação de Remoção Interativa de Fontes (`/menu` ➔ Workspaces ➔ Delete Source)

- **Objetivo**: Comprovar que o menu hierárquico `/menu` navega com profundidade total até a seleção e remoção de fontes individuais (pastas locais e URLs web) de um workspace, abrindo a lista de fontes ativas, exibindo modal de confirmação explícito e executando a remoção no SQLite e no LanceDB sem fechar o terminal.
- **Pré-requisito**: Versão `v0.28.63` ou superior.

#### 📋 Passo a Passo de Execução:

1. **💥 Simular Estado Zero (Reset de Fábrica):**
   ```powershell
   actx --factory-reset
   ```
   - Confirmar a execução do reset de fábrica.
   - Comprovar que o SQLite grava `onboarding_completed = 0`.

2. **🖥️ Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```
   - Comprovar que a interface OpenTUI abre e **dispara automaticamente** o modal interativo `🤖 Welcome to AnyContext AI Setup!` na tela.
   - Comprovar que o modal exibe as 3 opções com badges e ícones:
     - `⚡ OpenAI Cloud (Enter OpenAI API Key - Recommended) [Recommended]`
     - `🏠 Local Offline Server (LM Studio / Ollama - 100% Free & Offline) [Offline]`
     - `🛠️ Custom Setup (Configure custom models, base URL & keys) [Advanced]`

3. **🤖 Testar Configuração da OpenAI:**
   - Navegar com as setas `[↑/↓]` até `⚡ OpenAI Cloud` e pressionar `[Enter]`.
   - Comprovar que o modal fecha, preenche o prompt com `/key openai ` e exibe mensagem orientando a colar a chave.
   - Colar a chave `sk-...` e pressionar Enter.
   - Comprovar que a chave é gravada no banco, `onboarding_completed` vai para `1` e o status bar ativa imediatamente `🤖 GPT-4o Mini`.

4. **💬 Validar Comunicação no Chat:**
   - Digitar `Olá! Qual o resumo deste projeto?` e pressionar Enter.
   - Comprovar que o agente sintetiza e transmite a resposta sem travamentos.

5. **🔄 Validar Persistência em Reinicializações Futuras:**
   - Sair com `/exit` e reabrir `actx --tui`.
   - Comprovar que o aplicativo entra direto no chat em milissegundos sem reabrir o modal de boas-vindas.

---

### 📌 Cenário 5 (v0.28.51): Validação de Desinstalação Robusta, Resolução Canônica de Paths (`%LOCALAPPDATA%\AnyContext`) e Reset Seguro de Modelos (`gpt-4o-mini` / `openai`)

- **Objetivo**: Comprovar que o AnyContext opera unicamente sobre o diretório de dados canônico (`%LOCALAPPDATA%\AnyContext`), sem ressuscitar arquivos legados em `~\config\settings.db` ou no diretório de trabalho, e que o script de desinstalação (`uninstall.ps1` / `uninstall.sh`) remove diretórios canônicos, purga legados e desinstala resquícios de pacotes no ambiente Python (`pip uninstall`).
- **Pré-requisito**: Versão `v0.28.51` ou superior instalada.

#### 📋 Passo a Passo de Execução:

1. **🚀 Validar Inicialização Instantânea do AnyContext e Modelo Padrão:**
   ```powershell
   python -m any_context.cli.entrypoint --version
   actx --version
   ```
   - Comprovar que a resposta de versão é exibida instantaneamente sem nenhum erro de traceback do NumPy (`cannot load module more than once per process`).

2. **🤖 Validar Carregamento e Comunicação com o Modelo no Chat:**
   - Iniciar o AnyContext:
     ```powershell
     actx
     ```
   - Comprovar que o modelo ativo exibido na barra inferior é `🤖 gpt-4o-mini` sob o provedor `OpenAI`.
   - Enviar uma mensagem de teste: `Olá! Qual modelo você está usando agora?`
   - Comprovar que o agente sintetiza e transmite a resposta em streaming sem erros de autenticação ou travamento.

3. **🔍 Verificar Integridade de Arquivos no Sistema Operacional:**
   - Comprovar que nenhum arquivo `settings.db` foi criado na pasta do usuário `~\config\settings.db` ou na raiz do repositório.
   - Comprovar que apenas `%LOCALAPPDATA%\AnyContext\config\settings.db` permanece como banco ativo.

4. **🧹 Executar o Desinstalador com Preservação de Dados:**
   - Executar no terminal:
     ```powershell
     .\scripts\uninstall.ps1
     ```
   - Responder `Y` (Preservar Workspaces e Histórico).
   - Comprovar que o executável em `AppData\Local\actx` é removido, o `settings.db` canônico tem seus modelos resetados para os padrões de fábrica da OpenAI, os bancos legados órfãos são purgados e o script verifica/desinstala resquícios de pacotes `pip`.

---

### 📌 Cenário 5 (v0.28.37): Validação de Inicialização Standalone do PyInstaller (`actx --tui` e `actx --rpc`) sem Colisão de DLL

- **Objetivo**: Comprovar que o binário standalone compilado (`actx.exe`) inicializa o frontend OpenTUI (`actx --tui`) e dispara o backend de RPC (`actx --rpc`) sem erro de colisão de DLL do NumPy (`cannot load module more than once per process`) devido à higienização de caminhos `_MEI` no `PATH`.
- **Pré-requisito**: Binário standalone compilado ou ambiente na versão `v0.28.37`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI pelo Executável Standalone:**
   ```powershell
   actx --tui
   ```
2. **⚡ Validar Carregamento sem Erros:**
   - Comprovar que a interface OpenTUI abre fluidamente em tela cheia com o banner e prompt `👤 You:`.
   - Comprovar que não há mensagens de erro de traceback do Python ou DLL loader no terminal.
3. **💬 Enviar uma Pergunta de Teste no Chat:**
   - Digitar `Qual o resumo deste workspace?` e pressionar Enter.
   - Comprovar que a resposta do agente é sintetizada e transmitida em tempo real.

---

### 📌 Cenário 5 (v0.28.36): Auto-Restart do `/update` com Preservação do Diretório de Trabalho

- **Objetivo**: Comprovar que ao atualizar via `/update`, a aplicação é reiniciada automaticamente preservando o diretório de trabalho original do usuário (`-WorkingDirectory`) em vez de abrir em `C:\Windows\System32`.
- **Pré-requisito**: Versão `v0.28.36` ou superior.

#### 📋 Passo a Passo de Execução:

1. **🚀 Abrir o AnyContext em um Diretório Específico de Projeto:**
   ```powershell
   cd C:\Users\guilh\source\repos\any-context
   actx --tui
   ```
2. **⚡ Disparar a Atualização:**
   - No prompt, digitar `/update` e escolher `⚡ Update in background`.
3. **🔄 Verificar a Reinicialização Automática:**
   - Comprovar que o AnyContext fecha o processo antigo e reinicia a nova versão na tela mantendo o diretório ativo do repositório/projeto.

---

### 📌 Cenário 6 (v0.28.33): Normalização Canônica de Modelos de IA (`/model`)

- **Objetivo**: Comprovar que ao selecionar modelos no modal `/model` (ex: `GPT-4o Mini (Universal - Fast & Efficient)`), o ID canônico técnico (`gpt-4o-mini`) é gravado no SQLite e enviado para as APIs sem gerar erro `400: invalid model ID`.
- **Pré-requisito**: Versão `v0.28.33` ou superior.

#### 📋 Passo a Passo de Execução:

1. **🚀 Abrir o Modal de Modelos:**
   - No prompt do chat, digitar `/model` e pressionar Enter.
2. **🤖 Selecionar o Modelo:**
   - Navegar até `GPT-4o Mini` e pressionar Enter.
3. **💬 Enviar uma Pergunta de Raciocínio Geral:**
   - Digitar: `Qual é mais barato por ml, o Monster, o Redbull ou o Rockstar?`
   - Comprovar que a IA responde perfeitamente sem erros de API.

---

### 📌 Cenário 7 (v0.28.32): Exclusão Interativa de Workspaces com Confirmação de Segurança

- **Objetivo**: Comprovar que a opção `Delete Workspace` no menu `/menu` lista todos os workspaces customizados e abre um modal de confirmação explícito antes de qualquer remoção, impedindo deleções acidentais e revertendo para `Default` se o workspace ativo for excluído.
- **Pré-requisito**: Versão `v0.28.32` ou superior.

#### 📋 Passo a Passo de Execução:

1. **➕ Criar um Workspace Temporário de Teste:**
   - No chat, digitar `/switch --create TestDeleteFlow` e anexar ou não fontes.
2. **🗑️ Abrir a Exclusão pelo Menu:**
   - Digitar `/menu` ➔ **Workspaces & Folders Management** ➔ **Delete Workspace**.
   - Comprovar que o modal lista `TestDeleteFlow` com o número de fontes.
3. **⚠️ Validar o Modal de Confirmação:**
   - Selecionar `TestDeleteFlow` e pressionar Enter.
   - Comprovar que surge a tela de confirmação com `🗑️ Yes, permanently delete 'TestDeleteFlow'` e `🔙 Cancel (Keep 'TestDeleteFlow')`.
   - Testar o cancelamento (`Cancel`) e comprovar que o workspace é mantido intacto.
   - Repetir e confirmar com `Yes`: comprovar que o workspace é purgado e o AnyContext chaveia de volta para `Default`.

---

### 📌 Cenário 8 (v0.28.17): Atualização Interativa (`/update`) com Modal de Instâncias e Auto-Restart Automático (TUI & CLI)

- **Objetivo**: Comprovar que ao digitar `/update` dentro do chat da TUI (`actx --tui`) ou no CLI (`actx`), o AnyContext detecta a versão mais recente e as instâncias ativas, abre o modal de opções interativo (`<InteractiveModal>`) com as 3 opções (`⚡ Update in background`, `⏹️ Close other instances`, `🔙 Cancel`), baixa o binário e realiza a substituição atômica com **reinício automático da aplicação** sem necessidade de intervenção externa.
- **Pré-requisito**: Binário ou ambiente na versão `v0.28.17`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Executar o Comando `/update`:**
   - No prompt `👤 You:`, digitar `/update` e pressionar Enter.
   - Comprovar que abre o modal interativo `🚀 AnyContext Update Available: vX.Y.Z → vA.B.C` contendo:
     - `⚡ Update in background (Recommended) - Active background sessions continue working undisturbed.`
     - `⏹️ Close other instances and update now - Terminates background process(es) before updating.`
     - `🔙 Cancel update - Aborts the update process and returns to chat.`

3. **⚡ Testar Seleção e Auto-Restart:**
   - Navegar com `[↑/↓]` e pressionar `Enter` na opção `⚡ Update in background`.
   - Comprovar que o modal fecha, a mensagem de download/atualização é exibida e o AnyContext reinicia automaticamente abrindo a nova versão na tela!

---

### 📌 Cenário 5 (v0.28.16): Validação de Diretórios Nativos do SO e Criptografia em Repouso AES-GCM-256

- **Objetivo**: Comprovar que o AnyContext armazena todos os bancos de dados em diretórios nativos do sistema operacional (`%LOCALAPPDATA%\AnyContext\` no Windows), migra dados legados sem perda, cifra os textos dos chunks em disco com AES-GCM-256 amarrado ao hardware e entrega respostas RAG descriptografadas em tempo real com performance máxima tanto no CLI quanto na TUI.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.28.16`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Inspecionar o Diretório Oficial de Dados do SO:**
   - Abrir o PowerShell e verificar o diretório do AnyContext em `%LOCALAPPDATA%\AnyContext`:
     ```powershell
     Test-Path "$env:LOCALAPPDATA\AnyContext\data\context_db\lancedb"
     Test-Path "$env:LOCALAPPDATA\AnyContext\config\settings.db"
     ```
   - Comprovar que ambos retornam `True` e que a raiz do usuário (`C:\Users\<user>`) não é mais poluída com novas bases desprotegidas.

2. **🔒 Inspecionar a Criptografia em Repouso no LanceDB:**
   - Executar uma leitura direta em Python no arquivo bruto do LanceDB sem passar pelo AnyContext:
     ```powershell
     python -c "import lancedb; db = lancedb.connect('$env:LOCALAPPDATA/AnyContext/data/context_db/lancedb'); tbl = db.open_table('workspace_chunks'); row = tbl.search().limit(1).to_list()[0]; print('Encrypted text preview:', row['text'][:50])"
     ```
   - Comprovar que o campo `text` inicia com `enc::` e é completamente ilegível (cifrado com chave de hardware).

3. **⚡ Validar Consulta Semântica RAG no CLI e na TUI:**
   - Abrir o CLI (`actx`) e fazer a pergunta no workspace `ProvincialImmigration`:
     `Quais são os programas provinciais mais adequados para empreendedores?`
   - Abrir a TUI (`actx --tui`) e fazer a mesma pergunta no mesmo workspace.
   - Comprovar que ambos recuperam e descriptografam o contexto instantaneamente (< 2ms) trazendo as informações completas de Alberta e Saskatchewan!

---

### 📌 Cenário 5 (v0.28.0): Seletor Modal do `/mode` no OpenTUI (Strict, Hybrid, Proactive)

- **Objetivo**: Comprovar que o comando `/mode` (sem argumentos) abre um modal de seleção interativo estilizado (`<InteractiveModal>`) na TUI, listando as 3 estratégias de grounding (`Strict`, `Hybrid`, `Proactive`), destacando o modo ativo atual com o badge `[Active]`, navegável por setas `[↑/↓]`, selecionável com `[Enter/Tab]` e cancelável com `[Esc]`, atualizando a barra de status inferior imediatamente.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.28.0` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Abertura do Modal com `/mode`:**
   - No prompt `👤 You:`, digitar `/mode` e pressionar Enter.
   - Comprovar que abre o box estilizado `🎛️ AI Grounding & Answer Mode` contendo:
     - `🛡️ Strict (Audit & Legal) - 100% grounded to indexed documents, zero speculation`
     - `⚖️ Hybrid (Balanced) - Workspace facts + clearly labeled suggestions (Default)`
     - `🚀 Proactive (Research & Ideation) - Broad synthesis, insights & web recommendations`
   - Comprovar que a opção atualmente ativa exibe `[Active]`.

3. **⚡ Validar Navegação e Seleção:**
   - Navegar com as setas `[↑/↓]` até `🚀 Proactive`.
   - Pressionar `Enter` (ou `Tab`).
   - Comprovar que o modal se fecha, uma mensagem de sistema confirma a alteração e a barra de status inferior atualiza imediatamente para `🛡️ Proactive`.

4. **⚡ Validar Fechamento com `[Esc]`:**
   - Digitar `/mode` novamente e pressionar `Esc`.
   - Comprovar que o modal se fecha suavemente sem realizar alterações.

---

### 📌 Cenário 5 (v0.28.0): Menu Hierárquico Completo `/menu` e `/config` no OpenTUI (11 Categorias)

- **Objetivo**: Comprovar que `/menu` e `/config` abrem o modal de configuração hierárquica completa na TUI, replicando exatamente as 11 categorias de sistema do CLI, permitindo navegação em submenus com Breadcrumbs (`⚙️ Configuration ➔ 📂 Workspaces`), execução de toggles/ações e retorno de nível com `[Esc]`.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.28.0` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Abrir o Menu de Configuração:**
   - Digitar `/config` ou `/menu` e pressionar Enter.
   - Comprovar que abre o modal `⚙️ AnyContext Configuration & Settings` com as 11 categorias canônicas:
     1. `📂 Workspaces & Folders Management`
     2. `🤝 Workspace Sharing & Collaboration`
     3. `🎛️ AI Grounding & Answer Modes`
     4. `🌐 Live Web Search & External Intelligence`
     5. `🔍 Context Retrieval Density & RAG Presets`
     6. `🤖 AI Models, Base URL & API Keys`
     7. `🔑 Manage Saved API Keys`
     8. `🧠 Memory Compression & Reset Settings`
     9. `💳 Subscription & Payment Plans`
     10. `🛡️ User Accounts & Security Access Control`
     11. `💥 Factory Reset AnyContext`

2. **⚡ Navegar para um Submenu e Voltar:**
   - Selecionar `🎛️ AI Grounding & Answer Modes` com as setas e pressionar `Enter`.
   - Comprovar que o modal carrega o submenu exibindo o Breadcrumb `⚙️ Configuration ➔ 🎛️ Grounding`.
   - Pressionar `Esc` para voltar ao menu principal `⚙️ Configuration`.
   - Pressionar `Esc` novamente para fechar o menu.

---

### 📌 Cenário 6 (v0.28.0): Scrollbar, Rolagem por Teclado (`PageUp`/`PageDown`) e Respostas de Streaming sem Faixa Vazia

- **Objetivo**: Comprovar que a área de chat ocupa 100% da altura útil sem faixa vazia na metade inferior, as respostas da IA fluem mantendo a última linha sempre visível durante o streaming, e o histórico de mensagens pode ser rolado suavemente via teclado (`PageUp`, `PageDown`, `Ctrl+Up/Down`, `Home`, `End`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.28.0` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   - Comprovar que a tela inicial não possui divisão cinza/vazia cortando a metade inferior.

2. **⚡ Enviar Pergunta com Resposta Longa:**
   - Fazer uma pergunta detalhada para a IA (ex: `Liste detalhadamente 10 dicas de organização financeira`).
   - Comprovar que, à medida que os tokens chegam, o chat rola automaticamente e o texto novo nunca fica escondido atrás de faixas inferiores.

3. **⚡ Testar Rolagem por Teclado:**
   - Pressionar `PageUp` (ou `Ctrl+Up` / `Shift+Up`) várias vezes.
   - Comprovar que a conversa sobe suavemente para rever o início do texto.
   - Pressionar `PageDown` (ou `Ctrl+Down` / `Shift+Down`).
   - Comprovar que a conversa desce suavemente de volta até a última linha.
   - Pressionar `Home` (com `Ctrl` ou `Shift`) para saltar ao topo e `End` para saltar ao final.

---

### 📌 Cenário 7 (v0.28.0): Verificação de Margem Limpa e Ausência de Sobreposição nas Legendas Inferiores

- **Objetivo**: Comprovar que o texto de rodapé `💡 [↑/↓] Navigate • [Enter] Select • [Esc] Close` dentro dos modais e do Slash Palette possui espaçamento vertical limpo e nunca renderiza por cima da borda inferior da caixa.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.28.0` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Abrir o Palette (`/`) e os Modais (`/mode`, `/menu`):**
   - Digitar `/` para abrir o Slash Commands Palette.
   - Inspecionar visualmente o rodapé: comprovar que a legenda está desenhada com margem de 1 linha acima da borda arredondada inferior.
   - Digitar `/mode` e `/menu` e comprovar o mesmo espaçamento limpo em ambos os modais.

---

### 📌 Cenário 4 (v0.27.6): Validação de Header Dinâmico (Full Glory vs Compact) e Scrollbox Desacoplado

- **Objetivo**: Comprovar que o cabeçalho ASCII é renderizado no topo fora do `<scrollbox>`, exibindo a arte completa e plano dinâmico (`⭐ Pro Plan`) no início (0 mensagens), retraindo-se automaticamente para uma Top Bar compacta e limpa de 1 linha durante o diálogo (1+ mensagens), restaurando o modo completo ao executar `/clear`, com 100% de estabilidade e zero gaps no scroll de conversas.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.6` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Full Glory Mode Inicial (0 Mensagens):**
   - Comprovar que a tela inicial exibe a arte ASCII completa e o badge `⭐ Pro Plan`.
   - Comprovar que a área de mensagens abaixo está limpa, sem qualquer gap vertical ou barra de rolagem truncada.

3. **⚡ Validar Transição para Compact Top Bar Mode (1+ Mensagens):**
   - No prompt `👤 You:`, digitar uma pergunta qualquer (ex: `Olá!`) e pressionar Enter.
   - Comprovar que o cabeçalho se recolhe instantaneamente para a barra compacta de 1 linha:
     `🚀 AnyContext (actx) v0.27.6  │  Levix Digital  │  ⭐ Pro Plan`
   - Comprovar que o `<scrollbox>` ganha mais de 90% da altura da tela e as mensagens fluem suavemente com o scroll ancorado ao final.

4. **⚡ Validar Restauração do Modo Completo com `/clear`:**
   - Digitar `/clear` no prompt e pressionar Enter.
   - Comprovar que o histórico de mensagens é limpo e o cabeçalho retorna imediatamente ao **Full Glory Mode** com o banner ASCII completo.

---

### 📌 Cenário 4 (v0.27.5): Validação de Arte ASCII Completa, Menu Interativo Modal (/menu) e Flags (--on / --off)

- **Objetivo**: Comprovar a restauração da arte ASCII clássica com badge dinâmico do plano de assinatura ativo (`⭐ Pro Plan`), o funcionamento do Menu Interativo Modal acionado por `/menu` com seleção por setas `[↑/↓]` e ativação com `[Enter/Tab]`, e o suporte completo a flags com `--` no autocomplete e no dispatcher (`/web-search --on`, `/web-search --off`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.5` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Cabeçalho ASCII e Nível de Licença:**
   - Comprovar que o topo do chat exibe a arte ASCII clássica do AnyContext com o plano real do usuário (ex: `⭐ Pro Plan` ou `🏢 Enterprise Edition`).

3. **⚡ Validar Menu Interativo Modal (`/menu`):**
   - Digitar `/menu` e pressionar Enter.
   - Comprovar que abre uma caixa modal estilizada `📋 AnyContext Interactive Menu` com categorias numeradas.
   - Navegar com as setas `[↑/↓]` e pressionar `Enter` na opção `🌐 Real-Time Web Search` ou `📁 Workspace Sources`.
   - Comprovar que o comando é executado ou inserido no input e o menu se fecha.
   - Abrir novamente com `/menu` e pressionar `Esc` para validar o fechamento suave.

4. **⚡ Validar Suporte a Flags no Palette e Dispatcher:**
   - Digitar `/web-search --o` no input.
   - Comprovar que `/web-search --on` e `/web-search --off` aparecem na lista do palette.
   - Pressionar Enter com `/web-search --on` e validar que a busca web é ativada (`🟢 ON`).

---

### 📌 Cenário 4 (v0.27.4): Validação de Chat Streaming, Cópia Nativa de Texto, /menu e Ausência de Gap Vertical

- **Objetivo**: Comprovar que o streaming de inferência do agente de IA responde a perguntas sobre fontes indexadas sem erros de assinatura, a seleção e cópia com o mouse de qualquer resposta do chat funciona nativamente no terminal, o comando `/menu` abre diretamente o Slash Command Palette flutuante, a área de diálogo do chat ocupa toda a tela sem gaps verticais e comandos operacionais (`/update`, `/check-update`, `/inspect`, `/density`) estão presentes no catálogo.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.4` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Inferência do Chat com IA (Zero Erros de Assinatura):**
   - No prompt `👤 You:`, digitar uma pergunta sobre as fontes do workspace (ex: `Qual o preço do Windex?` ou `O que tem no Walmart?`).
   - Comprovar que o agente de IA processa e faz o streaming da resposta Markdown em tempo real sem qualquer erro de `create_anycontext_agent()`.

3. **⚡ Validar Cópia Nativa de Texto com o Mouse:**
   - Clicar e arrastar com o cursor do mouse sobre o texto da resposta do assistente.
   - Pressionar `Ctrl+C` (ou botão direito) e colar em um bloco de notas.
   - Comprovar que o texto selecionado é copiado com 100% de integridade.

4. **⚡ Validar Comando `/menu` e Catálogo de Comandos:**
   - Digitar `/menu` e pressionar Enter.
   - Comprovar que o Slash Command Palette flutuante é aberto diretamente na tela.
   - Digitar `/up` e comprovar que `/update` e `/check-update` aparecem na lista.
   - Pressionar `Esc` para fechar o palette.

5. **⚡ Validar Layout Compacto Sem Gap Vertical:**
   - Comprovar que as mensagens do chat fluem naturalmente de cima para baixo ocupando o espaço útil do terminal, sem faixa vazia com barra de rolagem truncada.

---

### 📌 Cenário 4 (v0.27.3): Validação de Layout do Slash Command Palette, Scoring por Prefixo e Indicador de Sync

- **Objetivo**: Comprovar a separação visual nítida do rodapé de instruções no Slash Command Palette (sem sobreposição com o último comando), a pontuação por relevância garantindo que `/source` selecione `/sources` e `/sources --all` em vez de `/sync`, e a exibição do badge verde `✔ Up to date` na barra inferior após sincronização.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.3` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar o Layout e Rodapé do Slash Command Palette:**
   - Digitar `/` para abrir o palette.
   - Comprovar que as instruções `💡 [↑/↓] Navigate  •  [↹ Tab] Select  •  [Esc] Close` aparecem em uma linha dedicada abaixo da lista, separadas por uma linha divisória nítida e sem misturar com o último item.

3. **⚡ Validar Scoring por Prefixo no Autocomplete:**
   - Digitar `/source` no input.
   - Comprovar que o primeiro item selecionado no topo é **`/sources`** e o segundo é **`/sources --all`** (e NÃO o `/sync`).
   - Pressionar `Tab` e validar que `/sources` é autocompletado.

4. **⚡ Validar Feedback de Sincronização na Barra de Status:**
   - Observar a barra de status inferior.
   - Validar que a barra exibe o status de sincronização em verde:
     `📂 Default  │  🤖 gpt-4o-mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu  │  ✔ Up to date        🚪 /exit`

---

### 📌 Cenário 4 (v0.27.2): Validação de Paridade Hexagonal Absoluta (CLI & TUI) e Encerramento Limpo (/exit)

- **Objetivo**: Comprovar a paridade absoluta de base de dados (`settings.db`), modelo ativo (`gpt-4o-mini`), plano de assinatura (`Pro Plan`), fontes indexadas (`Walmart.ca`) e catálogo universal de 23 comandos entre a CLI e a OpenTUI (`actx --tui`), além de validar o encerramento suave e limpo no `/exit` sem tela piscando.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.2` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Paridade de Dados no Dock e Status Bar:**
   - Comprovar que a barra inferior exibe o modelo real configurado pelo usuário no core:
     `📂 Default  │  🤖 gpt-4o-mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu        🚪 /exit`
   - O modelo NÃO deve ser diferente daquele configurado no CLI.

3. **⚡ Validar Listagem Completa de Fontes com `/sources`:**
   - Digitar `/sources` no chat da TUI.
   - Comprovar a exibição idêntica às fontes cadastradas no workspace `Default`:
     `### 📂 Indexed Sources in Default (1 sources)`
     `🌐 Web Portals & URLs:`
     `• [Online Shopping Canada: Everyday Low Prices at Walmart.ca!](https://www.walmart.ca/en) • 264 pages`

4. **⚡ Validar Comando Global de Fontes `/sources --all`:**
   - Digitar `/sources --all` e verificar a listagem consolidada de todos os workspaces.

5. **⚡ Validar Encerramento Limpo e Restauração de Terminal (`/exit`):**
   - Digitar `/exit` na TUI.
   - Comprovar que o aplicativo encerra **imediatamente e de forma limpa**, restaurando o cursor e o prompt do PowerShell/Bash sem qualquer loop de tela piscando.

---

### 📌 Cenário 4 (v0.27.1): Validação de Inicialização Direta da OpenTUI no Bun e Fallback para Community Edition

- **Objetivo**: Comprovar a inicialização imediata da interface OpenTUI (`actx --tui`) no Bun sem nenhum erro de sintaxe ES Module (`SyntaxError: export 'SlashCommandMeta' not found`), e verificar a exibição correta do badge `🌿 Community Edition` por padrão (ou a licença ativa configurada no SQLite).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.27.1` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Inicialização sem Erros de Sintaxe:**
   - Comprovar que o aplicativo não exibe `SyntaxError: export 'SlashCommandMeta' not found in './commands'` e abre a tela de chat interativa instantaneamente.
   - Validar que a barra de status inferior de 1 linha aparece com nitidez:
     `📂 Default  │  🤖 gpt-4o-mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu        🚪 /exit`

3. **⚡ Validar o Badge de Licença no Banner da CLI:**
   ```powershell
   actx /version
   ```
   - Validar que a CLI exibe por padrão `🌿 Community Edition` (quando nenhuma chave paga estiver ativada).

4. **⚡ Validar Comandos Interativos na TUI:**
   - Digitar `/` e comprovar a abertura suave da paleta flutuante.
   - Digitar `/help` e verificar a resposta com a listagem completa dos 23 comandos.
   - Digitar `/exit` e comprovar o encerramento limpo.

---

### 📌 Cenário 4 (v0.27.0): Validação do Desacoplamento Hexagonal & Universal Command Adapter (CLI & TUI)

- **Objetivo**: Comprovar a paridade absoluta e a execução universal de todos os 23 Slash Commands entre o Terminal CLI e a OpenTUI (`actx --tui`) através dos Core Application Services (`src/any_context/core/services/`), garantindo a supressão do banner duplicado da CLI na inicialização da TUI.
- **Pré-requisito**: Código atualizado para a versão `v0.27.0` com Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a OpenTUI (`actx --tui`):**
   ```powershell
   actx --tui
   ```

2. **⚡ Validar Inicialização Limpa (Sem Banner Duplicado):**
   - Comprovar que a tela inicial da TUI abre limpa e direta, sem o vazamento de texto do banner da CLI ou avisos de atualização duplicados.

3. **⚡ Validar Execução Direta e Autocompletação dos 23 Comandos na TUI:**
   - Teclar `/` para abrir a paleta flutuante.
   - Navegar com `↑` e `↓` ou filtrar por nome (`/help`, `/sources`, `/models`, `/billing`, `/version`).
   - Pressionar `Enter` em `/version` e comprovar a resposta formatada como mensagem do sistema no chat.
   - Executar `/help` e verificar a listagem completa dos 23 comandos agrupados por categoria.
   - Executar `/model gpt-4o-mini` e verificar a atualização imediata do modelo na barra inferior de status.
   - Executar `/mode strict` e comprovar a transição do modo de grounding no dock inferior.
   - Executar `/sources` e verificar a listagem das fontes ativas formatadas em Markdown.
   - Executar `/clear` e verificar a limpeza imediata do histórico visual.

4. **⚡ Validar Execução no Terminal CLI Clássico:**
   ```powershell
   actx
   ```
   - Executar `/help`, `/version`, `/sources`, `/mode hybrid` e comprovar que os mesmos Core Services respondem de forma idêntica.

5. **⚡ Testar Encerramento Limpo:**
   - Executar `/exit` em ambas as interfaces e validar o encerramento sem travamentos.

---

### 📌 Cenário 4 (v0.26.8): Validação de Inicialização Standalone da TUI sem Erro de Bootloader

- **Objetivo**: Comprovar a inicialização autônoma da interface OpenTUI (`actx --tui`) executando a partir do binário compilado PyInstaller (`actx.exe`), garantindo a ausência do erro `Security validation failure: parent process has different executable!`.
- **Pré-requisito**: Binário compilado atualizado para a versão `v0.26.8` e Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar a TUI via Binário:**
   ```powershell
   actx --update@0.26.8
   actx --tui
   ```

2. **⚡ Validar Ausência de Erros de Bootloader:**
   - Comprovar que o aplicativo não exibe `Security validation failure` e inicializa a tela cheia imediatamente.
   - Verificar que a conexão Stdio RPC entre `bun` e `actx --rpc` ocorre em sub-milissegundo sem falhas.

3. **⚡ Validar Interatividade Completa:**
   - Teclar `/` para abrir a Command Palette e selecionar `/model`.
   - Digitar uma pergunta e verificar resposta em streaming com o cabeçalho `🤖 AI [gpt-4o-mini]:`.
   - Finalizar com `/exit`.

---

### 📌 Cenário 4 (v0.26.7): Validação de Input Ativo, Background Transparente e Dock de Status na TUI

- **Objetivo**: Comprovar a interface reativa OpenTUI (`actx --tui`) com background transparente nativo do terminal, buffer de input com captura em tempo real (incluindo ativação automática da Command Palette ao teclar `/`), e visibilidade permanente do dock inferior de status de 1 linha.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.26.7` e Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a TUI OpenTUI:**
   ```powershell
   actx --update@0.26.7
   actx --tui
   ```

2. **⚡ Validar o Background Transparente e a Barra de Status:**
   - Verificar que a tela assume o fundo nativo do terminal (sem retângulos pretos ou caixas opacas cortadas).
   - Verificar que a barra de status inferior de 1 linha aparece com nitidez logo abaixo do input:
     `📂 Default  │  🤖 gpt-4o-mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu        🚪 /exit`

3. **⚡ Validar a Digitação e a Command Palette com `/`:**
   - Digitar `/` no campo de input.
   - Validar que a janela flutuante `📚 Slash Commands Palette` abre instantaneamente.
   - Digitar `mode` e navegar com as setas `↑` / `↓`.
   - Pressionar `Tab` ou `Enter` para autocompletar `/mode <strict|hybrid|proactive>`.

4. **⚡ Enviar Pergunta e Validar Streaming:**
   - Digitar uma pergunta no campo de input:
     ```text
     explique o conceito de RAG em 2 frases simples
     ```
   - Pressionar `Enter` e verificar que a pergunta é enviada e o buffer limpo.
   - Validar o streaming suave da resposta com o cabeçalho `🤖 AI [gpt-4o-mini]:`.

5. **⚡ Testar Encerramento Limpo:**
   - Digitar `/exit` para encerrar a sessão salvando a memória.

---

### 📌 Cenário 4 (v0.26.6): OpenTUI com Paridade Estética e Visual com a CLI UI

- **Objetivo**: Comprovar a interface reativa OpenTUI (`actx --tui`) reformulada com paridade visual completa com a CLI UI: renderização no scroll do clássico Banner ASCII Art (`ANYCONTEXT`), metadados de versão/edição (`🌿 Community Edition`), box de boas-vindas, prompt `👤 You:`, respostas `🤖 AI [modelo]:`, tickers ricos de status e dock inferior unificado em 1 linha.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.26.6` e Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a TUI OpenTUI:**
   ```powershell
   actx --update@0.26.6
   actx --tui
   ```

2. **⚡ Validar o Banner Inicial e o Prompt:**
   - Verificar se o Banner ASCII Art `ANYCONTEXT` em ciano, o badge `🌿 Community Edition` e o box de boas-vindas aparecem no topo do chat.
   - Verificar se o campo de input inferior exibe o prefixo `👤 You:` em ciano brilhante.
   - Validar a barra inferior (dock de 1 linha):
     `📂 Default  │  🤖 gpt-4o-mini  │  🛡️ Strict  │  🌐 Search: OFF  │  💡 /menu        🚪 /exit`

3. **⚡ Validar a Command Palette Flutuante com `/`:**
   - Digitar `/` no campo de input.
   - Validar a abertura da janela flutuante com filtro em tempo real.
   - Digitar `mode` e navegar com `↑` / `↓` até `/mode <strict|hybrid|proactive>`.
   - Pressionar `Tab` ou `Enter` para autocompletar.

4. **⚡ Enviar Pergunta e Validar Streaming & Cabeçalho da IA:**
   - Enviar uma pergunta solicitando código:
     ```text
     crie uma função em Python para calcular fatorial de forma recursiva e iterativa
     ```
   - Validar que o cabeçalho da resposta é exibido como `🤖 AI [gpt-4o-mini]:` em amarelo/violeta.
   - Validar o streaming suave e o realce de sintaxe colorido dos blocos de código em Markdown.

5. **⚡ Validar Seleção de Mouse e Clipboard:**
   - Selecionar com o mouse um trecho do código renderizado na tela.
   - Pressionar `Ctrl+C` e colar em outro editor para validar a cópia de texto limpa.

6. **⚡ Testar Encerramento Limpo:**
   - Digitar `/exit` para encerrar a sessão salvando a memória.

---

### 📌 Cenário 4 (v0.26.5): OpenTUI Desktop Frontend no Padrão Arquitetural Oficial do Cline

- **Objetivo**: Comprovar a interface OpenTUI reconstruída segundo o esqueleto oficial do Cline CLI (`apps/cli/src/tui/`), com `InputBar` (prompt `❯`, textarea multiline), `StatusBar` com pills de Grounding Mode, `ChatMessageList` com Markdown nativo e `AutocompleteDropdown` flutuante para comandos `/`.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.26.5` e Bun instalado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar a TUI OpenTUI:**
   ```powershell
   actx --update@0.26.5
   actx --tui
   ```






2. **⚡ Validar o Layout e a Command Palette:**
   - Digitar `/` no campo de input inferior.
   - Verificar a abertura instantânea da caixa flutuante `📚 Slash Commands Palette`.
   - Digitar `sw` e validar que a lista é filtrada em tempo real exibindo `/switch <workspace>`.
   - Navegar com as setas `↑` / `↓` e apertar `Tab` ou `Enter` para autocompletar.

3. **⚡ Enviar Pergunta e Validar Streaming & Markdown:**
   - Digitar uma pergunta técnica solicitando código:
     ```text
     crie uma classe em Python de singleton thread-safe e explique
     ```
   - Validar que a resposta é renderizada em Markdown com realce de sintaxe colorido para o bloco de código Python.
   - Validar que o ticker de status surge e desaparece suavemente.

4. **⚡ Validar Seleção de Mouse e Clipboard:**
   - Selecionar um trecho do código no terminal com o mouse.
   - Pressionar `Ctrl+C` e colar em outro editor para validar a cópia de texto limpa.

5. **⚡ Testar Encerramento Limpo:**
   - Digitar `/exit` ou pressionar `Ctrl+C` para fechar a aplicação salvando a memória.

---


### 📌 Cenário 4 (v0.25.1): CLI Nativa Consolidada & Execução de Batch/One-Shot

- **Objetivo**: Comprovar que a CLI executa no terminal nativo sem sequestro de tela, com seleção por mouse e copiar/colar nativos 100% funcionais, suporte a comandos interativos (`/sources`, `/switch`, `/sync`, etc.) e execução direta de prompt via argumentos de linha de comando (`actx "..."`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.25.1`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.25.1
   actx
   ```

2. **⚡ Validar Copiar, Colar e Seleção com Mouse:**
   - Selecione um trecho de texto no terminal com o mouse e copie (Ctrl+C).
   - Cole (Ctrl+V) no prompt e envie. Verifique que o texto foi inserido perfeitamente.

3. **⚡ Validar Comandos Internos:**
   - Execute `/sources` e veja a listagem de arquivos e fontes indexadas.
   - Execute `/help` e confira a lista de todos os 23 comandos.
   - Execute `/mode Hybrid` e verifique a alteração do modo.
   - Digite `/exit` para sair.

4. **⚡ Validar Execução Direta / One-Shot Batch:**
   - Execute direto no terminal sem abrir o loop interativo:
     ```powershell
     actx "quais os principais comandos do AnyContext?"
     ```
   - Verifique que a resposta é impressa diretamente no stdout e o processo finaliza com sucesso.

---


### 📌 Cenário 4 (v0.25.0): Textual Reactive TUI (Interface Estilo Cline / Claude Code)

- **Objetivo**: Comprovar a experiência completa da interface TUI reativa com layout persistente em tela cheia, painel de histórico de chat rolável com Markdown rico e realce de sintaxe, barra de input permanente e status footer dock ancorado na base.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.25.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext com a nova TUI:**
   ```powershell
   actx --update@0.25.0
   actx
   ```

2. **⚡ Validar o Layout Permanente da TUI:**
   - O cabeçalho superior deve exibir `🤖 AnyContext (actx) v0.25.0`.
   - O painel central exibe o card de boas-vindas.
   - O campo de input inferior permanece fixo acima do rodapé.
   - A barra de status no rodapé exibe os badges: `📂 Workspace`, `🤖 Model`, `🛡️ Grounding Mode`, `🌐 Search`, e atalhos.

3. **⚡ Enviar Pergunta com Código e Markdown:**
   - Digitar no input inferior e pressionar `Enter`:
     ```text
     escreva uma função em Python com docstring para calcular a sequência de Fibonacci e explique
     ```

4. **⚡ Critérios de Aceitação do Streaming e Markdown:**
   - O card do usuário (`👤 You`) surge no painel de chat.
   - O card da IA (`🤖 AI`) exibe o ticker de pensamento e inicia o streaming suave do texto.
   - O bloco de código Python é renderizado com realce de sintaxe colorido nativo do Textual.
   - O campo de input na base permanece 100% estático, estável e pronto para a próxima digitação.

5. **⚡ Testar Rolagem e Slash Commands:**
   - Rolar o painel de chat para cima e para baixo usando a roda do mouse e as teclas `Page Up` / `Page Down`.
   - Digitar `/mode Hybrid` e verificar a atualização instantânea do badge `🛡️ Hybrid` no rodapé.
   - Digitar `/web-search on` e verificar a atualização instantânea do badge `🌐 Search: ON`.
   - Digitar `/help` e verificar a exibição da lista de comandos.
   - Digitar `/exit` para encerrar a sessão salvando a memória.

---


### 📌 Cenário 4 (v0.24.8): Crescimento Natural de Conversa Top-Down e Preservação de Histórico no Terminal

- **Objetivo**: Comprovar que a conversa cresce naturalmente de cima para baixo (linha a linha após o banner), sem saltar para o final da tela deixando linhas em branco no 1º turno, e que após as respostas da IA todo o histórico de perguntas e respostas anteriores permanece 100% visível e rolável no buffer do terminal.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.8`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.8
   actx
   ```

2. **⚡ Observar o posicionamento inicial no 1º Turno:**
   - O primeiro prompt `👤 You:` deve aparecer imediatamente abaixo da caixa de boas-vindas do banner, sem um grande espaço vazio no meio da tela.
   - Enviar uma pergunta curta:
     ```text
     olá, quais são os comandos básicos?
     ```

3. **⚡ Critérios de Aceitação do Turno 1:**
   - O texto da IA faz o streaming fluido no local correto da tela.
   - Ao término da resposta, o histórico com a pergunta e a resposta permanece 100% visível na tela sem rolar para cima ou sumir.
   - O próximo prompt `👤 You:` aparece naturalmente logo abaixo da resposta anterior.

4. **⚡ Executar múltiplos turnos e validar a rolagem natural:**
   - Envie uma segunda pergunta:
     ```text
     explique o que é o grounding mode Strict
     ```
   - Envie uma terceira pergunta com resposta mais longa:
     ```text
     liste 5 boas práticas para organizar documentos em workspaces
     ```

5. **⚡ Critérios de Aceitação Multi-Turno:**
   - Todas as perguntas e respostas dos turnos 1, 2 e 3 permanecem intactas no histórico do terminal (rolável para cima com a roda do mouse / Page Up).
   - Nenhuma linha anterior foi sobrescrita ou apagada.
   - O comando `/clear` limpa a tela e redesenha o banner no topo conforme esperado.

---

### 📌 Cenário 5 (v0.24.7): Prioridade 0 em Portais Web Registrados e Desempate por Recência Temporal

- **Objetivo**: Comprovar que quando um portal web está registrado no workspace e o Web Search está ON, a IA pesquisa no portal registrado primeiro (Prioridade 0) antes de recorrer à web aberta, e aplica a regra de que a fonte mais recente sempre prevalece.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.7`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext e registrar um portal web no workspace:**
   ```powershell
   actx --update@0.24.7
   actx
   /web --add https://docs.python.org/3/
   /web-search --on
   /mode Hybrid
   ```

2. **⚡ Enviar uma pergunta sobre funcionalidade recente do portal:**
   ```text
   qual a novidade da versão mais recente do Python documentada no portal?
   ```

3. **⚡ Critérios de Aceitação:**
   - A IA consulta os documentos locais e dispara `live_web_search(target_domain='docs.python.org')` como Prioridade 0 primária.
   - Em caso de conflito de datas entre documentos locais mais antigos e páginas da web mais novas, a informação com data mais recente prevalece categoricamente.

---

### 📌 Cenário 6 (v0.24.6): Painel Fixo de Input & Barra de Status Ancorada no Rodapé (`PinnedBottomDock`)

- **Objetivo**: Comprovar que o divisor horizontal e a barra de status inferior (`bottom_toolbar`) permanecem permanentemente fixos e visíveis na base da tela durante a geração e streaming da IA, enquanto o texto sobe suavemente na área de rolagem superior sem apagar o rodapé.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.6`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.6
   actx
   ```

2. **⚡ Observar a tela e enviar uma pergunta que exija resposta longa:**
   ```text
   explique detalhadamente a arquitetura de banco de dados do AnyContext
   ```

3. **⚡ Critérios Visuais de Aceitação durante o Streaming:**
   - **Fixação do Rodapé**: A linha divisória `───...` e a barra de status (`📂 Workspace │ 🤖 Modelo │ 🛡️ Modo │ 🌐 Search │ 🚪 /exit`) **NÃO** somem e **NÃO** são empurradas para cima.
   - **Rolagem Suave Superior**: Conforme os parágrafos da resposta da IA vão surgindo, apenas a janela superior de conteúdo rola para cima.
   - **Atualização Dinâmica de Status**: Quando a IA pesquisa ou lê documentos, o status é refletido no rodapé e volta imediatamente ao estado normal.
   - **Transição Suave para o Próximo Input**: Ao terminar a resposta, o console já se encontra com a barra de status pronta e o cursor posicionado no próximo `👤 You:`.

---

### 📌 Cenário 7 (v0.24.5): Injeção Dinâmica via Strategy Pattern de Grounding & Matriz de Prioridades por Turno

- **Objetivo**: Comprovar que o AnyContext injeta dinamicamente a matriz de prioridades (0 vs 1) e regras de recência no turno ativo, garantindo 100% de aderência contra *prompt dilution* após múltiplos turnos sem poluir o histórico nem a UI.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.5`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.5
   actx
   ```

2. **⚡ Configurar Modo Strict com Web Search ON:**
   ```text
   /mode Strict
   /web-search --on
   ```

3. **⚡ Conversar durante 5 turnos com perguntas variadas e depois perguntar sobre fato não indexado:**
   - **Turno 1**: Pergunte sobre o help (`/help`).
   - **Turno 2**: Faça uma pergunta sobre arquivos do workspace.
   - **Turno 3**: Faça outra pergunta contextual.
   - **Turno 4**: Alterne para `/mode Hybrid` e faça uma pergunta comparativa.
   - **Turno 5**: Alterne de volta para `/mode Strict` e pergunte:
     ```text
     qual a cotação do dólar hoje?
     ```
   - **Critério de Sucesso:**
     - Mesmo no 5º turno longo, a IA **NÃO** alucina e **NÃO** busca na web autonomamente.
     - A IA responde estritamente:
       *`⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre 'cotação do dólar hoje'?`*

4. **⚡ Validar o critério de Recência no Modo Hybrid:**
   ```text
   /mode Hybrid
   /web-search --on
   qual a versão mais recente do Python lançada?
   ```
   - **Critério de Sucesso:**
     - A IA busca na web autonomamente (prioridade 1 com recência temporal vencendo a memória pré-treinada).
     - Apresenta a resposta estruturada com as seções e o rodapé de fontes web consultadas.

---

### 📌 Cenário 8 (v0.24.4): Gating Dinâmico Determinístico no Modo Strict & Bloco Mandatório de Fontes

- **Objetivo**: Comprovar que no modo `Strict` a IA jamais dispara busca na internet autonomamente na pergunta inicial (com ferramenta dinamicamente restrita), perguntando ao usuário e liberando a busca apenas sob confirmação explícita (`sim`), com rodapé completo de fontes.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.4`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.4
   actx
   ```

2. **⚡ Ativar Modo Strict e Busca Web:**
   ```text
   /mode Strict
   /web-search --on
   ```

3. **⚡ Fazer uma pergunta sobre dado não indexado no workspace:**
   ```text
   qual o preço do Windex?
   ```
   - **Critério de Sucesso:**
     - A IA **NÃO** faz a busca na web automaticamente.
     - A IA responde informando que o dado não consta no workspace e pergunta:
       *`⚠️ Essa informação não consta nos documentos deste workspace. Deseja que eu faça uma busca na internet sobre 'preço do Windex'?`*

4. **⚡ Confirmar a busca na web:**
   ```text
   sim
   ```
   - **Critério de Sucesso:**
     - A IA executa a busca na web e exibe a resposta com os preços.
     - No final da resposta, é exibido o rodapé obrigatório:
       ```markdown
       ---
       🌐 **Fontes Consultadas na Web:**
       - [Título da Loja/Página](https://...)
       ```

---

### 📌 Cenário 9 (v0.24.2): Barra de Progresso Universal de Duas Etapas (`TwoStageProgressRenderer`)

- **Objetivo**: Comprovar que durante a ingestão web e de pastas locais, o terminal exibe animação contínua e atualização fluida em tempo real tanto na **Etapa 1 (Crawling/Coleta)** quanto na **Etapa 2 (Vetorização & IA)**, sem congelar no zero.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.2`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.2
   actx
   ```

2. **⚡ Iniciar Ingestão de Portal Web (`/web add`):**
   ```text
   /web add https://www.saskatchewan.ca/residents/moving-to-saskatchewan
   ```
   - **Critério de Sucesso:**
     - Na **Etapa 1**, a barra `[1/2 Crawling]` avança página por página com spinner animado (`⠋`, `⠙`, `⠹`...).
     - Na **Etapa 2**, a barra `[2/2 Embedding]` inicia exibindo `Enriching Context` e progride fluentemente informando páginas e chunks (ex: `120/292 pages (450/1100 chunks) (41%) • Vector Knowledge Base`) até 100%.

---

### 📌 Cenário 10 (v0.24.1): Pureza Hexagonal do Core e Isolamento do Adaptador CLI

- **Objetivo**: Comprovar que o Core Domain do AnyContext opera de forma 100% agnóstica a UI (sem vazamentos de ANSI ou questionários no Core), enquanto o adaptador CLI ([`formatters.py`](file:///C:/Users/guilh/source/repos/any-context/src/any_context/cli/formatters.py)) renderiza perfeitamente os cards de status, planos de preços e relatórios de crawling.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.1`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.1
   actx
   ```

2. **⚡ Visualizar a Tabela de Preços e Planos (`/billing`):**
   ```text
   /billing
   ```
   - **Critério de Sucesso:**
     - Selecione `📊 View Complete Pricing & Capability Matrix Table`.
     - A matriz de planos é exibida com formatação visual e cores destacando o plano ativo.

3. **⚡ Testar o Assistente Interativo de Web Crawling (`/web add`):**
   ```text
   /web add https://fastapi.tiangolo.com/
   ```
   - **Critério de Sucesso:**
     - O relatório de descoberta de páginas web é apresentado com cores claras no terminal.
     - As opções de escopo (incremental ou total) são exibidas via menu interativo (`questionary`) isolado no adaptador CLI.

---

### 📌 Cenário 11 (v0.24.0): Desacoplamento Arquitetural & Orquestrador de Ingestão Multi-Fonte

- **Objetivo**: Comprovar a estabilidade do orquestrador multi-fonte desacoplado ([`orchestrator.py`](file:///C:/Users/guilh/source/repos/any-context/src/any_context/ingestion/orchestrator.py)) durante a inspeção e sincronização concorrente de pastas locais, portais web e drives na nuvem.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.24.0`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.24.0
   actx
   ```

2. **⚡ Executar Inspeção de Status Multi-Fonte (`/sync --status`):**
   ```text
   /sync --status
   ```
   - **Critério de Sucesso:**
     - O card multi-fonte é renderizado pelo `orchestrator.py` exibindo separadamente:
       `📂 Local Folders`, `🌐 Web Sources`, `☁️ Cloud Drives` e `📦 Pending Status`.

3. **⚡ Disparar Sincronização em Background (`/sync --bg`):**
   ```text
   /sync --bg
   ```
   - **Critério de Sucesso:**
     - O prompt `👤 You:` permanece interativo instantaneamente.
     - A micro-barra `⚡ Syncing [████░░░░]` é exibida na barra inferior até a conclusão (100%).

---

### 📌 Cenário 12 (v0.23.1): Auto-Consciência e Auto-Bootstrap Permanente do Sistema (Help & README Global)

- **Objetivo**: Comprovar que em qualquer workspace (inclusive workspaces 100% vazios ou contendo apenas páginas web), a IA responde imediatamente perguntas sobre comandos do AnyContext (ex: `/transfer`, `/switch`, `/sync`, `/web`, `/link`, `/config`) citando a sintaxe, opções e parâmetros exatos da documentação do sistema indexada em `Global`.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.23.1`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Iniciar o AnyContext:**
   ```powershell
   actx --update@0.23.1
   actx
   ```

2. **❓ Fazer Pergunta sobre Comandos do Sistema:**
   ```text
   Como mover um web source de um workspace para outro aqui no AnyContext?
   ```
   - **Critério de Sucesso:**
     - A IA consulta os chunks do `Global` e responde detalhadamente com a sintaxe do comando `/transfer` (ex: `/transfer <origem> <destino> <url>`), explicando o tempo de execução sub-50ms e custo zero ($0.00).
     - A IA NÃO responde com mensagens de que a informação não consta no workspace.

3. **❓ Perguntar sobre Outros Comandos (/link, /sync, /web):**
   ```text
   Qual comando uso para vincular uma pasta de outro workspace sem duplicar vetores?
   ```
   - **Critério de Sucesso:**
     - A IA responde citando o comando `/link <source_id>` e explicando o compartilhamento de fontes.

---

### 📌 Cenário 13 (v0.23.0): Sincronização Web de Alta Velocidade com HTTP 304, Sitemap Diff & Embeddings Paralelos

- **Objetivo**: Validar que a sincronização de portais web massivos (> 2.000 páginas) ocorre em alta velocidade usando HTTP Conditional GET (`304 Not Modified`), pre-filtragem por sitemap `<lastmod>` e paralelização concorrente de embeddings no `ParallelIndexer` com retry anti-429.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.23.0` com portal web indexado.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.23.0
   actx
   ```

2. **⚡ Disparar Sincronização Incremental (`/sync` ou `/web sync`):**
   ```text
   /sync
   ```
   - **Critério de Sucesso:**
     - O crawler utiliza até 20 workers simultâneos na rede.
     - Páginas com `<lastmod>` idêntico no sitemap ou que retornam `HTTP 304 Not Modified` são identificadas em milissegundos sem re-baixar o HTML.
     - Documentos novos/modificados são vetorizados em lotes paralelos no `ParallelIndexer` com redução substancial do tempo total.
     - A micro-barra `⚡ Syncing [████░░░░]` atualiza o progresso em tempo real.

3. **🔍 Verificar Persistência no LanceDB:**
   - Faça uma pergunta de busca sobre uma página recém-atualizada do site.
   - **Critério de Sucesso:** A resposta utiliza os vetores atualizados com precisão absoluta.
