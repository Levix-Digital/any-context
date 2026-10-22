# Componentes compartilhados da plataforma Levix

Documento transversal: componentes construídos uma vez e reaproveitados pelos produtos
Levix (AnyContext, DocMapper e futuros).

## Princípio de adoção

- **Desenhar os limites já:** o que é núcleo comum, o que é de cada produto, e os contratos
  entre eles.
- **Construir dentro das necessidades do AnyContext**, já como crates e serviço separados,
  sem dependência do AnyContext.
- **Os demais produtos adotam quando chegar a vez deles no roadmap.** Nada de plataforma
  genérica construída antes da hora.

## Componentes

### 1. Admin Hub (produto à parte)

Plano de controle de **vários produtos**: uma empresa que tem AnyContext e DocMapper instala
um Admin Hub só (sempre hospedado no cliente, zero-knowledge — ver
`identity-and-governance-v2.md`).

- **Núcleo comum:** organizações; usuários (diretório próprio, SSO, SCIM) e grupos;
  dispositivos e matrícula; **licença e assentos por produto**; distribuição e assinatura de
  políticas; auditoria; ITSM; quebra-vidro; espelho de atualizações.
- **Por produto:** cada produto registra o próprio **esquema de políticas** (o do AnyContext:
  modelos, níveis de confiança, busca web, MCP…); o núcleo distribui, assina e audita sem
  conhecer os detalhes.
- **Contrato:** API e WebSocket documentados — qualquer produto conversa com o Admin Hub pelo
  protocolo, independente da linguagem.

### 2. Identidade no cliente

Contas, `root`/`sudo`, sessão, hierarquia de chaves, cofre, biometria, matrícula e o motor
de governança (aplicação das políticas assinadas) — praticamente tudo de
`identity-and-governance-v2.md`, exceto workspaces, que são do AnyContext.

**As contas nunca são compartilhadas entre produtos.** Cada produto tem as próprias contas
na máquina; usuários de produtos diferentes **jamais se misturam**. O que é comum é a
**administração**: o Admin Hub gerencia as contas de todos os produtos com as mesmas
ferramentas de gestão de usuários.

### 3. Outros

- **Licenciamento** (o DocMapper já tem um módulo de licenças).
- **Catálogo verificado** de modelos e atualizações.
- **LSIE** (extração de formulários), já decidido como crate única
  (`ingestion-pipeline-v2.md`, passo 6).

## Linguagens

- **Admin Hub:** serviço em Rust (decidido em `identity-and-governance-v2.md`, tema 5).
- **Módulos de cliente** (identidade, políticas, LSIE…): **crates Rust, compiladas também
  para WASM**. No navegador, armazenamento e criptografia mudam (WebCrypto, IndexedDB no
  lugar do cofre do sistema) por **adaptadores**; um produto desktop em Tauri usa a crate
  direto.

## Repositórios

- `levix-admin-hub` — produto à parte.
- `levix-platform` — crates de identidade, políticas, criptografia, licença, LSIE.
- Os produtos dependem dessas crates por versão.

Efeito no backlog: itens desses componentes pertencem a `levix-admin-hub` e
`levix-platform`, não ao AnyContext.
