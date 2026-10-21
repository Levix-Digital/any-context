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
4. ~~Migração da conta local para a corporativa na matrícula~~ — **substituída** pelos
   ambientes pessoal e corporativo isolados (tema 6).

## Temas a detalhar

1. Contas locais e sessão
2. Hierarquia de chaves
3. Papéis e compartilhamento
4. Motor de governança e políticas
5. Integração com o Admin Hub
6. Ambientes pessoal e corporativo

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

Hoje há uma única chave, derivada do id da máquina com salt fixo no código, que protege só
os campos de texto do LanceDB; o índice lexical está em claro; e **as chaves de API ficam
em texto puro** na tabela `api_keys` do `settings.db`.

1. **Chave mestra do usuário** — aleatória, uma por conta, nunca gravada em claro. Embrulhada
   por: **senha** (Argon2id), **chave de recuperação** e **biometria** como conveniência (o
   cofre do sistema só a libera mediante presença do usuário a cada desbloqueio — nunca para
   um serviço sozinho).
2. **Par de chaves do usuário:** a pública sela a caixa de entrada do sync com o app
   fechado; a privada fica criptografada pela chave mestra.
3. **Chave de dados de cada workspace** — aleatória; criptografa tudo do workspace (camada
   de armazenamento do LanceDB e do índice lexical, base do workspace, resumos). Embrulhada
   pela chave mestra do dono e, no compartilhamento, pela **chave pública de cada membro**
   (contas da mesma máquina; no modo 3, o servidor a guarda no próprio cofre). **Saída de um
   membro troca a chave:** dados novos usam a nova; os antigos são recriptografados em
   segundo plano.
4. **Cofre pessoal**, criptografado a partir da chave mestra: chaves de API, senhas de
   arquivos, histórico de conversas, preferências — corrige as chaves de API em texto puro.
5. **Identidade do dispositivo** (chave Ed25519 da matrícula no Admin Hub) no cofre do
   sistema: identifica a máquina, não dá acesso a dados.
6. **Chaves só em memória** com a sessão desbloqueada; apagadas ao travar.
7. **Migração:** no primeiro login após a atualização, os dados criptografados com a chave
   antiga (derivada do id da máquina) são recriptografados com as novas, e as chaves de API
   vão para o cofre.
8. **Chave de recuperação da organização** (opção de política no modo empresarial): a chave
   mestra de cada usuário também é embrulhada por uma chave pública da organização; **a parte
   privada fica offline, sob controle da empresa, idealmente dividida entre duas ou mais
   pessoas** — não no Admin Hub, que segue zero-knowledge no dia a dia. Cada uso é auditado
   e o usuário é avisado de que a empresa tem esse poder de recuperação.

### 3. Papéis e compartilhamento

Já decidido (`deployment-architecture-v2.md`): papéis dono, editor e leitor; workspaces da
organização com o admin como dono e só leitura para os demais; chat individual; no modo 1
compartilha-se a definição do workspace, no modo 3 o índice.

1. **Permissões:**

   | ação | dono | editor | leitor |
   |---|---|---|---|
   | perguntar, abrir fontes citadas, 👍/👎 | ✓ | ✓ | ✓ |
   | adicionar e remover fontes, filtros, reprocessar | ✓ | ✓ | — |
   | editar prompt do workspace e pacote de domínio | ✓ | ✓ | — |
   | marcar confidencial, políticas de privacidade | ✓ | — | — |
   | compartilhar, mudar papéis | ✓ | — | — |
   | apagar o workspace | ✓ | — | — |

   Só o dono compartilha; editor não repassa acesso.
2. **Grupos e curadores:** no modo empresarial, compartilhamento com **grupos** (do SSO ou do
   diretório do Admin Hub); no workspace da organização, o admin nomeia **curadores**
   (editores de um departamento) sem perder a propriedade.
