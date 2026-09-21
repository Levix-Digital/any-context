import os
import time
import tempfile
import pytest

import any_context_core_rs
from any_context.ingestion.local_folder_ingestor import discover_workspace_files
from any_context.ingestion.orchestrator import check_workspace_changes
from any_context.config.db_store import ConfigDBStore


class TestWorkspaceScannerRust:
    """
    Validates native Rust WorkspaceScanner performance, parity, and differential
    change detection (new, modified, deleted, and zero-cost renamed files).
    """

    def test_scanner_instantiation_and_version(self):
        scanner = any_context_core_rs.WorkspaceScanner()
        assert scanner is not None
        assert hasattr(scanner, "discover_files")
        assert hasattr(scanner, "scan_and_diff")

    def test_scanner_discover_files_parity(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # 1. Valid files
            f_py = os.path.join(tmpdir, "main.py")
            with open(f_py, "w", encoding="utf-8") as f:
                f.write("print('hello')\n")

            f_tf = os.path.join(tmpdir, "infra.tf")
            with open(f_tf, "w", encoding="utf-8") as f:
                f.write('resource "null_resource" "x" {}\n')

            f_docker = os.path.join(tmpdir, "Dockerfile")
            with open(f_docker, "w", encoding="utf-8") as f:
                f.write("FROM alpine:3.18\n")

            f_env = os.path.join(tmpdir, ".env.production")
            with open(f_env, "w", encoding="utf-8") as f:
                f.write("KEY=VALUE\n")

            # 2. Ignored files & directories
            f_min = os.path.join(tmpdir, "bundle.min.js")
            with open(f_min, "w", encoding="utf-8") as f:
                f.write("var a=1;")

            f_lock = os.path.join(tmpdir, "~$doc.docx")
            with open(f_lock, "w", encoding="utf-8") as f:
                f.write("lock")

            git_dir = os.path.join(tmpdir, ".git")
            os.makedirs(git_dir, exist_ok=True)
            f_git = os.path.join(git_dir, "config")
            with open(f_git, "w", encoding="utf-8") as f:
                f.write("[core]\n")

            node_dir = os.path.join(tmpdir, "node_modules")
            os.makedirs(node_dir, exist_ok=True)
            f_node = os.path.join(node_dir, "package.json")
            with open(f_node, "w", encoding="utf-8") as f:
                f.write("{}")

            # Rust Scanner direct discovery
            scanner = any_context_core_rs.WorkspaceScanner()
            discovered = scanner.discover_files(tmpdir)
            basenames = [os.path.basename(p) for p in discovered]

            assert "main.py" in basenames
            assert "infra.tf" in basenames
            assert "Dockerfile" in basenames
            assert ".env.production" in basenames

            assert "bundle.min.js" not in basenames
            assert "~$doc.docx" not in basenames
            assert "config" not in basenames
            assert "package.json" not in basenames
            assert len(discovered) == 4

            # Python bridge wrapper parity
            py_discovered = discover_workspace_files(tmpdir)
            assert len(py_discovered) == 4
            py_basenames = [os.path.basename(p) for p in py_discovered]
            assert set(basenames) == set(py_basenames)

    def test_scanner_scan_and_diff_lifecycle(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            scanner = any_context_core_rs.WorkspaceScanner()

            # Create file A and file B
            fa = os.path.join(tmpdir, "file_a.txt")
            with open(fa, "w", encoding="utf-8") as f:
                f.write("Content of A\n")

            fb = os.path.join(tmpdir, "file_b.txt")
            with open(fb, "w", encoding="utf-8") as f:
                f.write("Content of B (different size)\n")

            fa_norm = os.path.abspath(fa)
            fb_norm = os.path.abspath(fb)

            # --- Test 1: Virgin scan (empty cache) ---
            diff1 = scanner.scan_and_diff([tmpdir], {})
            assert len(diff1["new_files"]) == 2
            assert len(diff1["modified_files"]) == 0
            assert len(diff1["deleted_files"]) == 0
            assert len(diff1["renamed_files"]) == 0
            assert diff1["total_disk_files"] == 2

            # Simulate caching file A and file B
            cached = {
                fa_norm: {
                    "last_mtime": os.path.getmtime(fa),
                    "file_size": os.path.getsize(fa)
                },
                fb_norm: {
                    "last_mtime": os.path.getmtime(fb),
                    "file_size": os.path.getsize(fb)
                }
            }

            # --- Test 2: Up to date (0 changes) ---
            diff2 = scanner.scan_and_diff([tmpdir], cached)
            assert len(diff2["new_files"]) == 0
            assert len(diff2["modified_files"]) == 0
            assert len(diff2["deleted_files"]) == 0
            assert len(diff2["renamed_files"]) == 0

            # --- Test 3: Modified file A ---
            time.sleep(0.05)
            with open(fa, "a", encoding="utf-8") as f:
                f.write("Appended line\n")

            diff3 = scanner.scan_and_diff([tmpdir], cached)
            assert len(diff3["modified_files"]) == 1
            assert os.path.abspath(diff3["modified_files"][0]) == fa_norm

            # --- Test 4: Renamed file B -> file_b_renamed.txt ($0.00 rename match) ---
            f_renamed = os.path.join(tmpdir, "file_b_renamed.txt")
            os.rename(fb, f_renamed)

            # Re-update cache for file A to isolate rename
            cached[fa_norm] = {
                "last_mtime": os.path.getmtime(fa),
                "file_size": os.path.getsize(fa)
            }

            diff4 = scanner.scan_and_diff([tmpdir], cached)
            assert len(diff4["renamed_files"]) == 1
            old_p, new_p = diff4["renamed_files"][0]
            assert os.path.abspath(old_p) == fb_norm
            assert os.path.abspath(new_p) == os.path.abspath(f_renamed)
            assert len(diff4["deleted_files"]) == 0
            assert len(diff4["new_files"]) == 0
