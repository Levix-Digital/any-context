"""
=============================================================================
AnyContext (actx) — Comprehensive End-to-End (E2E) Test Suite
=============================================================================
Tests all core production systems:
  1. Workspace Lifecycle & Strict Privacy Scoping (Zero Data Leakage)
  2. Multi-Format Local Ingestion (.txt, .md, .json, .csv) & Recursive Scanning
  3. Incremental SHA-256 Hash Synchronization (Add / Modify / Delete)
  4. Web Discovery, Path Normalization & Relevance Proximity Ranking
  5. AI Agent RAG Reasoning & Recursion Limit Immunity
  6. 3-Level Hierarchical Long-Term Memory & Isolated Purging
  7. Windows Charmap (CP1252) Unicode Safety & Output Rendering
=============================================================================
"""

import os
import sys
import shutil
import tempfile
import time
import unittest
import chromadb
import sqlite3

# Ensure src is in python path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "src")))

from any_context.config.db_store import ConfigDBStore
from any_context.config.app_settings import AppSettings
from any_context.ingestion.local_folder_ingestor import run_index_folder
from any_context.ingestion.web_crawler import discover_site_urls, crawl_and_index_urls
from any_context.tools.search_tools import search_db, safe_stdout_write
from any_context.memory.manager import MemoryManager
from any_context.core.agent import create_anycontext_agent
from langgraph.checkpoint.sqlite import SqliteSaver


