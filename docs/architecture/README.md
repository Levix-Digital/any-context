# Arquitetura v2 — documentos e método

Revisão geral do AnyContext (outubro de 2026): código, documentação e backlog.

## Documentos

| documento | conteúdo |
|---|---|
| `ingestion-pipeline-v2.md` | pipeline de ingestão — design alvo |
| `ingestion-audit-2026-10.md` | auditoria da ingestão ("Any Context"), propostas e decisões |
| `retrieval-pipeline-v2.md` | pipeline de recuperação — estado atual e design alvo |
| `deployment-architecture-v2.md` | onde cada peça roda: portas, níveis de confiança, modelo de workspace, Admin Hub, modos de implantação |
| `identity-and-governance-v2.md` | usuários, chaves, papéis e políticas, com e sem Admin Hub |
| `platform-shared-components.md` | componentes compartilhados entre produtos Levix (Admin Hub, identidade, licença, LSIE) |
| `desktop-ui-v2.md` | referência de layout da interface desktop (Tauri 2) |

## Método de planejamento

1. **Backlog completo** do AnyContext no seu ideal máximo, sem cortar por prioridade.
2. **Roadmap de lançamentos:** um primeiro lançamento para entrar no mercado o mais rápido
   possível e os seguintes como evolução natural do produto.
3. **Priorização:** os itens do backlog são encaixados nos lançamentos do roadmap.

Cuidados:

- **Fundações entram cedo**, mesmo invisíveis (porta de armazenamento com consistência,
  porta de modelos por papel, separação do `settings.db`, hierarquia pai–filhos): o primeiro
  lançamento tem pouca funcionalidade sobre uma base já certa.
- **Cada item do backlog aponta para a seção do design** de onde veio.
