# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**  
> _Nota: Este arquivo é acumulativo. Todos os cenários passo a passo permanecem completos até que o usuário execute e esvazie o arquivo manualmente._

---

## 🎯 Testes Pendentes de Validação Humana

### 📌 Cenário 1 (v0.27.3): Validação de Layout do Slash Command Palette, Scoring por Prefixo e Indicador de Sync

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

### 📌 Cenário 1 (v0.27.2): Validação de Paridade Hexagonal Absoluta (CLI & TUI) e Encerramento Limpo (/exit)

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

### 📌 Cenário 1 (v0.27.1): Validação de Inicialização Direta da OpenTUI no Bun e Fallback para Community Edition

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

### 📌 Cenário 1 (v0.27.0): Validação do Desacoplamento Hexagonal & Universal Command Adapter (CLI & TUI)

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

### 📌 Cenário 1 (v0.26.8): Validação de Inicialização Standalone da TUI sem Erro de Bootloader

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

### 📌 Cenário 1 (v0.26.7): Validação de Input Ativo, Background Transparente e Dock de Status na TUI

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

### 📌 Cenário 1 (v0.26.6): OpenTUI com Paridade Estética e Visual com a CLI UI

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

### 📌 Cenário 1 (v0.26.5): OpenTUI Desktop Frontend no Padrão Arquitetural Oficial do Cline

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


### 📌 Cenário 1 (v0.25.1): CLI Nativa Consolidada & Execução de Batch/One-Shot

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


### 📌 Cenário 1 (v0.25.0): Textual Reactive TUI (Interface Estilo Cline / Claude Code)

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


### 📌 Cenário 1 (v0.24.8): Crescimento Natural de Conversa Top-Down e Preservação de Histórico no Terminal

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

### 📌 Cenário 2 (v0.24.7): Prioridade 0 em Portais Web Registrados e Desempate por Recência Temporal

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

### 📌 Cenário 3 (v0.24.6): Painel Fixo de Input & Barra de Status Ancorada no Rodapé (`PinnedBottomDock`)

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

### 📌 Cenário 4 (v0.24.5): Injeção Dinâmica via Strategy Pattern de Grounding & Matriz de Prioridades por Turno

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

### 📌 Cenário 5 (v0.24.4): Gating Dinâmico Determinístico no Modo Strict & Bloco Mandatório de Fontes

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

### 📌 Cenário 6 (v0.24.2): Barra de Progresso Universal de Duas Etapas (`TwoStageProgressRenderer`)

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

### 📌 Cenário 7 (v0.24.1): Pureza Hexagonal do Core e Isolamento do Adaptador CLI

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

### 📌 Cenário 8 (v0.24.0): Desacoplamento Arquitetural & Orquestrador de Ingestão Multi-Fonte

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

### 📌 Cenário 9 (v0.23.1): Auto-Consciência e Auto-Bootstrap Permanente do Sistema (Help & README Global)

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

### 📌 Cenário 10 (v0.23.0): Sincronização Web de Alta Velocidade com HTTP 304, Sitemap Diff & Embeddings Paralelos

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
