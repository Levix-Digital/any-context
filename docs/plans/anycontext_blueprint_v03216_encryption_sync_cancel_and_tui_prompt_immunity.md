# Blueprint v0.32.16: Transparent Vector Decryption, Sync Cancellation, Session Directory Navigation & TUI Prompt Immunity

## 1. Contexto & Causa Raiz
Na versão `v0.32.15`, após testes em múltiplos sistemas operacionais (Windows e Linux), três gargalos de engenharia e usabilidade foram identificados:
1. **Texto Criptografado no Linux com `/inspect --full` (`enc::...`)**:
   - O AnyContext implementa criptografia em repouso AES-GCM-256 com PBKDF2-HMAC-SHA256 atrelada à assinatura de hardware (`SecurityEngine` em Python).
   - Quando documentos eram sincronizados no Linux via worker Python, os campos `text`, `document_summary` e `keywords` eram persistidos no LanceDB como `enc::<base64_payload>`.
   - No Core Rust, `execute_inspect` e o leitor de vetores liam esses dados crus, exibindo strings cifradas indecifráveis para o usuário e ameaçando poluir o contexto fornecido ao LLM em queries RAG.
2. **Impossibilidade de Cancelar Sincronizações em Andamento (`/sync cancel`)**:
   - Durante a sincronização de diretórios extensos (ex: 268.865 arquivos), o worker em background monopolizava I/O e CPU sem opção na TUI ou CLI de interrupção imediata.
   - O comando `/folder .` dentro da TUI referia-se ao diretório de lançamento do processo sem possibilidade de navegação interna.
3. **Vazamento de Warning do BM25 no Widget de Prompt e Concorrência de I/O**:
   - Mensagens de advertência impressas com `eprintln!` em `pipeline.rs` vazavam diretamente para o `stderr`. No Ratatui (modo *Alternate Screen*), isso causava sobrescrita direta de caracteres na linha do cursor do widget `Prompt`.
   - Durante reindexações concorrentes, `BM25Index::load_from_file` tentava ler o arquivo enquanto o worker gravava ou antes do flush atômico, resultando em `failed to fill whole buffer`.

---

## 2. Decisões Arquiteturais & Solução (ADR-100)

### 2.1 NativeSecurityEngine no Core Rust (`crates/any-context-core-rs/src/security/mod.rs`)
- **Derivação de Hardware Idêntica ao Python**:
  - Linux: `/etc/machine-id` ou `/var/lib/dbus/machine-id` (`lin_mid_*`).
  - Windows: Registro `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` (`win_guid_*`).
  - macOS: `IOPlatformUUID` (`mac_uuid_*`).
  - Override para testes via variável de ambiente `ACTX_MACHINE_ID`.
- **KDF & Cifra**: PBKDF2-HMAC-SHA256 (100.000 rounds) com `DOMAIN_SALT` e `INTERNAL_PEPPER`, gerando chave de 256 bits para `Aes256Gcm`.
- **Descriptografia Transparente em Tempo de Execução**:
  - `NativeLanceStore::extract_scored_results` decifra automaticamente `text`, `document_summary` e `keywords` para todas as consultas vetoriais e metadados.
  - `/inspect` (com ou sem `--full`) decifra o conteúdo na amostragem e na taxonomia.

### 2.2 Cancelamento de Sincronização & Navegação de Diretórios
- **`/sync cancel` (e `/cancel`, `/sync --cancel`)**:
  - Consulta o PID do worker gravado no SQLite (`workspace_sync_status.pid`).
  - Encerra a árvore do processo (`taskkill /PID /F /T` no Windows e `kill -9` no Unix).
  - Atualiza o SQLite para `is_syncing = 0`, `stage = "cancelled"`, `progress_bar = "[cancelled]"`.
  - Adiciona o botão "Cancelar Sincronização em Andamento" ao menu TUI (`[F1]` ou `/sync`).
- **`/pwd` e `/cd <path>`**:
  - `/pwd`: Consulta e exibe o diretório de trabalho atual do processo.
  - `/cd <caminho>`: Atualiza o diretório de trabalho em tempo de execução (`std::env::set_current_dir`).
  - `/folder .`: Emite nota explícita `(resolved from current working directory)`.

### 2.3 Imunização do Prompt TUI e Resiliência Concorrente do BM25
- **Eliminação de `eprintln!`**: Avisos de corrupção ou quarentena são gravados em arquivo dedicado (`bm25_quarantine.log`) sem vazar bytes no buffer de tela do Ratatui.
- **Tolerância a 0 Bytes**: `BM25Index::load_from_file` detecta arquivos vazios (`meta.len() == 0`) e inicializa um índice novo sem disparar erro.
- **Retry com Backoff no Windows**: `save_to_file` e `load_from_file` utilizam laços de retry exponencial com sleep tolerando bloqueios momentâneos de leitura concorrente.

---

## 3. Marcos de Execução (Milestones)
- **M1: NativeSecurityEngine & LanceDB Decryption**: Implementação do módulo criptográfico em Rust com testes de interoperabilidade Python/Rust 100% verdes.
- **M2: Sync Cancellation & Comandos `/cd`/`/pwd`**: Implementação de `/sync cancel` (com `taskkill`/`kill`), `/cd`, `/pwd`, e opção no menu TUI.
- **M3: Blindagem do BM25 e Imunização da TUI**: Remoção de `eprintln!`, tolerância a 0 bytes e retry backoff concorrente.
- **M4: Suíte de Testes Automatizados**: 131 testes unitários do Core, 18 testes de comandos (`command_tests.rs`), 14 testes CLI (`cli_tests.rs`) e testes Python 100% verdes.
- **M5: Dual-Doc & Cenário 17 em MANUAL_TESTS.md**: Documentação completa em `README.md`, `TECDOC.md` (ADR-100) e Cenário 17 acumulativo não-truncado.
- **M6: Verificação Gate 8 & Release v0.32.16**: Validação pelo `verify_gate.py`, tag `v0.32.16`, push e CI watch dos 12 binários de release.
