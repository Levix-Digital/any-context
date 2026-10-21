# Identidade e governança — design alvo (v2)

Documento transversal sobre usuários, chaves, papéis e políticas, com e sem Admin Hub. É
fundação: toca a separação do `settings.db`, a chave de criptografia, o dono e os papéis de
cada workspace, o chat individual e o cofre de senhas de arquivos.

## Decisões de base

1. **Contas locais com usuário e senha na versão sem Admin Hub**, com **vários usuários por
   máquina**. Cada pessoa tem os seus workspaces e conversas; ninguém vê os do outro. Serve
   ao computador da família e ao escritório pequeno sem Admin Hub.
2. **A senha libera a chave dos dados.** Hoje a chave fica no cofre do sistema operacional e
   qualquer um logado naquela conta do sistema consegue usá-la; com a senha do AnyContext,
   notebook roubado ou outra pessoa na mesma conta do sistema não lê os dados.
   - **Chave de recuperação** gerada na criação da conta, para o usuário guardar (imprimir
     ou salvar) — senha esquecida sem ela significa dados perdidos.
   - **Desbloqueio por biometria** (Windows Hello, Touch ID) como conveniência, sem
     enfraquecer a senha.
   - A senha é guardada só como hash forte (Argon2id); nunca é enviada a lugar nenhum.
3. **Um motor único de identidade e governança no cliente, com duas origens** — não duas
   implementações (que tendem a divergir):

   | | sem Admin Hub | com Admin Hub |
   |---|---|---|
   | usuários | contas locais (usuário e senha) | identidade corporativa (SSO da empresa via Admin Hub) |
   | políticas | padrões locais, ajustáveis pelo dono | políticas assinadas pelo Admin Hub, empurradas pela API do cliente |
   | quem aplica | o mesmo motor no cliente | o mesmo motor no cliente |

   Usuário, dono, papéis, políticas, níveis de confiança e cofre existem uma vez; o Admin
   Hub é uma origem de identidades e políticas.
4. **Migração da conta local para a corporativa na matrícula:** ao matricular o dispositivo,
   a conta local é vinculada à identidade corporativa e os workspaces continuam, sujeitos à
   política.

## Temas a detalhar

1. Contas locais e sessão
2. Hierarquia de chaves
3. Papéis e compartilhamento
4. Motor de governança e políticas
5. Integração com o Admin Hub
6. Migração local → corporativa

### 1. Contas locais e sessão

_Em discussão._

### 2. Hierarquia de chaves

_Pendente._

### 3. Papéis e compartilhamento

_Pendente._

### 4. Motor de governança e políticas

_Pendente._

### 5. Integração com o Admin Hub

_Pendente._

### 6. Migração local → corporativa

_Pendente._
