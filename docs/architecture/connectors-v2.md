# Conectores — design alvo (v2)

Documento vivo da revisão dos conectores. Decisões registradas conforme são tomadas; os
pontos ainda em discussão ficam em **Pendentes**.

## Estado atual

- Não há código de conector. As fontes são pasta ou arquivo local, site ou página web e a
  ajuda do sistema (`builtin`). Um plano antigo (`docs/plans/...chain_of_responsibility...`)
  previa um `CloudDriveSyncHandler`, nunca implementado.
- A tabela `sources` já reserva `kind = connector:<nome>`, com alvo e modo avulso ou
  recursivo (`ingestion-pipeline-v2.md`, passo 1).
- Já decidido em outros documentos: permissões herdadas da origem (ID do escopo de
  permissão no chunk, detalhe em SQLite separado — `ingestion-pipeline-v2.md`, controle de
  acesso por fonte); política do Admin Hub com conectores e domínios de conta permitidos
  (`identity-and-governance-v2.md`, tema 4); sync automático e governador de recursos
  (`ingestion-pipeline-v2.md`, seções transversais).

## Decisões

1. **O conector só capta.** É um adaptador da aquisição (ingestão, passo 2): traz o
   conteúdo externo, e do passo 3 em diante é o mesmo pipeline de qualquer arquivo. Nada de
   parsing, chunking ou limpeza próprios, salvo limpeza específica de uma origem que o
   pipeline não cobre. Contrato único:
   - **listar** os itens do alvo (avulso ou recursivo);
   - **ler** o conteúdo em stream (uma leitura só);
   - **mudanças** desde o último cursor;
   - **permissões** (escopos de permissão da origem);
   - **metadados** (autor, datas, link para abrir na origem e os metadados próprios do
     conector — decisão 13).

   Cada item tem um **ID nativo estável** da origem, que substitui o hash na detecção de
   rename e move.
2. **Só indexado, nunca consulta ao vivo.** O conector sincroniza para dentro do
   workspace; a pergunta nunca é enviada à API da origem. Mantém a qualidade da recuperação
   (busca híbrida, reranker, citação precisa), não manda a pergunta a terceiros e respeita
   "nenhuma consulta atravessa workspaces".
3. **Só leitura, com os menores escopos possíveis.** Nunca escreve na origem.
4. **Autenticação OAuth 2.0 com PKCE pelo navegador do sistema**, com redirecionamento para
   o próprio computador (`127.0.0.1`), **sem passar por servidor da Levix**.
   - O token fica no **cofre pessoal** do usuário (`identity-and-governance-v2.md`, tema 2):
     é da pessoa, não do workspace.
   - **Ambiente pessoal:** app OAuth público registrado pela Levix.
   - **Ambiente corporativo:** app registrado pela **própria empresa** (no Entra ID da
     Microsoft, no Google Workspace dela), distribuído pela política do Admin Hub.
   - **AnyContext Server (índice compartilhado na VPC da empresa):** conta de serviço da
     empresa, com o filtro de permissões por leitor obrigatório.
   - **Viabilidade a confirmar antes de construir** (custos e auditorias exigidos pelo
     Google e por outros fornecedores): ver Pendentes.
5. **Compartilhar um workspace sem servidor leva o alvo, nunca a credencial.** A definição
   do workspace (que é o que se compartilha quando não há AnyContext Server) leva o conector
   e o ID do alvo; cada membro entra com a própria conta e indexa com o próprio acesso.
6. **Sync pela lista de mudanças da origem, não por varredura** (ex.: `changes` do Google
   Drive, `delta` do Microsoft Graph, cursor do Dropbox, `CONDSTORE` do IMAP).
   - **No computador do usuário:** consulta periódica com cursor enquanto o AnyContext está
     aberto (não há como receber webhooks); diff de recuperação ao abrir; serviço em segundo
     plano opcional, como nas fontes locais.
   - **No AnyContext Server:** webhooks da origem.
   - Respeita os limites de taxa das APIs (espera crescente).
   - **Rede tarifada** (dados móveis, conexão medida): o governador de recursos segura o
     sync dos conectores, e **o usuário pode liberar** (por fonte ou por enquanto).
7. **Formatos nativos da nuvem:** Google Docs, Sheets e Slides exportados para DOCX, XLSX e
   PPTX e passados aos parsers existentes; páginas de wiki (Confluence, Notion) convertidas
   da árvore de blocos para Markdown determinístico; atalhos e links da nuvem resolvidos e
   deduplicados como duplicatas (ingestão, passo 3).
