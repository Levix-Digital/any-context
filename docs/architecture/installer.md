---
title: Installer and Updater
summary: Installs, updates and uninstalls actx from the releases repository, with atomic binary swap and model provisioning.
code: [crates/actx-installer, install.ps1, install.sh, scripts/install.ps1, scripts/install.sh, scripts/uninstall.ps1, scripts/uninstall.sh]
---

# Installer and Updater

Installs, updates and uninstalls actx from the releases repository, with atomic binary swap and model provisioning.

## Responsibilities

- Install `actx` into the canonical bin directory from a GitHub release of `Levix-Digital/any-context-releases`, add it to the user `PATH` and record the chosen hardware profile.
- Update an existing installation (latest or a specific version) without breaking the running executable: download, extract to staging, validate, swap atomically, and roll back on failure.
- Repair shadowed copies of `actx` elsewhere in `PATH` and clean leftovers from older layouts.
- Provision the local models the profile needs (today the cross-encoder re-ranker), never failing the installation because of them.
- Log every update step to `update.log`.

## Structure

| Item | File | Role |
|---|---|---|
| `actx-installer` binary | `src/bin/actx-installer.rs` | Runs `run_installer_workflow`, or the launcher with `--launcher`. |
| `actx-shim` binary | `src/bin/actx-shim.rs` | Runs `run_launcher_or_update_workflow` (launcher mode). |
| Workflows | `src/lib.rs` | `run_installer_workflow`, `run_launcher_or_update_workflow`, `execute_standalone_update`, `run_standalone_update`, `replace_target_binary`, `heal_executing_and_shadowed_binaries`, `configure_system_path`, install profile helpers. |
| Paths and logging | `src/paths.rs` | Canonical bin and logs directories, executable and asset names, `log_update_event`. |
| Downloads | `src/downloader.rs` | `fetch_latest_release_tag`, `download_release_asset` (with progress, and the `ASSET_NOT_YET_AVAILABLE` sentinel on HTTP 404). |
| Extraction | `src/extractor.rs` | Unpacks `.zip` and `.tar.gz` packages. |
| Validation | `src/validator.rs` | `validate_binary_format`: accepts real PE/ELF executables, rejects archives and corrupt files. |
| Swap | `src/atomic_swap.rs` | `finalize_staging_update`, `retry_rename`, `retry_remove_file`. |
| Hardware | `src/hardware.rs` | `InstallerProfileTier`, pre-flight probing and summary, the models each profile provisions. |
| Provisioning | `src/provision.rs` | `provision_models`, `print_report`, `provisioning_disabled`. |
| Scripts | `install.ps1`, `install.sh` (identical copies in `scripts/`), `scripts/uninstall.ps1`, `scripts/uninstall.sh` | Bootstrap installers and uninstallers. |

## Behavior

### Locations and assets

- Bin directory: `%LOCALAPPDATA%\actx\bin` on Windows, `~/.local/bin` on Linux, the platform data directory on macOS; overridable with `ACTX_BIN_DIR` or `ACTX_UPDATE_DIR`.
- Logs: `%LOCALAPPDATA%\AnyContext\logs`, `~/.local/share/any-context/logs` on Linux, `~/Library/Logs/AnyContext` on macOS.
- Release packages: `actx-windows-x86_64.zip` and `actx-linux-x86_64.tar.gz`, containing `actx`, a compatibility copy named `actx-core`, `actx-installer` and the ONNX Runtime library (see [CI and Release](ci-release.md)). The release also publishes the standalone binaries and the install and uninstall scripts. Releases are published only for Windows and Linux x86_64.

### Fresh installation

`actx-installer [--install] [--version=<tag>] [--dir=<path>] [--profile=<tier>]`:

1. Probe the hardware and evaluate the profiles; use `--profile` or the recommended profile. `--preflight-only` prints the semaphore summary and exits.
2. Resolve the version (`--version` or the latest release tag).
3. Download the package to a temporary file, extract it into `<bin>/actx_staging` and call `finalize_staging_update`.
4. Add the bin directory to the user `PATH` with the highest precedence (Windows user environment) and write `install_profile.txt`.
5. For any profile other than Lite, provision the re-ranker files into the models directory, unless `ACTX_SKIP_MODEL_DOWNLOAD=1`. Failures are reported and the installation still succeeds; retrieval then uses the RRF fallback.

