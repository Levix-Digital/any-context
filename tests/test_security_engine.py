"""
Unit and Integration Tests for AnyContext SecurityEngine & Hardware Encryption.
100% native unittest.TestCase without pytest dependencies.
"""
import os
import sys
import shutil
import tempfile
import unittest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from any_context.core.security_engine import SecurityEngine
from any_context.vector_engine.store import LanceDBStore


class TestSecurityEngine(unittest.TestCase):
    def test_security_engine_encryption_decryption(self):
        engine = SecurityEngine.get_instance()
        original_text = "This is confidential context document text for ProvincialImmigration."
        encrypted = engine.encrypt_text(original_text)

        self.assertTrue(encrypted.startswith("enc::"))
        self.assertNotEqual(encrypted, original_text)

        decrypted = engine.decrypt_text(encrypted)
        self.assertEqual(decrypted, original_text)

    def test_security_engine_tamper_detection(self):
        engine = SecurityEngine.get_instance()
        original_text = "Highly sensitive corporate business plan."
        encrypted = engine.encrypt_text(original_text)

        # Tamper with the ciphertext
        raw_b64 = list(encrypted)
        raw_b64[-5] = "A" if raw_b64[-5] != "A" else "B"
        tampered = "".join(raw_b64)

        decrypted = engine.decrypt_text(tampered)
        self.assertEqual(decrypted, "[Protected Context Data - Hardware Key Mismatch]")

    def test_security_engine_machine_binding_isolation(self):
        engine_machine_a = SecurityEngine(machine_id_override="machine_alpha_uuid_1111")
        engine_machine_b = SecurityEngine(machine_id_override="machine_beta_uuid_2222")

        secret_text = "Patent proprietary algorithm details."
        encrypted_on_a = engine_machine_a.encrypt_text(secret_text)

        # Machine A decrypts perfectly
        self.assertEqual(engine_machine_a.decrypt_text(encrypted_on_a), secret_text)

        # Machine B cannot decrypt Machine A's data
        decrypted_on_b = engine_machine_b.decrypt_text(encrypted_on_a)
        self.assertEqual(decrypted_on_b, "[Protected Context Data - Hardware Key Mismatch]")

    def test_lancedb_hardware_encryption_integration(self):
        temp_dir = tempfile.mkdtemp(prefix="actx_test_sec_lancedb_")
        try:
            store = LanceDBStore(db_path=temp_dir)

            test_records = [
                {
                    "id": "test_chunk_1",
                    "vector": [0.05] * 1536,
                    "text": "Alberta immigrant entrepreneur immigration program guidelines.",
                    "file_name": "alberta_guide.pdf",
                    "file_path": "/docs/alberta_guide.pdf",
                    "workspace": "TestSecWS",
                    "last_modified": "2026-08-27",
                    "content_type": "PDF Document",
                    "document_summary": "Summary of Alberta entrepreneur routes.",
                    "keywords": "alberta, entrepreneur, immigration",
                    "content_hash": "hash_sec_1"
                }
            ]

            store.upsert_records(test_records, table_name="workspace_chunks", dim=1536)

            # 1. Verify that raw disk table contains encrypted ciphertext for payload content
            table = store.get_table("workspace_chunks", dim=1536)
            raw_rows = table.search().limit(1).to_list()
            self.assertEqual(len(raw_rows), 1)
            self.assertTrue(raw_rows[0]["text"].startswith("enc::"))
            self.assertTrue(raw_rows[0]["document_summary"].startswith("enc::"))
            self.assertTrue(raw_rows[0]["keywords"].startswith("enc::"))
            self.assertEqual(raw_rows[0]["file_path"], "/docs/alberta_guide.pdf")

            # 2. Verify that vector search transparently decrypts and returns plaintext ScoredChunk
            scored_chunks = store.search_vector(
                query_vector=[0.05] * 1536,
                limit=5,
                workspace="TestSecWS",
                table_name="workspace_chunks"
            )
            self.assertEqual(len(scored_chunks), 1)
            chunk = scored_chunks[0]
            self.assertEqual(chunk.text, "Alberta immigrant entrepreneur immigration program guidelines.")
            self.assertEqual(chunk.document_summary, "Summary of Alberta entrepreneur routes.")
            self.assertEqual(chunk.file_path, "/docs/alberta_guide.pdf")
            self.assertEqual(chunk.workspace, "TestSecWS")
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
