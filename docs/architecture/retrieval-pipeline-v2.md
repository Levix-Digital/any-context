# Pipeline de recuperação — design alvo (v2)

Documento vivo da revisão do pipeline de recuperação, no mesmo método do
`ingestion-pipeline-v2.md`. A seção "Estado atual" registra como o código funciona hoje
(levantado em 2026-10-20); o design alvo é preenchido passo a passo depois de discutido.

## Estado atual

1. **Roteamento da pergunta.** Um classificador por palavras-chave fixas em português e
   inglês ("arquitetura", "compare", "onde está"…), somado ao tamanho da pergunta, decide
   entre três caminhos:
   - **Fast (1-hop):** uma busca e uma única chamada ao LLM.
   - **ReAct:** o LLM chama ferramentas em loop, até 10 turnos.
   - **Deep Search:** decompõe a pergunta em até 4 subperguntas, busca, reflete sobre o que
     falta, busca de novo e sintetiza.

   Perguntas de status ("o sync terminou?") vão para um caminho próprio. O usuário pode
   forçar o modo com `/fast` e `/deep`.
2. **Pré-processamento da consulta.**
   - Detecta nomes de arquivo na pergunta e datas (formatos ISO, numéricos e meses em
     inglês).
   - Os arquivos citados entram com score máximo fixo (1,0). As datas viram filtros por
     trechos do caminho do arquivo (`/2025/03/`), também com score fixo (0,95).
   - A pergunta é usada como está: não é reescrita com o contexto da conversa (IMP-009).
3. **Busca densa.** A pergunta é embedada com o modelo registrado do workspace e buscada no
   LanceDB, no workspace atual e no `Global`. Se o embedding falha, a busca continua só com
   o BM25 e a falha é registrada.
4. **Busca lexical:** BM25 no mesmo workspace.
5. **Fusão RRF** das duas listas (k = 60). Candidatos que só vieram do BM25 têm o texto
   buscado no LanceDB.
6. **Score mínimo:** corte no score do RRF, que varia por preset (Turbo, Balanced, Deep).
7. **Reranking:** cross-encoder multilíngue local via ONNX reordena até 30 candidatos. Sem
   corte por relevância depois disso (IMP-007).
8. **Diversificação e orçamento.**
   - No máximo 2, 3 ou 5 chunks por arquivo, conforme o preset.
   - Top-k de 10, 20 ou 40.
   - Teto de 20, 40 ou 60 mil caracteres de contexto (em caracteres, não em tokens).
9. **Gate de "zero contexto".** Se nenhum chunk passa de um score mínimo, responde "nada
   encontrado" sem chamar o LLM.
10. **Montagem do prompt.** Cada chunk numa tag XML com origem, versão e localização; ordem
    em formato de U (mais relevantes nas pontas); texto recuperado escapado.
11. **Geração.**
    - **Fast:** histórico da conversa + prompt numa única chamada.
    - **ReAct e Deep Search:** prompts próprios, com o mesmo formatador de contexto.
12. **Resposta e histórico:** a resposta e a pergunta original, sem o XML, vão para o
    histórico da sessão no SQLite.

### Pendências herdadas da ingestão

- Perguntas agregadas sobre tabelas grandes ("soma das vendas de março"): guardar a tabela
  num formato consultável e responder por consulta (ingestão, passo 5).
- Uso dos chunks "pai" (small-to-big) na montagem do contexto (ingestão, passo 5).
- **Nenhuma consulta atravessa workspaces.** Hoje toda pergunta busca no workspace atual e
  no `Global` e mistura os resultados; a documentação do AnyContext vira a ajuda do
  sistema, consultada só em perguntas sobre o próprio AnyContext, por rota ou ferramenta
  própria (ingestão, passo 1).
- Um chat por workspace, individual de cada usuário mesmo em workspace compartilhado
  (`deployment-architecture-v2.md`).
- Famílias de versões: preferir a versão vigente por padrão e usar a família inteira em
  perguntas sobre mudanças entre versões (ingestão, passo 3).
- Entidades e metadados do documento como filtros e facetas; busca sem máscara e
  pseudonimização reversível do contexto enviado a um endpoint externo, com os valores reais
  devolvidos localmente na resposta (ingestão, passo 7; níveis de confiança em
  `deployment-architecture-v2.md`).
