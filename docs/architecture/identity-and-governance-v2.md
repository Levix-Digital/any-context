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

Hoje não existe usuário: tudo fica no `settings.db` da conta do sistema operacional; há só
vestígios (coluna `added_by_email` com padrão `"user"`, injeção de identidade na sessão —
ADR-0082, FEAT-013 no backlog).

**Conta de administração no estilo Linux**

- **`root`** é a conta fixa de administração da instalação, com rótulo claro na interface
  ("Administrador (root)"). **Sem senha padrão:** a senha é definida obrigatoriamente na
  instalação ou na primeira execução. Recebe chave de recuperação própria.
- **`sudo`:** no dia a dia usa-se a conta pessoal; ações administrativas (criar ou remover
  contas, baixar modelos, configurações da instalação) pedem a senha do `root` só para
  aquela ação (`actx sudo <comando>` na CLI; "Ação administrativa: digite a senha do
  administrador" na interface).
- **O `root` não tem workspaces nem chat** e **não lê os dados das contas pessoais** — cada
  conta tem as próprias chaves.

**Contas pessoais**

- **Primeira execução:** uma tela define a senha do `root` e cria a conta pessoal (nome de
  exibição, usuário ou e-mail opcional, senha), já recomendada e preenchida; permite a mesma
  senha nas duas, com aviso de que senhas diferentes protegem melhor. Mostra a **chave de
  recuperação** e pede confirmação de que foi guardada; oferece biometria se houver.
- **Dados separados por conta:** diretório próprio com a base local (preferências,
  histórico, cofre) e os workspaces. Remover uma conta apaga os dados dela.
- **Tela de entrada** com as contas da máquina, senha ou biometria, troca rápida de usuário.
- **Sessão:** desbloqueada enquanto o app está em uso; trava após inatividade (ex.: 30 min,
  preferência pessoal), ao bloquear a tela ou suspender o sistema, e manualmente. CLI e TUI
  pedem a senha (ou biometria) uma vez por sessão do terminal, com o mesmo bloqueio.
- **Espera crescente** entre tentativas após senhas erradas.
- **Troca de senha** só reembrulha a chave dos dados — instantânea, sem recriptografar.

**Sync com o app fechado — caixa de entrada selada** (sem chave vinculada ao dispositivo,
recusada por ser arriscada)

- O serviço em segundo plano recebe só a **chave pública** do usuário: consegue criptografar,
  nunca descriptografar, e não lê nada do que já está indexado.
- Com o app fechado ele faz o trabalho pesado — detectar arquivos alterados, baixar páginas,
  extrair, OCR, transcrição, chunking e embedding **com modelos locais** — e grava o
  resultado numa **caixa de entrada criptografada com a chave pública**.
- Ao entrar, o app desbloqueado incorpora a caixa de entrada ao índice (rápido: só gravar).
- **Sem risco novo:** o serviço só usa o que já está em claro no disco (os arquivos do
  usuário). Limitação: chaves de API de nuvem ficam no cofre trancado, então com embedding na
  nuvem o embedding fica para o login.
- Modos de sync: **padrão** com a sessão desbloqueada (e diff de recuperação ao entrar);
  **opcional** caixa de entrada selada com o app fechado; **modo 3** no AnyContext Server.

**No modo empresarial (com Admin Hub)**

- **O poder do `root` passa para o Admin Hub:** o `root` local é desativado para o uso
  comum; criar contas, classificar endpoints, instalar modelos fora do catálogo e mudar a
  instalação passam a ser governados pelo Admin Hub.
- **Origens de identidade:** **SSO** pelo provedor da empresa e/ou **diretório próprio do
  Admin Hub** (para clientes sem provedor de identidade — escritórios, clínicas, prefeituras
  menores). Contas só locais ficam proibidas pela política.
- **Ativação de usuário do diretório do Admin Hub:** o admin cadastra (nome, login, papéis);
  o Admin Hub gera um **código de ativação de uso único** com validade curta (ex.: 72 h),
  entregue por canal à parte; no primeiro uso num dispositivo matriculado, o usuário digita
  login e código, **cria a própria senha** e recebe a chave de recuperação. **A senha nunca
  passa pelo Admin Hub**; as chaves são geradas no dispositivo. A política pode exigir
  tamanho e complexidade, troca periódica e **segundo fator** (aplicativo autenticador).
- **Senha esquecida:** o admin emite um novo código de ativação; sem chave de recuperação, os
  dados antigos ficam ilegíveis (opção de chave de recuperação da organização: tema 2).
- **`sudo` vira "pedir ao TI":** autenticação de um admin corporativo, ou pedido de aprovação
  no Admin Hub (que pode virar chamado no ServiceNow/Jira).
- **Quebra-vidro:** o `root` continua existindo para emergência no local e offline, com
  **senha única por dispositivo, gerada e trocada pelo Admin Hub** (modelo LAPS), visível só
  a administradores autorizados com registro de acesso; todo uso é auditado, relatado no
  próximo heartbeat, e a senha é trocada depois.
- **Regra de ouro:** nem o `root`, nem o admin corporativo, nem o Admin Hub leem os dados dos
  usuários — governam a instalação, não o conteúdo.
- **Saída:** dispositivo revogado (kill-switch) é bloqueado; desmatriculado de forma
  ordenada (ex.: devolvido ao TI), a política decide se os dados são apagados ou mantidos.

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