3. **Definição do workspace** (o que se compartilha no modo 1): nome, fontes, filtros,
   pacote de domínio e prompt — as instruções de montagem, **nunca o conteúdo indexado**
   (chunks, vetores, índice), que fica criptografado na máquina de quem indexou. Cada membro
   indexa as mesmas fontes na própria máquina (as fontes precisam ser acessíveis a todos,
   ex.: pasta de rede, site).
   - **Sem Admin Hub:** a definição é exportada num arquivo (ex.:
     `processos-2025.actx-workspace`) e importada pelo colega — sem documentos, trechos,
     vetores, chaves de API nem senhas.
   - **Com Admin Hub:** a definição é distribuída pelo Admin Hub (só metadados; ele segue
     zero-knowledge).
4. **Confidencial vale para todos os membros:** o teto "nunca enviar para endpoint externo"
   se aplica a quem quer que pergunte no workspace.
5. **Saída de um membro:** a chave do workspace é trocada (tema 2); as conversas dele
   continuam com ele (dados pessoais), mas as citações deixam de abrir; no modo empresarial,
   a política pode exigir apagá-las.
6. **Permissões herdadas da fonte** (modo 3 e conectores): num índice compartilhado, um
   leitor nunca recebe trechos de documentos que não pode abrir na origem (ex.: permissões do
   SharePoint ou do servidor de arquivos). Implementação detalhada na auditoria P23.

### 4. Motor de governança e políticas

Hoje o core não tem política nenhuma. A especificação do Admin Hub já define uma política
rica (modo de modelos e modelos permitidos, cota de tokens, OCR, busca web com escopo e
domínios, servidor MCP, conectores e domínios de conta), enviada por `UPDATE_POLICIES` e
guardada em cache criptografado com validade de 72 h — nada disso está ligado ao cliente. A
especificação embute a chave pública do Admin Hub no binário, o que não funciona com cada
cliente hospedando o próprio Admin Hub.

1. **Modelo único de política no cliente, versionado**, por categoria:
   - modelos e endpoints (níveis de confiança, modelos permitidos);
   - dados (pseudonimização, workspaces confidenciais, retenção, chave de recuperação da
     organização);
   - fontes (tipos, conectores e domínios web permitidos, limites);
   - ferramentas (busca web, servidor MCP);
   - contas (regras de senha, segundo fator, tempo de sessão, sync em segundo plano);
   - cotas (tokens).
2. **Três camadas de precedência:** padrão do produto < política local (definida pelo `root`
   sem Admin Hub) < política da organização (assinada pelo Admin Hub). **A camada de cima
   vence; dentro de um workspace, o dono só pode endurecer as regras, nunca afrouxá-las**
   (pode marcar confidencial; não pode liberar a busca web proibida pela empresa).
3. **Políticas assinadas, com a chave do Admin Hub fixada na matrícula:** o código de
   matrícula traz a impressão digital da chave, o dispositivo a confirma e registra, e
   depois só aceita políticas assinadas por ela (corrige a chave embutida no binário).
4. **Um só motor aplica tudo:** resolução de modelos, níveis de confiança e
   pseudonimização, busca web, conectores e OCR, servidor MCP, contas e sessão, cotas.
5. **Offline:** vale a última política válida em cache criptografado; a **validade** (72 h na
   especificação) é **configurável pela empresa**; vencida, o dispositivo é suspenso até
   reconectar.
