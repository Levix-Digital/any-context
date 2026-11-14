---
name: environment
description: Machines, cloud and CI: versions, blocked hosts, setup quirks. Read when a command, install or network call fails or behaves differently between machines.
updated: 2026-11-14
---

- 2026-10-30: Building the core crate and `actx-bench` needs `protoc` (`apt-get update && apt-get install -y protobuf-compiler`) for lance-encoding.
- 2026-10-30: Tests that need ONNX Runtime locally: `pip install onnxruntime==1.31.0` and point `ORT_DYLIB_PATH` at its `libonnxruntime.so.1.31.0`; `ACTX_REQUIRE_ORT=1` makes them fail instead of skip when it is missing.
- 2026-10-31: Cloud session disks fill with debug test builds: use `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`, or test crate by crate.
- 2026-10-29: huggingface.co and its CDN are reachable from cloud sessions, so models download there.
- 2026-10-30: The protocol's diagnosis can take more than two minutes; run it with a long timeout.
- 2026-10-31: The Ubuntu CI runner can run out of disk; rerun the failed job once the workflow finishes.
- 2026-10-25: Deleting remote branches from cloud sessions fails through the proxy; ask the owner to delete them.
- 2026-10-25: Cloud sessions that start outside the repository do not read `.claude/settings.json`; the cloud environment's setup script installs the protocol plugin and turns attribution off.
