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
- Consulta a vários índices por workspace (atual + `Global`) fundida por posição (ingestão,
  passo 9).
- Famílias de versões: preferir a versão vigente por padrão e usar a família inteira em
  perguntas sobre mudanças entre versões (ingestão, passo 3).
- Entidades e metadados do documento como filtros e facetas; mascaramento de dados
  pessoais na resposta conforme a política do workspace (ingestão, passo 7).
- Formulários do mesmo modelo de layout como tabela consultável (ingestão, passo 6).
- Conjunto de avaliação por workspace formado pelas perguntas sintéticas da autoavaliação
  (ingestão, seção transversal).

## Design alvo

_Pendente._