6. **Transparência:** bloqueio diz qual regra bloqueou ("busca web desativada pela política
   da organização"); o usuário pode ver a política vigente, só leitura.
7. **Auditoria:** tentativas bloqueadas registradas localmente e enviadas ao Admin Hub só
   como metadados (regra, quando, dispositivo), nunca com conteúdo.

### 5. Integração com o Admin Hub

As especificações iniciais do Admin Hub (matrícula, WebSocket só de saída, heartbeat,
kill-switch, ITSM, SCIM, validade offline) se encaixam, com estes ajustes:

1. **Pacote instalável e licença offline.** Sem SaaS da Levix, o Admin Hub é entregue como
   pacote (contêineres para servidor único; Helm para Kubernetes). O controle de assentos
   usa uma **licença assinada pela Levix** (assentos, validade), verificada **offline** pelo
   Admin Hub.
2. **Admin Hub em Rust**, compartilhando crates com o core: modelo de políticas, verificação
   de assinaturas e formato da definição do workspace precisam ser idênticos nos dois lados
   (a especificação previa Python/FastAPI; o AnyContext eliminou o Python — ADR-0106).
3. **Telemetria de fontes com nível definido pela política:** só contagens; **identificadores
   anonimizados** (hash que diz "é a mesma fonte" sem revelar o nome) — **padrão**; ou
   caminhos completos, quando a empresa precisa auditar e informa os usuários. (A
   especificação enviava os caminhos completos, que podem ser dado pessoal, ex.:
   `...\Processos\Divórcio Fulano de Tal`.)
4. **Espelho interno:** o Admin Hub distribui atualizações do AnyContext, modelos ONNX,
   pacotes de domínio e o Chromium para dispositivos sem internet (air-gapped); o TI baixa
   uma vez, verificado pela assinatura da Levix, e controla quando cada versão chega à
   frota.
5. **Dois perfis de instalação, mesmo código:** **compacto** (um contêiner com banco
   embutido — escritórios, clínicas) e **escalável** (Postgres, Redis, réplicas — grandes
   empresas).
6. **O que passa pelo canal do Admin Hub:** identidades (SSO, diretório próprio, SCIM) e
   ativação; políticas assinadas; definições de workspace e workspaces da organização; grupos
   e curadores; senha de quebra-vidro (estilo LAPS); pedidos de `sudo` e chamados ITSM;
   auditoria só com metadados; cotas; espelho de atualizações; matrícula do AnyContext Server
   (modo 3).

Decisão comercial para o roadmap: assentos por dispositivo (como na especificação) ou por
usuário (comum no mercado); a arquitetura suporta as duas.

### 6. Ambientes pessoal e corporativo

Substitui a decisão de base 4 ("migração da conta local para a corporativa"): a matrícula
de uma empresa não pode afetar as contas da família numa máquina de trabalho remoto.

1. **Dois ambientes isolados no mesmo binário**, na linha do "perfil de trabalho" do
   Android, com seletor ("Pessoal | Empresa X"):
   - **pessoal:** `root` local e contas da família; a empresa nunca vê nem governa nada aqui
     (sem telemetria, política ou kill-switch);
   - **corporativo:** criado na matrícula, com **dados, chaves, contas, políticas e `root`
     próprios** (o quebra-vidro do Admin Hub); só a empresa o governa.

   A empresa pode distribuir o próprio pacote de instalação pelo TI; o resultado é o mesmo
   ambiente corporativo isolado.
2. **Sem caminho entre ambientes.** Não há exportação do pessoal para o corporativo: quem
   quiser as mesmas fontes as adiciona no corporativo, onde são indexadas sob a política (e
   conteúdo de trabalho que estava fora da governança deve mesmo ser recriado). Se surgir
   demanda real de usuários, alternativas são avaliadas depois. A exportação e importação da
   definição do workspace (tema 3) vale **dentro** de cada ambiente; a política corporativa
   pode aceitar só definições vindas da própria organização (assinadas pelo Admin Hub).
3. **Computador da empresa:** a política pode proibir o ambiente pessoal em máquinas que
   pertencem à empresa; no notebook do próprio funcionário, não.
4. **Saída e mudança de empresa** afetam só o ambiente corporativo (dados apagados ou
   bloqueados conforme a política); o pessoal segue como estava. Mudar de empresa remove o
   ambiente da anterior e cria o da nova na nova matrícula.
5. **Nenhum dado atravessa entre ambientes** — nem busca, nem chat, nem chave (a regra
   "nenhuma consulta atravessa workspaces", um nível acima).
