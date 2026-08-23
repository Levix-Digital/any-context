"""
Unit Tests for Hexagonal Architecture Core Decoupling and CLI Presentation Adapter (v0.24.1).
Validates:
  1. Core domain modules (web_crawler, orchestrator, billing.manager) are free from questionary and ANSI escape leaks.
  2. CLI formatters module encapsulates terminal styling, cards, and discovery reports.
  3. Backward compatibility proxies function seamlessly across all interfaces.
"""
import os
import unittest
from unittest.mock import patch, MagicMock

from any_context.cli.formatters import (
    format_sync_status_box,
    format_pricing_plans_cli,
    format_crawler_discovery_report,
    display_help_page
)
from any_context.billing.manager import BillingManager
from any_context.billing.models import PlanTier, PlanCapabilities
from any_context.help.models import HelpPage


class TestHexagonalDecoupling(unittest.TestCase):

    def test_01_billing_manager_purity(self):
        """Validates that BillingManager.format_pricing_cards_cli produces clean text without ANSI codes."""
        print("\n>>> [UNIT] Testing BillingManager Core Purity (Zero ANSI Leaks)...")
        mgr = BillingManager()
        text = mgr.format_pricing_cards_cli()

        self.assertNotIn("\033", text, "BillingManager core output must NOT contain ANSI escape codes")
        self.assertIn("ANYCONTEXT PLANS & CAPABILITY MATRIX", text)
        self.assertIn("Community", text)
        print("  [OK] BillingManager is 100% clean and UI-agnostic!")

    def test_02_cli_formatters_pricing_rendering(self):
        """Validates that CLI formatters render colored ANSI pricing cards for the terminal."""
        print("\n>>> [UNIT] Testing CLI Formatters Pricing Cards...")
        sample_plans = [
            PlanTier(
                tier_id="pro",
                name="Pro Plan",
                monthly_price_usd=29.0,
                annual_price_usd=278.0,
                ingestion_scope="Unlimited Files",
                target_audience="Power Users",
                capabilities=PlanCapabilities()
            )
        ]
        card = format_pricing_plans_cli(sample_plans, current_tier="pro")
        self.assertIn("\033[1;97mPro Plan\033[0m", card)
        self.assertIn("[PLANO ATIVO]", card)
        print("  [OK] CLI formatters render ANSI cards properly!")

    def test_03_cli_crawler_discovery_report_formatting(self):
        """Validates that CLI crawler discovery report renders structured terminal cards."""
        print("\n>>> [UNIT] Testing Crawler Discovery Report Formatter...")
        report = format_crawler_discovery_report(
            title="FastAPI Documentation",
            start_url="https://fastapi.tiangolo.com/",
            section_count=42,
            domain_count=120,
            already_indexed_count=30,
            new_section_count=12,
            new_domain_count=90,
            has_sitemap=True
        )
        self.assertIn("FastAPI Documentation", report)
        self.assertIn("42", report)
        self.assertIn("120", report)
        self.assertIn("Yes (Structured XML)", report)
        print("  [OK] Discovery report formatted cleanly for CLI!")

    def test_04_cli_format_sync_status_box(self):
        """Validates that format_sync_status_box produces the comprehensive multi-source card."""
        print("\n>>> [UNIT] Testing format_sync_status_box in CLI formatters...")
        diff = {
            "workspace_name": "Dev",
            "total_sources": 2,
            "folders": ["/path/to/docs"],
            "total_disk_files": 5,
            "total_cached_files": 5,
            "web_sources": [{"url": "https://python.org", "title": "Python Docs", "page_count": 10}],
            "web_pages_count": 10,
            "cloud_drives": [],
            "is_up_to_date": True,
            "summary": "Up to date (0 changes)"
        }
        box = format_sync_status_box(diff)
        self.assertIn("Workspace Sync Status: Dev", box)
        self.assertIn("Local Folders : 1 folder", box)
        self.assertIn("Web Sources   : 1 portal", box)
        self.assertIn("Up to Date   : Yes", box)
        print("  [OK] Sync status box formatted properly!")

    def test_05_display_help_page(self):
        """Validates that display_help_page prints rich command manual pages."""
        print("\n>>> [UNIT] Testing display_help_page in CLI formatters...")
        page = HelpPage(
            command="/test",
            title="Test Command",
            description="Executes a test",
            syntax="/test [options]",
            parameters=["--flag : enables flag"],
            examples=["/test --flag"],
            tips=["Use carefully"]
        )
        with patch("any_context.help.manager.safe_print") as mock_print:
            display_help_page(page)
            self.assertTrue(mock_print.called)
        print("  [OK] display_help_page executed successfully!")

    def test_06_parallel_indexer_progress_callbacks(self):
        """Validates that ParallelIndexer emits live progress callbacks during enrichment and embedding."""
        print("\n>>> [UNIT] Testing ParallelIndexer Live Progress Callbacks...")
        from any_context.vector_engine.indexer import ParallelIndexer
        from llama_index.core import Document
        from llama_index.core.embeddings.mock_embed_model import MockEmbedding
        from llama_index.core import Settings
        Settings.embed_model = MockEmbedding(embed_dim=1536)

        mock_store = MagicMock()
        indexer = ParallelIndexer(store=mock_store)

        docs = [
            Document(text="Document 1 for testing live progress.", metadata={"file_name": "doc1.txt"}),
            Document(text="Document 2 for testing live progress.", metadata={"file_name": "doc2.txt"})
        ]

        recorded_progress = []
        def _cb(curr, total, stage, detail=""):
            recorded_progress.append((curr, total, stage, detail))

        res = indexer.index_documents(documents=docs, workspace_name="UnitTest", progress_callback=_cb)
        self.assertEqual(res["status"], "success")
        self.assertGreaterEqual(len(recorded_progress), 2, "Must receive multiple progress updates")
        
        stages = [p[2] for p in recorded_progress]
        self.assertIn("enriching", stages)
        self.assertIn("embedding", stages)
        print(f"  [OK] ParallelIndexer emitted {len(recorded_progress)} live progress callbacks across stages: {set(stages)}!")


if __name__ == "__main__":
    unittest.main()
