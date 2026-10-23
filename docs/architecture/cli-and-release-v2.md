# CLI and release — target design (v2)

Decisions for the command line, the build, the installers and the release, made on
2026-10-23 during the backlog rebuild (batch 8). The current behavior is described in
`cli.md`, `tui.md`, `ci-release.md` and `installer.md`.

## 1. Command line and TUI

1. **Headless CLI stays**, for automation: ask, sync, status, diagnostics and `actx sudo`,
   over the same core ports as the desktop app, with sign-in once per terminal session
   (`identity-and-governance-v2.md`, topic 1).
2. **Plan limits are enforced in the core, never in an interface.** The license and policy
   checks (`identity-and-governance-v2.md`, topic 4; `connectors-v2.md`, decision 28) run in
   the governance engine below every surface — desktop app, CLI, TUI and MCP server — so
   wrapping the CLI in a script, an API or an MCP server cannot unlock what the plan does
   not include.
3. **Changing the hardware profile** is an administrative action: `actx sudo profile`
   installs or switches the profile (it downloads models).
4. **The TUI is retired once the desktop app reaches parity** (decided 2026-10-23). Until
   then it keeps working over the current core but receives no v2 features; the CLI
   stays as the command-line surface. Keeping it was weighed against maintaining a second
   full interface for every v2 feature (accounts, Fast and Deep, queue, branches, source
   report, citations at the locator); the plan-bypass concern does not apply because of
   rule 2.

## 2. Build profile

- `lto = "fat"` and `codegen-units = 1`: quality over build time.
- Published binaries stay stripped; split debug symbols are kept privately by CI to read
  crash reports.
- `panic = "unwind"` stays: `retrieval/bm25.rs:336` and `actx-onnx/src/runtime.rs:80` rely
  on `catch_unwind`.

## 3. Installers and updates

- **Installers per platform** (Windows, Linux, macOS) carry ONNX Runtime, the embedded
  Chromium (`ingestion-pipeline-v2.md`, step 2) and FFmpeg as a separate library (LGPL;
  step 5), updated with every release.
- **macOS** is built and notarized like the other platforms.
- **Code signing** of the Windows and macOS binaries; buying the certificates is the
  owner's decision, and the item depends on it.
- **Everything downloaded is verified by the Levix signature:** application updates,
  models and domain packs (today the installer only checks that the binary is a valid
  PE/ELF file; models carry a SHA-256 in the catalog).
- **The updater sits behind a port:** the core stops calling `actx_installer` directly.

## 4. Diagnostics, feedback and crashes

- A **local diagnostic bundle** (logs, versions, configuration, sensitive data masked) the
  user reviews before anything leaves the machine.
- **`/feedback`** shows exactly what would be sent and sends only after confirmation.
- **A crash writes a local report** and offers to send it, also only with confirmation.
- **Nothing is ever sent automatically**, not even in beta or release-candidate builds; the
  corporate policy can turn sending off.

## 5. Housekeeping

- Documents and files with no relation to the current project are removed, not kept as
  backlog items (decided 2026-10-23: `.clinerules`, the subscription tiers plan and the
  CLI/TUI blueprint were removed).
- The ADRs are already in English; the ones the v2 design replaces are marked superseded.
- The repository gets its license file once the owner chooses the license.
