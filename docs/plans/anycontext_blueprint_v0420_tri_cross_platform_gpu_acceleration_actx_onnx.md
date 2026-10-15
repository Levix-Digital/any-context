# 📐 Blueprint AnyContext v0.42.0: Matriz Tri-Cross-Platform de Aceleração por GPU & Hardware (DirectML, Metal/CoreML, CUDA/Vulkan) no `actx-onnx`

> **Status**: Proposto / Em Validação de Gate (Gate 3)  
> **Versão Alvo**: `v0.42.0`  
> **Data**: 2026-10-15  
> **Autor**: Antigravity (Advanced Agentic Pair-Programming)  
> **Origem do Produto**: Calgary, Alberta, Canada (100% Global, Agnóstico de Regiões)  

---

## 🎯 1. Objetivo & Decisões Arquiteturais (ADR-127)

### Contexto
Na versão `v0.41.0`, o AnyContext conta com o downloader assistido de pesos da família Gemma 4 e catálogo verificado de modelos de percepção. Contudo, toda a computação neural opera prioritariamente sobre a CPU com vetorização SIMD.
Em cenários de produção, modelos de linguagem locais (Gemma 4 Compact ~1.85GB, 12B ~6.75GB, 27B ~14.8GB e 32B ~20.2GB) demandam aceleração por hardware para atingir fluidez conversacional humana (15 a 50+ tokens/segundo).
Como o AnyContext é uma plataforma estritamente global, a aceleração por hardware deve contemplar com igualdade de excelência os 3 ecossistemas operacionais dominantes:

1. **Windows**: **DirectML (DirectX 12 Machine Learning)** — Solução nativa e agnóstica de hardware (NVIDIA GeForce/RTX, AMD Radeon, Intel Arc/Iris Xe, Qualcomm Snapdragon X Elite NPUs).
2. **macOS**: **Metal & CoreML** — Aproveita a arquitetura de Memória Unificada (UMA) dos chips Apple Silicon (M1 a M4) para inferência com zero cópia PCIe e velocidade estonteante (25 a 50 tok/s).
3. **Linux**: **CUDA / TensorRT (NVIDIA)** e **Vulkan Compute** — Padrão incontestável em workstations de desenvolvimento e servidores em nuvem/VPC.

### Decisão Arquitetural
1. **Core Domain (`crates/any-context-core-rs/src/hardware/mod.rs`)**:
   - Criar `HardwareProfiler::probe_accelerators() -> Vec<GpuDevice>` com verificação defensiva em tempo de execução (<5ms):
     - No Windows: Enumeração DXGI 1.2 via FFI direto com zero dependências externas pesadas.
     - No macOS: Detecção de Apple Silicon (M1–M4) e GPUs Metal via chamadas `sysctl` e IOKit.
     - No Linux: Inspeção de drivers NVIDIA (`/dev/nvidia*`, `libcuda.so`) e dispositivos de renderização DRM (`/sys/class/drm`).
   - Integrar `SystemMetrics` com o acelerador recomendado para a máquina ativa.
2. **Engine Neural (`crates/actx-onnx`)**:
   - Introduzir o enum `ExecutionDevice`:
     - `Auto`: Seleciona automaticamente o melhor acelerador disponível no sistema operacional host.
     - `DirectML { adapter_index: usize }`: Força DirectX 12 no Windows.
     - `MetalCoreML`: Força Metal/CoreML no macOS.
     - `Cuda { device_id: usize }`: Força NVIDIA CUDA no Linux ou Windows.
     - `Vulkan { device_id: usize }`: Força Vulkan Compute multiplataforma.
     - `Cpu`: Força execução 100% em CPU com multi-threading SIMD.
   - Garantia Inviolável ADR-110 (*Zero-Breakage Guarantee*): Se o acelerador selecionado falhar ao alocar tensores (ex: VRAM insuficiente para modelos 27B/32B), o motor reverte imediatamente para CPU sem crash ou interrupção.
3. **Provedor de LLM (`crates/actx-lm/src/providers/gemma.rs`)**:
   - `GemmaOnnxProvider` propaga telemetria do acelerador ativo (`tok/s`, VRAM alocada) para o streaming de tokens e accordion de raciocínio.
4. **Paridade de Superfícies (CLI Headless, Chat TUI e Menu `[F1]`)**:
   - Nova flag CLI: `actx --device <auto|dml|metal|cuda|cpu>`.
   - Novo slash command: `/device` (aliases `/gpu`, `/accelerator`) para listar adaptadores e alternar em tempo real.
   - Menu TUI `[F1]`: Submenu `Hardware Acceleration & GPUs`.
   - Cabeçalho da TUI: Exibe badge ativo do acelerador (`[🚀 DirectML: RTX 4070]`, `[🚀 Metal: M3 Pro]`, `[🚀 CUDA: RTX 3090]`, ou `[💻 CPU: 8 Cores]`).

