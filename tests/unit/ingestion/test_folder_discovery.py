import os
import tempfile
import pytest
from any_context.ingestion.local_folder_ingestor import (
    discover_workspace_files,
    SUPPORTED_EXTENSIONS,
    SUPPORTED_FILENAMES
)


class TestFolderDiscovery:
    """
    Validates discovery of API schemas (.proto, .graphql), Cloud/IaC (.tf),
    special files without extension (Dockerfile, Makefile), and rejection of minified/ignored files.
    """

    def test_schema_and_iac_extensions_in_supported_list(self):
        assert ".proto" in SUPPORTED_EXTENSIONS
        assert ".graphql" in SUPPORTED_EXTENSIONS
        assert ".gql" in SUPPORTED_EXTENSIONS
        assert ".thrift" in SUPPORTED_EXTENSIONS
        assert ".tf" in SUPPORTED_EXTENSIONS
        assert ".tfvars" in SUPPORTED_EXTENSIONS
        assert ".hcl" in SUPPORTED_EXTENSIONS
        assert ".bicep" in SUPPORTED_EXTENSIONS
        assert ".mod" in SUPPORTED_EXTENSIONS
        assert ".sum" in SUPPORTED_EXTENSIONS
        assert ".gradle" in SUPPORTED_EXTENSIONS
        assert ".properties" in SUPPORTED_EXTENSIONS
        assert ".rst" in SUPPORTED_EXTENSIONS

    def test_supported_filenames_without_extension(self):
        assert "dockerfile" in SUPPORTED_FILENAMES
        assert "containerfile" in SUPPORTED_FILENAMES
        assert "jenkinsfile" in SUPPORTED_FILENAMES
        assert "makefile" in SUPPORTED_FILENAMES

    def test_discover_workspace_files_real_mix(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            # 1. Valid schemas and code
            proto_path = os.path.join(tmpdir, "service.proto")
            with open(proto_path, "w", encoding="utf-8") as f:
                f.write('syntax = "proto3";\nmessage Ping { string msg = 1; }\n')

            gql_path = os.path.join(tmpdir, "schema.graphql")
            with open(gql_path, "w", encoding="utf-8") as f:
                f.write("type Query { hello: String }\n")

            tf_path = os.path.join(tmpdir, "main.tf")
            with open(tf_path, "w", encoding="utf-8") as f:
                f.write('resource "aws_s3_bucket" "b" { bucket = "test" }\n')

            # 2. Files without extension
            dockerfile_path = os.path.join(tmpdir, "Dockerfile")
            with open(dockerfile_path, "w", encoding="utf-8") as f:
                f.write("FROM alpine:latest\nCMD ['echo', 'ok']\n")

            makefile_path = os.path.join(tmpdir, "Makefile")
            with open(makefile_path, "w", encoding="utf-8") as f:
                f.write("all:\n\t@echo build\n")

            # 3. Build manifests
            mod_path = os.path.join(tmpdir, "go.mod")
            with open(mod_path, "w", encoding="utf-8") as f:
                f.write("module example.com/test\ngo 1.22\n")

            # 4. Filtered files (should NOT be discovered)
            min_js_path = os.path.join(tmpdir, "bundle.min.js")
            with open(min_js_path, "w", encoding="utf-8") as f:
                f.write("var a=1;")

            unsupported_path = os.path.join(tmpdir, "archive.bin")
            with open(unsupported_path, "w", encoding="utf-8") as f:
                f.write("data")

            # Execute discovery
            discovered = discover_workspace_files(tmpdir)
            discovered_basenames = [os.path.basename(p) for p in discovered]

            # Verify included
            assert "service.proto" in discovered_basenames
            assert "schema.graphql" in discovered_basenames
            assert "main.tf" in discovered_basenames
            assert "Dockerfile" in discovered_basenames
            assert "Makefile" in discovered_basenames
            assert "go.mod" in discovered_basenames

            # Verify excluded
            assert "bundle.min.js" not in discovered_basenames
            assert "archive.bin" not in discovered_basenames
            assert len(discovered) == 6