8. **Sem cópia do original.** Fica só o conteúdo extraído, no índice criptografado. A aba de
   leitura mostra o texto indexado e oferece **"abrir na origem"** (link nativo). Download em
   stream, com a reserva de disco do governador.
9. **Conectores só da Levix**, em Rust, numa crate `actx-connectors` com um trait e um módulo
   por conector; sem plugins de terceiros. Cliente HTTP comum com **proxy corporativo** e
   **certificados da empresa** (autoridade certificadora interna, lida do repositório de
   certificados do sistema e da política).
10. **Seleção explícita do alvo.** O usuário navega pela árvore da origem e escolhe o que
    quer (pasta, que vale recursivo, ou arquivo avulso, ou alvos próprios da origem: drive
    compartilhado, biblioteca do SharePoint, rótulo do Gmail, canal ou playlist). **Tudo o
    que ele selecionar e tiver acesso é baixado e indexado**; nada fora da seleção. Os
    filtros de inclusão e exclusão (sintaxe `.gitignore`) valem também para os caminhos
    remotos.
11. **Política do Admin Hub:** conectores permitidos, **domínios de conta permitidos** (ex.:
    só contas `@empresa.com` no ambiente corporativo) e teto de volume por fonte. O bloqueio
    sempre diz qual regra bloqueou.
12. **Schema:** `sources` ganha `account_ref` (referência à credencial no cofre) e
    `target_id` (ID nativo do alvo, além do caminho exibido). O cursor de mudanças fica no
    estado de sync do workspace.
13. **Metadados próprios de cada conector.** Cada conector declara um **esquema de
    metadados tipado** (ex.: YouTube: `view_count`, `like_count`, `published_at`; Jira:
    `status`, `assignee`; e-mail: `from`, `labels`), usável como filtro pela recuperação
    (entendimento da pergunta) do mesmo jeito que os metadados de um pacote de domínio.
14. **Metadados vivos separados do conteúdo.** O que muda sem o conteúdo mudar (views,
    likes, status, rótulos) fica só em colunas filtráveis, **fora do texto embedado e do
    cabeçalho do chunk**; uma mudança atualiza só essas colunas, sem re-embedar — a mesma
    lógica das permissões (ID no chunk, detalhe à parte).
15. **Fontes de alto volume e ruído** (ex.: comentários): indexa-se o conjunto mais
    relevante (ex.: os comentários mais curtidos e suas respostas), cada item como chunk
    próprio com autor e relevância no cabeçalho; o relatório da fonte diz quanto ficou de
    fora ("100 de 10.000 comentários") e o teto é configurável pelo usuário, como o teto de
    crawl. Autores de comentários são dados pessoais de terceiros: vale a pseudonimização
    reversível do que vai para endpoint externo (ingestão, passo 7).
16. **YouTube:** só pela API oficial. Metadados e comentários de qualquer vídeo público;
    transcrição pela legenda oficial só quando o usuário é dono do canal (a API só entrega
    legendas a quem pode editar o vídeo). Nada de cliente não oficial (InnerTube) nem de
    disfarce de outro cliente.
17. **Obsidian:** o cofre é uma pasta local; o parser de Markdown ignora `.obsidian/` por
    padrão, lê o frontmatter YAML como metadados e transforma `[[wikilinks]]` em relações
    do grafo (ingestão, passo 7, P16).

## Pendentes

- **Viabilidade de custos nas origens** (decisão 4):
  - **Google Drive:** o escopo de leitura de todo o Drive é "restrito" e exige a avaliação
    de segurança anual CASA para app público; o Google não publica preço e há relatos de
    alguns milhares a dezenas de milhares de dólares por ano. Alternativa a avaliar: escopo
    `drive.file` com o seletor de arquivos do Google (o usuário escolhe; não é restrito).
    Apps internos de uma empresa no próprio Google Workspace não passam por essa avaliação.
  - **YouTube:** dados públicos sem login, com chave de API; cota padrão de 10.000 unidades
    por dia por projeto, ampliada por auditoria de conformidade gratuita.
- Estados da fonte quando algo falha (apagado na origem, sem acesso, conta desconectada).
- Arquivos "só online" de OneDrive, Google Drive e Dropbox dentro de pastas locais.
- Mapas de extração atualizáveis pelo catálogo verificado.
- Modelo "tudo é conector" e licença por conector (inclusive período de teste).
- Quais conectores existem e em que ordem: decidido no backlog e no roadmap, não aqui.
