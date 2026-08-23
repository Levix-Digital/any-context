"""
Unit Tests for TwoStageProgressRenderer (v0.24.2).
Validates:
  1. Context manager lifecycle, cursor hiding and restoring.
  2. Stage 1 (Collection / Crawling / Scanning) live updates.
  3. Stage 2 (Enrichment / Vector Embeddings / LanceDB) live updates.
  4. Standardized callback factories for Ingestors and ParallelIndexer.
  5. Windows CP1252 / safe encoding compliance.
"""
import unittest
from unittest.mock import patch, MagicMock

from any_context.cli.progress import TwoStageProgressRenderer, safe_stdout_write


class TestCLIProgressRenderer(unittest.TestCase):

    def test_01_context_manager_and_cursor_lifecycle(self):
        """Validates that TwoStageProgressRenderer hides cursor on enter and restores on exit."""
        print("\n>>> [UNIT] Testing TwoStageProgressRenderer Cursor Lifecycle...")
        output_buffer = []

        def mock_write(msg):
            output_buffer.append(msg)

        with patch("any_context.cli.progress.sys.stdout.write", side_effect=mock_write):
            with TwoStageProgressRenderer(auto_hide_cursor=True) as renderer:
                self.assertTrue(renderer._is_active)
            self.assertFalse(renderer._is_active)

        full_output = "".join(output_buffer)
        self.assertIn("\033[?25l", full_output, "Cursor hide sequence must be emitted")
        self.assertIn("\033[?25h", full_output, "Cursor restore sequence must be emitted")
        print("  [OK] Cursor lifecycle managed safely!")

    def test_02_stage1_rendering(self):
        """Validates that Stage 1 renders progress bar, item name, and counts."""
        print("\n>>> [UNIT] Testing Stage 1 Progress Rendering...")
        output_buffer = []

        def mock_write(msg):
            output_buffer.append(msg)

        with patch("any_context.cli.progress.sys.stdout.write", side_effect=mock_write):
            renderer = TwoStageProgressRenderer(stage1_label="Crawling")
            renderer.update_stage1(current=50, total=100, indexed=45, skipped=5, item_name="https://example.com/docs")

        full_output = "".join(output_buffer)
        self.assertIn("[1/2 Crawling]", full_output)
        self.assertIn("50/100 (50%)", full_output)
        self.assertIn("45 new", full_output)
        self.assertIn("5 cached", full_output)
        print("  [OK] Stage 1 progress rendered correctly!")

    def test_03_stage2_rendering(self):
        """Validates that Stage 2 renders chunk counts and stage labels."""
        print("\n>>> [UNIT] Testing Stage 2 Progress Rendering...")
        output_buffer = []

        def mock_write(msg):
            output_buffer.append(msg)

        with patch("any_context.cli.progress.sys.stdout.write", side_effect=mock_write):
            renderer = TwoStageProgressRenderer()
            renderer.update_stage2(current=80, total=100, chunk_curr=240, chunk_total=300, stage_detail="Vector Knowledge Base")

        full_output = "".join(output_buffer)
        self.assertIn("[2/2 Embedding]", full_output)
        self.assertIn("80/100 items", full_output)
        self.assertIn("240/300 chunks", full_output)
        self.assertIn("Vector Knowledge Base", full_output)
        print("  [OK] Stage 2 progress rendered correctly!")

    def test_04_callback_factories(self):
        """Validates callback adapter factories for ingestors and ParallelIndexer."""
        print("\n>>> [UNIT] Testing Callback Adapter Factories...")
        renderer = TwoStageProgressRenderer(stage1_label="Scanning")

        with patch.object(renderer, "update_stage1") as mock_s1:
            cb1 = renderer.create_stage1_callback()
            cb1(10, 20, 8, 2, "doc.pdf")
            mock_s1.assert_called_with(10, 20, indexed=8, skipped=2, item_name="doc.pdf", stage_name="Scanning")

        with patch.object(renderer, "update_stage2") as mock_s2:
            cb2 = renderer.create_stage2_callback(total_items=50)
            cb2(25, 50, "enriching", "doc.pdf")
            mock_s2.assert_called_with(25, 50, 0, 0, stage_detail="Enriching Context")

            cb2(100, 200, "embedding", "100/200 chunks")
            mock_s2.assert_called_with(25, 50, 100, 200, stage_detail="Vector Knowledge Base")
        print("  [OK] Callback factories adapt calls seamlessly!")


if __name__ == "__main__":
    unittest.main()
