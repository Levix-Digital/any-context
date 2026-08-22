# 🧪 AnyContext Acceptance & Manual Test Suite (`MANUAL_TESTS.md`)

> **Roteiros oficiais de teste manual e critérios de aceitação para validação humana a cada release do AnyContext (`actx`).**

---

## 🎯 Teste Ativo (Última Release: v0.17.7)

### 📌 Cenário: Perguntas Compostas Multi-Tópico & Orçamento Proporcional de Contexto

- **Objetivo**: Comprovar que perguntas com múltiplos tópicos simultâneos (ex: autorização de menores + regras de outra jurisdição) preservam os fatos de **todos os tópicos pesquisados** no prompt final sem estourar o limite de 128k tokens da OpenAI.
- **Pré-requisito**: Binário ou ambiente atualizado para a versão `v0.17.7`.

#### 📋 Passo a Passo de Execução:

1. **🚀 Atualizar e Iniciar o AnyContext:**
   ```powershell
   actx --update@0.17.7
   actx
   ```

2. **💬 Turno 1 (Pergunta Composta com Múltiplos Sub-Tópicos):**
   ```text
   👤 You: Quem deve assinar autorizações para crianças? O que as leis do Canadá dizem sobre isso?
   ```
   - **Critério de Sucesso:**
     - A resposta é gerada em **menos de 4 segundos** (sem loops de 10 buscas).
     - **Zero Erro 400 (`ContextOverflowError - 139001 tokens`)**.
     - A IA aborda **ambos os aspectos** (autorização de menores E a legislação/regras pertinentes) com citações exatas das fontes.

3. **💬 Turno 2 (Segunda Pergunta Consecutiva na Mesma Sessão):**
   ```text
   👤 You: E quais são os prazos e penalidades caso essa autorização não seja entregue a tempo?
   ```
   - **Critério de Sucesso:**
     - O Turno 1 é compactado em runtime para `"[Prior workspace context retrieved and synthesized in conversation history]"`.
     - O prompt permanece calibrado em ~10.000 tokens e a resposta sai instantaneamente.

4. **💬 Turno 3 (Verificação de Memória do Diálogo):**
   ```text
   👤 You: Resuma em 3 tópicos os pontos principais que discutimos até aqui.
   ```
   - **Critério de Sucesso:**
     - A IA sintetiza as respostas dos Turnos 1 e 2 perfeitamente, comprovando a integridade da memória conversacional.

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