`--update` and `--rollback` (restores `actx_old` if present) are also accepted.

The bootstrap scripts download `actx-installer` from the latest release and run it with `--install`; when it is not available they fall back to downloading the package archive directly. The PowerShell script can also fetch a portable Tesseract with English and Portuguese language data for OCR.

### Updates

`actx --update[@<version>]` (and `actx update`, `actx-installer --update`) runs `execute_standalone_update`:

1. Resolve the target tag; download the package. An HTTP 404 means the release packages are still being built, and the user gets a friendly message with the release link instead of a raw error.
2. Extract into `actx_staging` and call `finalize_staging_update`, which validates the staged binary format and then moves the old files aside and the new ones into place with retried renames (20 attempts, 50 ms apart), rolling everything back if any step fails. The running Windows executable is renamed rather than overwritten, which Windows allows.
3. Ensure `PATH`, then `heal_executing_and_shadowed_binaries`: every other `actx` found in `PATH` (for example `~/.cargo/bin/actx.exe`) is replaced too, with a 15-attempt copy loop to survive antivirus or indexer locks.
4. Provision models for the profile chosen by the user: the `active_profile` setting (`/profile`), then `install_profile.txt`, and only then the hardware recommendation. Lite never downloads models. Provisioning is quiet and logged.

`actx --check-update` compares the latest tag with the running version as semantic version tuples. Inside the TUI, `/update` only explains how to run the update from the terminal. The bootstrap scripts, the installer and `--check-update` use `/releases/latest`, which never returns a release candidate (pre-release); a candidate is installed only by naming its tag (`actx-installer --version=<tag>`, `actx --update@<tag>`).

### Launcher mode

In launcher mode (`actx-installer --launcher`, or the `actx-shim` binary, which is built but not included in the release packages) the binary answers `-v` from `version.txt`, finalizes a pending staged update (`actx_staging/pending_update.json`, or exit code 42 from the child), removes leftover `.old` files, validates the core executable and runs it as a child process, forwarding arguments and setting `ACTX_LAUNCHER_PID`.

### Model provisioning

`provision_models` downloads each missing file of the profile's model set with size and SHA-256 verification and an atomic rename, removes partial files on any failure and keeps files that are already intact. The file list and checksums match the core catalog (a parity test keeps them in sync, see [Model Catalog and Hardware Profiling](model-catalog.md)).

## Interfaces

- **Used by**: the [CLI](cli.md) (`--update`, `--check-update`, `actx update`), the [command engine](commands.md) (paths for spawning workers, update logging), and users running the bootstrap scripts.
- **Calls**: GitHub Releases over HTTPS (`ureq`), the local filesystem.
- **Environment**: `ACTX_BIN_DIR`, `ACTX_UPDATE_DIR`, `ACTX_SKIP_MODEL_DOWNLOAD`.

## Decisions

- [ADR-0083](../adr/0083-dedicated-releases-repository-semver-updater.md): updates come from the dedicated releases repository with semantic version comparison.
- [ADR-0091](../adr/0091-hermetic-workspace-isolation-atomic-exe-swap.md): atomic `.exe.old` rename for Windows self-updates; no developer virtual environment assumptions.
- [ADR-0101](../adr/0101-shadowed-binary-sync-user-path.md): shadowed binaries updated, user `PATH` prioritized, clean messages.
- [ADR-0103](../adr/0103-windows-multi-binary-self-update.md): 15-attempt copy loop and `update.log`.
- [ADR-0107](../adr/0107-orphan-cleanup-legacy-updater-compatibility.md): compatibility with legacy updaters and cleanup of `actx-core.exe` and `_internal`.
- [ADR-0117](../adr/0117-aurora-palette-borderless-canvas-onboarding.md): friendly handling of HTTP 404 during updates.
- [ADR-0121](../adr/0121-modular-onnx-crate-hardware-profiles.md): `--profile` and `--preflight-only`.
- [ADR-0130](../adr/0130-real-onnx-runtime-multilingual-cross-encoder.md): ONNX Runtime shipped in the package, automatic re-ranker provisioning, updates honoring the chosen profile.