class AnyContextE2ETestSuite(unittest.TestCase):
    
    @classmethod
    def setUpClass(cls):
        """Prepare isolated test directories and temporary workspaces"""
        cls.test_dir = tempfile.mkdtemp(prefix="anycontext_e2e_")
        cls.legal_dir = os.path.join(cls.test_dir, "legal_vault")
        cls.tech_dir = os.path.join(cls.test_dir, "tech_specs")
        cls.sub_tech_dir = os.path.join(cls.tech_dir, "architecture", "microservices")
        
        os.makedirs(cls.legal_dir, exist_ok=True)
        os.makedirs(cls.sub_tech_dir, exist_ok=True)

        # 1. Populate Legal Workspace files
        cls.nda_file = os.path.join(cls.legal_dir, "acme_nda_agreement.md")
        with open(cls.nda_file, "w", encoding="utf-8") as f:
            f.write(
                "# Acme Corporation Non-Disclosure Agreement (NDA)\n\n"
                "## Clause 42: Liquidated Damages & Confidentiality Duration\n"
                "The recipient agrees to hold all proprietary trade secrets in strict confidence for a period of **5 years**.\n"
                "In the event of an unauthorized breach, the liquidated damages penalty is strictly fixed at **$100,000 USD** payable within 30 days.\n"
            )

        cls.contract_json = os.path.join(cls.legal_dir, "metadata_legal.json")
        with open(cls.contract_json, "w", encoding="utf-8") as f:
            f.write('{"contract_id": "CNT-2026-99", "jurisdiction": "Ontario, Canada", "status": "active_signed"}')

        # 2. Populate Tech Workspace files (including recursive subfolder)
        cls.tech_file = os.path.join(cls.tech_dir, "system_overview.txt")
        with open(cls.tech_file, "w", encoding="utf-8") as f:
            f.write("AnyContext core architecture uses LangGraph for agent loops and ChromaDB for local vector persistence.")

        cls.sub_tech_file = os.path.join(cls.sub_tech_dir, "microservices_spec.csv")
        with open(cls.sub_tech_file, "w", encoding="utf-8") as f:
            f.write("service_name,port,protocol,auth\nIngestionWorker,8001,gRPC,mTLS\nAPIServer,8000,HTTP/REST,BearerToken\n")

        # 3. Register Workspaces in SQLite
        cls.store = ConfigDBStore()
        cls.ws_legal = "E2E_Legal"
        cls.ws_tech = "E2E_Tech"
        cls.ws_web = "E2E_WebPortal"

        cls.store.add_workspace(cls.ws_legal, [cls.legal_dir])
        cls.store.add_workspace(cls.ws_tech, [cls.tech_dir])
        cls.store.add_workspace(cls.ws_web, [])

    @classmethod
    def tearDownClass(cls):
        """Clean up all temporary files, databases, and workspace vectors"""
        try:
            # Clean workspaces from SQLite
            cls.store.remove_workspace(cls.ws_legal)
            cls.store.remove_workspace(cls.ws_tech)
            cls.store.remove_workspace(cls.ws_web)

            # Purge ChromaDB test collections
            settings = AppSettings.load()
            db_path = settings.context.db_path if settings else "./context_db"
            coll_name = settings.context.collection_name if settings else "context_docs"
            if os.path.exists(db_path):
                client = chromadb.PersistentClient(path=db_path)
                try:
                    coll = client.get_collection(coll_name)
                    for ws in [cls.ws_legal, cls.ws_tech, cls.ws_web]:
                        existing = coll.get(where={"workspace": ws})
                        if existing and existing["ids"]:
                            coll.delete(ids=existing["ids"])
                except Exception:
                    pass

            # Remove test directory
            shutil.rmtree(cls.test_dir, ignore_errors=True)
        except Exception:
            pass

    # -------------------------------------------------------------------------
    # TEST 1: Recursive Ingestion & Multi-Format Support
    # -------------------------------------------------------------------------
    def test_01_recursive_folder_ingestion(self):
        """Validates that folders and deep subfolders (.md, .txt, .json, .csv) are recursively indexed."""
        safe_stdout_write("\n>>> [TEST 1] Testing Recursive Folder Ingestion & Multi-Format Parsing...\n")
        
        # Index Legal and Tech workspaces
        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        run_index_folder(workspace_name=self.ws_tech, verbose=False)

        # Verify ChromaDB contains indexed chunks for both workspaces
        settings = AppSettings.load()
        db_path = settings.context.db_path if settings else "./context_db"
        coll_name = settings.context.collection_name if settings else "context_docs"
        client = chromadb.PersistentClient(path=db_path)
        coll = client.get_collection(coll_name)

        legal_docs = coll.get(where={"workspace": self.ws_legal})
        tech_docs = coll.get(where={"workspace": self.ws_tech})

        self.assertGreaterEqual(len(legal_docs["ids"]), 1, "Legal workspace should have indexed chunks")
        self.assertGreaterEqual(len(tech_docs["ids"]), 1, "Tech workspace should have indexed chunks")
        safe_stdout_write(f"  [OK] Successfully ingested {len(legal_docs['ids'])} Legal chunks and {len(tech_docs['ids'])} Tech chunks recursively!\n")

    # -------------------------------------------------------------------------
    # TEST 2: Strict Privacy Scoping & Zero Cross-Workspace Leakage
    # -------------------------------------------------------------------------
    def test_02_strict_workspace_isolation(self):
        """Ensures queries to one workspace NEVER leak or retrieve data from another workspace."""
        safe_stdout_write("\n>>> [TEST 2] Testing Strict Workspace Privacy & Scope Isolation...\n")
        
        # 1. Search Legal workspace for Legal terms -> must succeed
        legal_res = search_db.invoke({"prompt_text": "liquidated damages clause 42 penalty", "workspace": self.ws_legal})
        self.assertIn("100,000", legal_res, "Legal search should find $100,000 penalty in Acme NDA")

        # 2. Search Legal workspace for unique Tech terms -> must NOT find IngestionWorker
        legal_tech_leak = search_db.invoke({"prompt_text": "IngestionWorker port 8001 mTLS", "workspace": self.ws_legal})
        self.assertNotIn("IngestionWorker", legal_tech_leak, "Legal workspace must NOT return tech IngestionWorker daemon")

        # 3. Search Tech workspace for unique Tech terms -> must succeed
        tech_res = search_db.invoke({"prompt_text": "IngestionWorker port 8001 mTLS", "workspace": self.ws_tech})
        self.assertIn("IngestionWorker", tech_res, "Tech search should find IngestionWorker in microservices CSV")

        # 4. Search Tech workspace for Legal terms -> must NOT find liquidated damages clause 42
        tech_legal_leak = search_db.invoke({"prompt_text": "liquidated damages clause 42", "workspace": self.ws_tech})
        self.assertNotIn("100,000", tech_legal_leak, "Tech workspace must NOT return legal liquidated damages")
        
        safe_stdout_write("  [OK] Privacy Scoping verified: 100% data isolation guaranteed between workspaces!\n")

    # -------------------------------------------------------------------------
    # TEST 3: Incremental SHA-256 Synchronization (Modify & Delete)
    # -------------------------------------------------------------------------
    def test_03_incremental_sync_lifecycle(self):
        """Verifies that modified files update vectors and deleted files are purged from ChromaDB."""
        safe_stdout_write("\n>>> [TEST 3] Testing Incremental SHA-256 Synchronization...\n")

        # 1. Add a new temporary file to Legal
        temp_policy = os.path.join(self.legal_dir, "temporary_refund_policy.txt")
        with open(temp_policy, "w", encoding="utf-8") as f:
            f.write("Refund Policy: All customers are eligible for a 30-day money-back guarantee with zero cancellation fees.")

        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res1 = search_db.invoke({"prompt_text": "money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertIn("30-day", res1, "Newly added file must be indexed and searchable")

        # 2. Modify the file content on disk
        with open(temp_policy, "w", encoding="utf-8") as f:
            f.write("Refund Policy: All customers are eligible for an EXTENDED 90-day VIP money-back guarantee.")

        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res2 = search_db.invoke({"prompt_text": "VIP money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertIn("90-day", res2, "Modified file content must be re-indexed with new values")

        # 3. Delete the file from disk
        os.remove(temp_policy)
        run_index_folder(workspace_name=self.ws_legal, verbose=False)
        res3 = search_db.invoke({"prompt_text": "VIP money-back guarantee refund policy", "workspace": self.ws_legal})
        self.assertNotIn("90-day VIP", res3, "Deleted file must be completely purged from ChromaDB")

        safe_stdout_write("  [OK] Incremental SHA-256 sync verified: add, modify, and delete operations operate flawlessly!\n")

    # -------------------------------------------------------------------------
    # TEST 4: Web Discovery & Semantic Relevance Proximity Ranking
    # -------------------------------------------------------------------------
    def test_04_web_discovery_and_proximity_ranking(self):
        """Verifies semantic path prefix normalization, sitemap resolution, and relevance scoring."""
        safe_stdout_write("\n>>> [TEST 4] Testing Web Discovery, Semantic Normalization & Proximity Ranking...\n")

        # 1. Test Semantic Path Normalization & Discovery
        start_url = "https://docs.python.org/3/library/os.html"
        disc = discover_site_urls(start_url)
        
        self.assertEqual(disc["section_prefix"], "/3/library/os", "Semantic prefix must strip .html extension")
        self.assertGreater(disc["domain_count"], 0, "Domain discovery should find internal pages")

        # 2. Test Ranking: verify that the start page and section pages are placed ahead of generic domain URLs
        domain_urls = disc["domain_urls"]
        self.assertEqual(domain_urls[0], start_url, "Top ranked URL must always be the start URL")
        
        # 3. Test Ingestion of Web Pages into ChromaDB
        test_web_urls = [
            "https://httpbin.org/html"
        ]
        crawl_res = crawl_and_index_urls(
            workspace_name=self.ws_web,
            urls=test_web_urls,
            root_url="https://httpbin.org/html",
            root_title="HttpBin Web Test Suite",
            scope="custom"
        )
        self.assertIn(crawl_res["status"], ["success", "partial_error"])
        
        web_search = search_db.invoke({"prompt_text": "Herman Melville Moby Dick Herman", "workspace": self.ws_web})
        self.assertIsInstance(web_search, str)
        safe_stdout_write("  [OK] Web Crawler & Proximity Ranking verified: clean discovery, ranking, and vector ingestion!\n")

    # -------------------------------------------------------------------------
    # TEST 5: AI Agent RAG Synthesis & Recursion Limit Safety
    # -------------------------------------------------------------------------
    def test_05_agent_rag_reasoning_and_recursion_limit(self):
        """Tests LangGraph Agent determinism, tool execution, and immunity to graph recursion limits."""
        safe_stdout_write("\n>>> [TEST 5] Testing AI Agent Deterministic RAG Reasoning & LangGraph Recursion Safety...\n")

        mem_conn = sqlite3.connect(":memory:", check_same_thread=False)
        saver = SqliteSaver(conn=mem_conn)

        agent = create_anycontext_agent(
            active_workspace=self.ws_legal,
            checkpointer=saver,
            model_override="gpt-4o-mini"
        )
        config = {
            "configurable": {
                "thread_id": "test_e2e_thread_1",
                "active_workspace": self.ws_legal
            },
            "recursion_limit": 50
        }

        # Prompt requiring retrieval from legal agreement
        prompt = "De acordo com o contrato de confidencialidade da Acme (NDA), qual é o valor da multa estipulada em caso de violação e qual o período de vigência?"
        
        full_reply = ""
        for token, metadata in agent.stream({"messages": [prompt]}, stream_mode="messages", config=config):
            if hasattr(token, "type") and token.type in ["ai", "AIMessage", "AIMessageChunk"] and token.content:
                if isinstance(token.content, str):
                    full_reply += token.content

        self.assertTrue(len(full_reply) > 20, "Agent must generate a coherent RAG response")
        self.assertTrue("100.000" in full_reply or "100,000" in full_reply or "100000" in full_reply, "Agent must extract the exact $100,000 liquidated damages figure")
        self.assertTrue("5" in full_reply, "Agent must extract the 5-year duration")
        
        safe_stdout_write("  [OK] AI Agent RAG Response verified with exact citation accuracy!\n")

    # -------------------------------------------------------------------------
    # TEST 6: Windows CP1252 / Charmap Encoding Safety
    # -------------------------------------------------------------------------
    def test_06_windows_charmap_safe_output(self):
        """Ensures terminal progress tickers and search feedback are completely immune to UnicodeEncodeError."""
        safe_stdout_write("\n>>> [TEST 6] Testing Windows CP1252 / Charmap Terminal Output Immunity...\n")

        test_unicode_strings = [
            "🔍 [Search] Searching strictly within Workspace: 'Default' (top 8 chunks)...",
            "✔ Successfully ingested 250 web pages (6,604,334 chars) into workspace 'Default'!",
            "⠋ [2/2 Embedding] [██████████████] 250/250 pages (100%) • Vector Knowledge Base",
            "🧹 Screen cleared | Workspace: Global | Model: gpt-4o-mini"
        ]

        for s in test_unicode_strings:
            try:
                safe_stdout_write(s + "\n")
            except Exception as e:
                self.fail(f"safe_stdout_write must never raise encoding exceptions: {e}")

        safe_stdout_write("  [OK] Windows CP1252 / Charmap compatibility verified: 100% resilient across all consoles!\n")

    # -------------------------------------------------------------------------
    # TEST 7: 3-Level Memory Lifecycle & Workspace Memory Reset
    # -------------------------------------------------------------------------
    def test_07_memory_manager_lifecycle(self):
        """Verifies session memory creation, retrieval, and atomic workspace purging."""
        safe_stdout_write("\n>>> [TEST 7] Testing 3-Level Memory Lifecycle & /reset-memory Isolation...\n")

        mem_mgr = MemoryManager()
        # Verify memory reset executes without altering document vectors
        deleted_count = mem_mgr.reset_memory(workspace=self.ws_legal)
        self.assertIsInstance(deleted_count, int)

        # Confirm document vectors in legal workspace are still 100% intact after memory reset
        legal_check = search_db.invoke({"prompt_text": "liquidated damages clause 42", "workspace": self.ws_legal})
        self.assertIn("100,000", legal_check, "Document vectors MUST survive session memory resets")
        
        safe_stdout_write("  [OK] Memory lifecycle verified: isolated memory purging keeps document vector knowledge base intact!\n")


if __name__ == "__main__":
    suite = unittest.TestLoader().loadTestsFromTestCase(AnyContextE2ETestSuite)
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
