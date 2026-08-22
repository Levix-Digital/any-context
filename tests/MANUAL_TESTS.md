# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.17.5)

### 📌 Cenário: Resiliência de Contexto & Multi-Turn Chat (Prevenção de Overflow de 128k Tokens)

- **Objetivo**: Comprovar que o `PruningChatModelWrapper` poda chunks de ferramentas antigas em runtime, mantendo o prompt da LLM calibrado em ~10.000 tokens e permitindo conversas longas de múltiplos turnos sem erro 400 (`OpenAIContextOverflowError`).
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.17.5`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.17.5
   actx
   ```

2. **💬 Turno 1 (Primeira Pergunta no Workspace):**
   ```text
   👤 You: Qual é a regra geral do nosso projeto/documento?
   ```
   - **Critério de Sucesso:**
     - Exibe `📚 [RAG] Reading retrieved context documents for AI analysis...`
     - A IA responde normalmente baseada nos documentos.

3. **💬 Turno 2 (Segunda Pergunta Consecutiva na Mesma Sessão):**
   *(Sem sair do terminal ou fechar o chat)*
   ```text
   👤 You: Quem deve assinar autorizações para crianças? O que as leis do Canadá dizem sobre isso?
   ```
   - **Critério de Sucesso:**
     - O sistema realiza a busca normalmente.
     - A resposta é sintetizada com rapidez (~2 a 4s) sem erro de limite de taxa.

4. **💬 Turno 3 (Terceira Pergunta Consecutiva - Ponto Crítico Anterior):**
   ```text
   👤 You: E quanto aos prazos de recursos ou penalidades descritos nos documentos?
   ```
   - **Critério de Sucesso:**
     - **Zero Erro 400 (`ContextOverflowError - 138992 tokens`):** A IA responde imediatamente.
     - O wrapper de pruning reduziu os chunks dos turnos 1 e 2 para marcadores compactos de 71 caracteres.

5. **💬 Turno 4 (Verificação de Continuidade do Diálogo Humano):**
   ```text
   👤 You: Você pode resumir em tópicos o que me explicou na pergunta anterior?
   ```
   - **Critério de Sucesso:**
     - A IA lembra perfeitamente do diálogo humano e resume os pontos com precisão, confirmando que apenas o lixo de chunks brutos foi podado sem afetar a memória da conversa.

---

## 📚 Histórico de Cenários de Testes Anteriores

### 🔬 v0.17.3 - Busca Paralela Multi-Fonte (`ThreadPoolExecutor`) & Presets de RAG
1. **Configuração de Presets:**
   - Execute `/config` ➔ Navegue até `Context & RAG Settings` ➔ Alterne entre **Turbo**, **Balanced** e **Deep Research**.
2. **Execução de Busca Paralela:**
   - No chat, digite uma pergunta abrangente: observe no log a mensagem `🔍 [Search] Parallel Multi-Source Scan: 'Workspace' (pool: 100 -> diversified top 20)...`.
   - Tempo de resposta: Fusão vetorial em sub-15ms na CPU.

### 📦 v0.17.1 - Scripts de Instalação e Desinstalação Automáticos
1. **Verificação de Assets no GitHub:**
   - Acessar `https://github.com/Levix-Digital/any-context/releases/latest` e validar que `install.ps1`, `install.sh`, `uninstall.ps1` e `uninstall.sh` estão anexados ao lado dos executáveis.
