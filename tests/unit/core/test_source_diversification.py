import os
import unittest
from typing import List
from dataclasses import dataclass

from any_context.config.app_settings import ContextSettings, AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.tools.search_tools import _diversify_nodes, safe_stdout_write


@dataclass
class MockNode:
    node_id: str
    text: str
    metadata: dict


class TestSourceDiversification(unittest.TestCase):
    """
    Unit Test Suite: Validates High-Density Source-Fair Round-Robin Diversification
    and Multi-Source RAG Retrieval Presets (Balanced, Turbo, Deep Research).
    """

    def setUp(self):
        self.store = ConfigDBStore()

    def test_01_presets_application(self):
        """Validates that preset switches correctly adjust top_k, candidate_pool, and max_per_source."""
        safe_stdout_write("\n>>> [CORE UNIT] Testing RAG Retrieval Density Presets...\n")
        ctx = ContextSettings()
        self.assertEqual(ctx.retrieval_preset, "balanced")
        self.assertEqual(ctx.top_k, 40)
        self.assertEqual(ctx.candidate_pool_size, 100)
        self.assertEqual(ctx.max_chunks_per_source, 3)

        # Test Turbo Preset
        ctx.apply_preset("turbo")
        self.assertEqual(ctx.retrieval_preset, "turbo")
        self.assertEqual(ctx.top_k, 20)
        self.assertEqual(ctx.candidate_pool_size, 50)
        self.assertEqual(ctx.max_chunks_per_source, 2)

        # Test Deep Research Preset
        ctx.apply_preset("deep_research")
        self.assertEqual(ctx.retrieval_preset, "deep_research")
        self.assertEqual(ctx.top_k, 60)
        self.assertEqual(ctx.candidate_pool_size, 150)
        self.assertEqual(ctx.max_chunks_per_source, 4)

        # Test Balanced Reset
        ctx.apply_preset("balanced")
        self.assertEqual(ctx.retrieval_preset, "balanced")
        self.assertEqual(ctx.top_k, 40)
        self.assertEqual(ctx.candidate_pool_size, 100)
        self.assertEqual(ctx.max_chunks_per_source, 3)
        safe_stdout_write("  [OK] Preset configuration switching verified!\n")

    def test_02_source_fair_round_robin_diversification(self):
        """
        Simulates 100 candidate chunks across 15 distinct sources (e.g. Federal + 14 Provinces).
        Ensures all 15 sources are included in the top-40 chunks without single-source monopoly.
        """
        safe_stdout_write(">>> [CORE UNIT] Testing Multi-Source Round-Robin Diversification...\n")
        raw_nodes: List[MockNode] = []

        # Source 1 (Federal Start-up Visa) has 40 chunks
        for i in range(40):
            raw_nodes.append(MockNode(
                node_id=f"fed_chunk_{i}",
                text=f"Federal Startup Visa chunk {i}",
                metadata={"file_name": "startup_visa_federal.html", "root_url": "https://canada.ca/startup-visa"}
            ))

        # Sources 2 to 15 (14 Provinces) have 4 chunks each = 56 chunks
        provinces = ["alberta", "bc", "ontario", "quebec", "manitoba", "saskatchewan", 
                     "nova_scotia", "new_brunswick", "pei", "newfoundland", "yukon", "nwt", "nunavut", "atlantic"]
        for p in provinces:
            for j in range(4):
                raw_nodes.append(MockNode(
                    node_id=f"{p}_chunk_{j}",
                    text=f"{p.upper()} entrepreneur stream chunk {j}",
                    metadata={"file_name": f"{p}_entrepreneur.html", "root_url": f"https://{p}.ca/business"}
                ))

        self.assertEqual(len(raw_nodes), 96, "Candidate pool should contain 96 nodes")

        # Run diversification with target_top_k=40, max_per_source=3
        diversified = _diversify_nodes(raw_nodes=raw_nodes, target_top_k=40, max_per_source=3)

        self.assertEqual(len(diversified), 40, "Target top_k must be exactly 40 chunks")

        # Verify that all 15 distinct sources are represented in the 40 chunks
        distinct_sources = set()
        source_counts = {}
        for n in diversified:
            s_id = n.metadata.get("root_url")
            distinct_sources.add(s_id)
            source_counts[s_id] = source_counts.get(s_id, 0) + 1

        self.assertEqual(len(distinct_sources), 15, "All 15 distinct web sources must be represented!")
        
        # Verify that Federal is capped at 3 in initial passes (no monopoly)
        self.assertLessEqual(source_counts["https://canada.ca/startup-visa"], 4, "Federal source must not monopolize all slots")
        
        # Verify provincial presence
        for p in provinces:
            self.assertIn(f"https://{p}.ca/business", distinct_sources, f"Province {p} must be present in diversified results")
            self.assertGreaterEqual(source_counts[f"https://{p}.ca/business"], 2, f"Province {p} should have at least 2 chunks")

        safe_stdout_write("  [OK] Multi-source fair round-robin representation (15/15 sources in Top 40) verified!\n")

    def test_03_db_store_context_settings_persistence(self):
        """Validates that ConfigDBStore persists and reloads new retrieval parameters and presets."""
        safe_stdout_write(">>> [CORE UNIT] Testing ConfigDBStore Retrieval Settings Persistence...\n")
        ctx = ContextSettings(
            top_k=45,
            candidate_pool_size=110,
            max_chunks_per_source=4,
            retrieval_preset="custom"
        )
        self.store.update_context_settings(ctx)

        loaded_settings = self.store.get_app_settings()
        self.assertIsNotNone(loaded_settings)
        loaded_ctx = loaded_settings.context
        self.assertEqual(loaded_ctx.top_k, 45)
        self.assertEqual(loaded_ctx.candidate_pool_size, 110)
        self.assertEqual(loaded_ctx.max_chunks_per_source, 4)
        self.assertEqual(loaded_ctx.retrieval_preset, "custom")

        # Reset back to balanced
        ctx.apply_preset("balanced")
        self.store.update_context_settings(ctx)
        reloaded_ctx = self.store.get_app_settings().context
        self.assertEqual(reloaded_ctx.retrieval_preset, "balanced")
        self.assertEqual(reloaded_ctx.top_k, 40)
        safe_stdout_write("  [OK] SQLite ContextSettings persistence and reload verified!\n")


if __name__ == "__main__":
    unittest.main()
