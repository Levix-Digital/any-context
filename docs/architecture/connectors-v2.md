# Conectores — design alvo (v2)

Documento da revisão dos conectores. Todas as decisões estão tomadas; o que é comercial ou
de ordem de entrega fica para o roadmap.

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
     Google e por outros fornecedores): decidido na decisão 20.
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

18. **Estados da fonte quando algo falha:**
    - item **apagado na origem** → sai do índice;
    - **sem acesso técnico** (token vencido ou revogado, senha trocada, rede fora) → fonte
      **desconectada** ("reconectar conta"); nada é apagado e o indexado continua
      pesquisável;
    - **acesso a um item revogado** para o usuário → o item sai do índice e aparece no
      relatório da fonte com o motivo.
19. **Mapas de extração** (para origens sem API estável): arquivo declarativo de seletores,
    distribuído pelo catálogo verificado como os pacotes de domínio, **assinado pela Levix e
    revisado por uma pessoa** antes de publicar — nunca gerado e publicado automaticamente.
20. **Google Drive completo, só pela API oficial**, com o escopo de leitura de todo o Drive
    (sem atalhos como restringir ao seletor de arquivos). Se a avaliação de segurança do
    Google (CASA) não couber cedo no ambiente pessoal, o conector entra quando couber; no
    ambiente corporativo usa o app interno da empresa, que não passa pela avaliação.

## Modelo de produto: tudo é conector

21. **A base é a busca nos arquivos locais e da rede interna**, sem nenhuma chamada externa.
    Todo o resto é **conector plugado explicitamente** pelo usuário — o "air-gapped por
    padrão" passa a ser literal: cada conector é uma abertura para fora, visível e
    governável. A ajuda do sistema não é conector.
22. **Dois tipos internos, um só nome para o usuário:**
    - **conector de origem:** traz conteúdo de fora pelo contrato da decisão 1 (sites web,
      Google Drive, OneDrive e SharePoint, YouTube, e-mail, Obsidian…);
    - **conector de capacidade:** libera um processamento sobre arquivos locais (ex.:
      transcrição de vídeos e áudios locais).

    Sem o conector, o que ele cobriria aparece no relatório da fonte ("12 vídeos não
    indexados — ative o conector de Vídeo"); nada some em silêncio.
23. **Conector pago entrega mais que a base.** Ex.: o cofre do Obsidian é uma pasta de
    Markdown e a base indexa o texto dele; o conector Obsidian acrescenta os wikilinks no
    grafo, o frontmatter como metadados, notas embutidas, aliases e canvas (a decisão 17
    descreve o que esse conector faz).
24. **A busca web na hora da pergunta também é conector**, por ser a principal saída de
    dados para fora.
25. **Licença por conector**, com **período de teste** por conector, na licença assinada e
    verificada offline (`levix-platform`). Licença ≠ política: a licença diz o que foi
    pago, a política diz o que a empresa permite; o mesmo motor aplica as duas e a mensagem
    de bloqueio diz qual delas bloqueou.
26. **Licença vencida: fontes desativadas, nunca apagadas.** Os índices das fontes daquele
    conector ficam guardados mas fora da busca, e o sync para. Ao renovar, a fonte volta e o
    diff incremental reprocessa só o que mudou na origem. O usuário pode apagar
    explicitamente os índices desativados para liberar disco.
27. **No ambiente corporativo**, a licença da organização libera os conectores, e a
    política do Admin Hub ainda pode bloquear cada um.

28. **YouTube só pela API oficial, confirmado.** Transcrição de vídeo de terceiro vem do
    conector de Vídeo (capacidade) sobre um arquivo que o usuário tenha de forma legítima,
    transcrito localmente — nunca de cliente não oficial nem de serviço de transcrição de
    terceiros.
29. **Arquivos "só online" em pastas locais** (OneDrive, Google Drive, Dropbox — Cloud Files
    no Windows, File Provider no macOS): o scanner local os detecta e **não força o
    download**; o relatório da fonte avisa ("120 arquivos estão só na nuvem — use o conector
    ou marque 'manter neste dispositivo'"). Os já baixados no disco são indexados
    normalmente. Hoje a varredura baixa tudo sem o usuário pedir: **bug atual**.

## Para o roadmap

- **Embalagem comercial** (quais conectores são grátis, quais pagos, preços, prazo do
  teste).
- Quais conectores existem e em que ordem: decidido no backlog e no roadmap.
