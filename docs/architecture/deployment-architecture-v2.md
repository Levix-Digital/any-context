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

_Em discussão._

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
