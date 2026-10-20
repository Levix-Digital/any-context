# Arquitetura de implantação — design alvo (v2)

Documento transversal: os pipelines de ingestão e de recuperação descrevem **o que**
acontece; este documento descreve **onde** cada peça roda. É decidido antes do backlog,
porque define as costuras que os dois pipelines precisam respeitar.

Motivação: no plano empresarial, o admin pode querer o armazenamento num servidor da
própria VPC, modelos pesados num servidor com GPU, e uma base de conhecimento indexada uma
vez e consultada por todos.

## Princípio

O core fala com **portas** (interfaces). Cada porta tem uma implementação local (padrão) e
uma remota. Decide-se agora as costuras; os modos remotos são construídos depois, como itens
do plano empresarial — sistema distribuído antes de ter clientes empresariais é
complexidade sem retorno.

## Costuras

1. Porta de armazenamento
2. Porta de modelos, por papel
3. Nível de confiança de cada endpoint
4. Identidade do modelo de embedding inclui onde ele roda
5. Worker de ingestão como papel separável

### 1. Porta de armazenamento

Hoje não há interface: o core usa direto o `NativeLanceStore` (9 arquivos) e três
armazenamentos sem coordenação — LanceDB (vetores e texto), índice lexical e `settings.db`
(que mistura preferências, chaves, histórico, fontes, cache do diff e resumos).

- **Porta única de "base de conhecimento" por workspace**, que esconde os três
  armazenamentos: gravar e remover um documento com toda a árvore (pais, filhos, vetores,
  termos, metadados); busca densa, lexical e por filtros; chunks por id e pai de um chunk;
  consulta a tabelas e formulários; estatísticas. **A porta é responsável pela
  consistência:** cada gravação passa por um diário (journal) e é aplicada de forma
  idempotente nos três lugares; uma falha no meio é completada no próximo sync. Resolve um
  problema que já existe hoje, mesmo sem servidor.
- **O `settings.db` é separado em dois:**
  - **local do usuário** (nunca sai da máquina): preferências, histórico de conversa,
    referências do cofre, perfil de hardware;
  - **do workspace** (acompanha a base de conhecimento): fontes, cache do diff, entidades,
    mapas de layout, resumos, famílias de versões.
- **Sem "LanceDB direto no S3/MinIO"**: o Tantivy precisa de disco de baixa latência, a
  consistência entre os três armazenamentos fica frágil pela rede e o cálculo continua no
  cliente. Armazenamento remoto só pelo AnyContext Server (modo 3), que roda os três
  armazenamentos com disco local e expõe a porta por API.
- **Segurança no servidor:** chave de criptografia no cofre ou serviço de chaves do servidor;
  comunicação com TLS e autenticação mútua.
- **Abrir o arquivo original** (navegar pastas, abrir arquivos, ver sites sem sair do
  AnyContext) fica para a interface em Tauri 2 e não é decidido aqui.

## Modelo de workspace

- **Cada usuário cria os seus workspaces e é o único dono.** Pode compartilhar com outros
  usuários, definindo papéis.
- **O admin pode criar workspaces da organização**, disponíveis para toda a empresa, que
  ninguém além dele edita.
- **Um chat por workspace, nunca misturado.**
- **Nenhuma consulta atravessa workspaces.** Separar contextos ao gosto do usuário é a
  função do workspace.

### 2. Porta de modelos, por papel

_Pendente._

### 3. Nível de confiança de cada endpoint

_Pendente._

### 4. Identidade do modelo de embedding

_Pendente._

### 5. Worker de ingestão

_Pendente._

## Modos de implantação

| modo | descrição | quando |
|---|---|---|
| 1. Desktop | tudo local; padrão | já |
| 2. Desktop + servidor de modelos | índice local, modelos pesados na VPC; só a porta de modelos fica remota | primeiro passo empresarial |
| 3. Cliente + AnyContext Server | índice, ingestão e modelos no servidor; a máquina é cliente; conhecimento compartilhado | plano empresarial completo, com o módulo de usuários e o controle de acesso por fonte (auditoria P23) |