- Teto "confidencial" por workspace e busca web tratada como saída para o externo
  (`deployment-architecture-v2.md`, costura 3).
- Resumos de documento e de pasta para perguntas de visão geral e para escolher em quais
  documentos procurar (ingestão, passo 7).
- Orientações de resposta dos pacotes de domínio (ingestão, seção transversal H3).
- Formulários do mesmo modelo de layout como tabela consultável (ingestão, passo 6).
- Conjunto de avaliação por workspace formado pelas perguntas sintéticas da autoavaliação
  (ingestão, seção transversal).

## Design alvo

### Visão geral (caminho feliz)

Pergunta do usuário → Fast ou Deep → (Deep: refinamento da pergunta) → busca densa + lexical
→ RRF → seleção dos melhores chunks → ordem em U + XML → modelo de inferência. Os passos
abaixo detalham esse caminho e acrescentam os passos que o código já tem e as pendências
da ingestão.

1. Entrada da pergunta
2. Roteamento
3. Entendimento da pergunta
4. Busca
5. Fusão
6. Reranking
7. Seleção
8. Montagem do contexto
9. Geração e citações
10. Histórico e avaliação

### 1. Entrada da pergunta

Hoje a pergunta é buscada como foi digitada (o histórico vai para o LLM, não para a busca
— IMP-009); o histórico é uma janela de 30 turnos; a documentação do AnyContext é
misturada em toda busca; perguntas de status são detectadas por palavras-chave fixas em
PT/EN ("qual o status do processo 123?" cai na rota de status do AnyContext).

- **Reescrita da continuação, só para a busca.** Uma pergunta que depende da conversa ("e o
  prazo da segunda?") é reescrita como pergunta completa e autônoma antes de buscar; a
  original é a que fica no histórico e que o LLM de resposta vê. Quem reescreve: o LLM local
  do perfil; sem ele, o LLM de chat, respeitando o nível de confiança e a pseudonimização.
  Sem LLM local e com chat externo, a alternativa sem LLM é buscar com a pergunta anterior
  do usuário somada à atual. Funciona em qualquer idioma.
- **Histórico por orçamento de tokens:** mensagens recentes inteiras; as antigas viram um
  resumo da conversa, gerado em segundo plano.
- **Ajuda e status são ferramentas, não rotas por palavra-chave:** `anycontext_help`
  (documentação do AnyContext) e `workspace_status` (estado do sync e das fontes), chamadas
  pelo LLM em qualquer idioma; a interface também oferece painel de ajuda e status das
  fontes na árvore.
- **Sem anexo temporário no chat.** O contexto é o que está no workspace — um conceito só.
  **Arrastar um arquivo para o chat é um atalho para adicioná-lo ao workspace como arquivo
  avulso**, com confirmação curta (ação explícita), progresso da indexação visível e, para
  arquivos grandes, a estimativa de tempo da ingestão. O arquivo fica disponível nas
  próximas conversas.
- **`@fonte` na pergunta** restringe a busca àquela fonte, sempre dentro do workspace
  (detalhado no passo 3).
- **Fila de perguntas.** Enquanto o agente responde, o usuário pode enviar novas perguntas,
  que ficam enfileiradas:
  - processadas **em ordem, uma de cada vez** — cada uma pode depender da resposta
    anterior, e a reescrita da continuação usa essa resposta;
  - a fila fica visível no chat; perguntas ainda não iniciadas podem ser editadas,
    removidas ou reordenadas; há "parar a atual e enviar agora";
  - a fila é do chat do workspace e sobrevive ao fechar o app (como rascunhos locais);
  - cada pergunta conta para as cotas de tokens da política (Admin Hub).

### 2. Roteamento

_Pendente._

### 3. Entendimento da pergunta

_Pendente._

### 4. Busca

_Pendente._

### 5. Fusão

_Pendente._

### 6. Reranking

_Pendente._

### 7. Seleção

_Pendente._

### 8. Montagem do contexto

_Pendente._

### 9. Geração e citações

_Pendente._

### 10. Histórico e avaliação

_Pendente._
