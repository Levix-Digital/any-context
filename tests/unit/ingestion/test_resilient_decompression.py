import gzip
import zlib
import unittest
from unittest.mock import patch, MagicMock

from any_context.ingestion.web_ingestor import resilient_decompress
from any_context.core.services.source_service import SourceService
from any_context.commands.dispatcher import CommandDispatcher


class TestResilientDecompression(unittest.TestCase):
    """Unit test suite for resilient HTTP/stream decompression and auto-recovery."""

    def test_uncompressed_passthrough(self):
        sample = b"Hello, AnyContext plain text!"
        result = resilient_decompress(sample)
        self.assertEqual(result, sample)

    def test_empty_or_invalid_type(self):
        self.assertEqual(resilient_decompress(b""), b"")
        self.assertEqual(resilient_decompress(None), b"")

    def test_valid_gzip_decompression(self):
        text = b"<html><head><title>Test Page</title></head><body>Hello world!</body></html>"
        compressed = gzip.compress(text)
        result = resilient_decompress(compressed, encoding="gzip")
        self.assertEqual(result, text)

    def test_truncated_gzip_stream_recovery(self):
        """
        Critical test: Verifies that a truncated gzip stream (which normally crashes with
        zlib.error: Error -5 while decompressing data: incomplete or truncated stream)
        is safely recovered by resilient_decompress without raising any exception.
        """
        text = b"<!DOCTYPE html><html><body>" + (b"Useful paragraph content. " * 30) + b"</body></html>"
        compressed = gzip.compress(text)
        # Strip the gzip trailer (last 8-12 bytes)
        truncated = compressed[:-10]

        # Standard decompressors fail with EOFError or zlib Error -5
        with self.assertRaises((EOFError, zlib.error)):
            gzip.decompress(truncated)

        # Resilient decompressor recovers content gracefully
        recovered = resilient_decompress(truncated, encoding="gzip")
        self.assertTrue(len(recovered) > 0)
        self.assertIn(b"Useful paragraph content", recovered)

    def test_valid_deflate_decompression(self):
        text = b"Deflate compressed payload content for test."
        compressed = zlib.compress(text)
        result = resilient_decompress(compressed, encoding="deflate")
        self.assertEqual(result, text)

    def test_source_service_add_web_retry(self):
        service = SourceService()
        call_count = [0]

        def mock_add_web_url(workspace_name, url, **kwargs):
            call_count[0] += 1
            if call_count[0] == 1:
                raise zlib.error("Error -5 while decompressing data: incomplete or truncated stream")
            return {"id": "web_test123", "url": url, "workspace_name": workspace_name}

        with patch("any_context.ingestion.web_scheduler.WebSchedulerStore.add_web_url", side_effect=mock_add_web_url):
            res = service.add_web("Default", "https://example.com/portal")
            self.assertTrue(res["added"])
            self.assertEqual(call_count[0], 2)

    def test_dispatcher_web_add_recovery_and_logging(self):
        dispatcher = CommandDispatcher()
        call_count = [0]

        def flaky_add_web(ws, url):
            call_count[0] += 1
            if call_count[0] == 1:
                raise zlib.error("Error -5 while decompressing data: incomplete or truncated stream")
            return {"added": True, "message": f"Added web source '{url}' to workspace '{ws}'."}

        with patch.object(dispatcher.source_svc, "add_web", side_effect=flaky_add_web):
            with patch.object(dispatcher.sync_svc, "start_sync"):
                result = dispatcher.dispatch("/web --add https://canada.ca/immigration", active_workspace="Default")
                self.assertTrue(result.success)
                self.assertIn("Auto-recovered", result.message)
                self.assertEqual(call_count[0], 2)


if __name__ == "__main__":
    unittest.main()