---

## 🏛️ 2. Mapeamento de Paridade de Superfícies

| Superfície | Capacidade na v0.42.0 |
| :--- | :--- |
| **CLI Headless (`actx`)** | Flag `--device <auto\|dml\|metal\|cuda\|cpu>` e subcomando `actx --diag` com relatório de GPUs e VRAM. |
| **Chat TUI (`actx`)** | Comando `/device` interativo, telemetria de tokens por segundo no rodapé/accordion e badge de acelerador no cabeçalho. |
| **Menu TUI `[F1]`** | Submenu dedicado `Hardware Acceleration` permitindo alternar entre GPU e CPU com feedback imediato. |
| **Hardware Guardrails** | Proteção contra estouro de VRAM dedicada em GPUs de 4GB/6GB/8GB, impedindo OOM com reversão para CPU. |

---

## 📁 3. Arquivos e Linhas Impactadas

1. **`crates/actx-onnx/src/lib.rs` & `crates/actx-onnx/src/device.rs`**:
   - Adicionar módulo `device.rs` com enum `ExecutionDevice` e struct `GpuDevice`.
2. **`crates/any-context-core-rs/src/hardware/mod.rs`**:
   - Implementar `HardwareProfiler::probe_accelerators()`.
   - Expandir `SystemMetrics` com campo `accelerators: Vec<GpuDevice>` e `recommended_device: ExecutionDevice`.
3. **`crates/actx-onnx/src/session.rs`, `engine.rs` e `gemma.rs`**:
   - Atualizar construtores de `OnnxEngine` e `OnnxGemmaEngine` para receber `ExecutionDevice`.
   - Configurar pipeline com fallback transparente para CPU.
4. **`crates/actx-lm/src/providers/gemma.rs`**:
   - Injetar telemetria de tokens/s e dispositivo ativo na geração.
5. **`crates/any-context-core-rs/src/commands/engine.rs` & `models.rs`**:
   - Adicionar rota `/device` e ação `CommandAction::SetDevice(ExecutionDevice)`.
6. **`crates/actx-cli/src/cli/args.rs` & `tui/app.rs`**:
   - Suporte à flag `--device` e renderização de badge de acelerador no cabeçalho TUI.
7. **Bump de Versão para `v0.42.0`**:
   - `version.txt`, `Cargo.toml` em todos os crates do monorepo.

---

## 📚 4. Plano de Atualização Dual-Doc (`UserDoc` + `TecDoc`)

- **`UserDoc` (`README.md`)**:
  - Nova subseção: **"🚀 Aceleração Nativa por GPU (DirectML, Metal/CoreML e CUDA)"**.
  - Documentação dos comandos `/device` e flag `--device`.
- **`TecDoc` (`TECDOC.md`)**:
  - Registro formal do **ADR-127: Matriz Tri-Cross-Platform de Aceleração por Hardware no actx-onnx com Fallback Gracioso**.
  - Diagrama de sequência de handshake e alocação de buffers tensores.

---

## 🧪 5. Estratégia de Testes Automatizados & NFRs

- Testes unitários para enumeração de aceleradores em `crates/any-context-core-rs/tests/hardware_tests.rs`.
- Teste de fallback Gracioso: simulação de falha de alocação de GPU garantindo execução 100% funcional na CPU sem panic.
- Teste de parsing e dispatch de comandos no `crates/actx-cli/tests/cli_tests.rs`.
- Meta NFR: tempo de probe de aceleradores $< 5\text{ms}$ sem impacto no boot instantâneo do AnyContext.

---

## 📝 6. Plano de Cenários de Teste Manual (`tests/MANUAL_TESTS.md`)

- Prepend do **Cenário 42** no topo de `tests/MANUAL_TESTS.md`:
  - Passo a passo para validação de `/device` em Windows (DirectML), macOS (Metal) e Linux (CUDA/CPU).
  - Verificação de fallback gracioso forçado (`actx --device cpu`).
  - Inspeção dos badges de acelerador no cabeçalho e telemetria de tokens/segundo.

---

## 📐 7. Cronograma dos 6 Marcos de Execução

- **Marco 1**: Profilagem Tri-Cross-Platform no Core (`HardwareProfiler::probe_accelerators`).
- **Marco 2**: Abstração de Execution Providers no `actx-onnx` (`ExecutionDevice`).
- **Marco 3**: Fiação no `actx-lm` (`GemmaOnnxProvider`) e Telemetria de Tokens/s.
- **Marco 4**: CLI Flag `--device`, Slash Command `/device` e Menu TUI.
- **Marco 5**: Suíte de Testes Automatizados (100% verde no workspace).
- **Marco 6**: Dual-Doc (`README.md`, `TECDOC.md` ADR-127), Cenário 42 e Release `v0.42.0`.
