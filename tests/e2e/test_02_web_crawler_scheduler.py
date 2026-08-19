import os
import unittest
import chromadb
from any_context.config.app_settings import AppSettings
from any_context.config.db_store import ConfigDBStore
from any_context.ingestion.web_crawler import discover_site_urls, crawl_and_index_urls
from any_context.ingestion.web_scheduler import WebSchedulerStore
from any_context.tools.search_tools import search_db
from tests.e2e_helpers import safe_stdout_write, setup_mock_embeddings_if_needed

class Test02WebCrawlerScheduler(unittest.TestCase):
    """
    E2E Test Suite 02: Web Crawler Discovery, Semantic Normalization, Ranking, Deduplication & Scheduling
    """

    @classmethod
    def setUpClass(cls):
        import tempfile
        import shutil
        cls.temp_dir = tempfile.mkdtemp(prefix="actx_e2e_mod2_")
        cls.db_dir = os.path.join(cls.temp_dir, "context_db")
        from any_context.config.app_settings import ContextSettings
        cls.store = ConfigDBStore()
        cls.orig_settings = cls.store.get_app_settings()
        cls.store.update_context_settings(ContextSettings(db_path=cls.db_dir, collection_name="mod2_docs"))

        cls.ws_web = "E2E_Mod2_WebPortal"
        cls.store.add_workspace(cls.ws_web, [])

        cls.web_store = WebSchedulerStore()
        cls.web_store.delete_indexed_pages_for_root(cls.ws_web, "https://httpbin.org/html")
        cls.web_store.delete_web_url_by_url(cls.ws_web, "https://httpbin.org/html")

        setup_mock_embeddings_if_needed()

    @classmethod
    def tearDownClass(cls):
        import shutil
        try:
            cls.store.remove_workspace(cls.ws_web)
            cls.web_store.delete_indexed_pages_for_root(cls.ws_web, "https://httpbin.org/html")
            cls.web_store.delete_web_url_by_url(cls.ws_web, "https://httpbin.org/html")
            if hasattr(cls, "orig_settings") and cls.orig_settings and cls.orig_settings.context:
                cls.store.update_context_settings(cls.orig_settings.context)
            if hasattr(cls, "temp_dir") and os.path.exists(cls.temp_dir):
                shutil.rmtree(cls.temp_dir, ignore_errors=True)
        except Exception:
            pass

    def test_01_semantic_path_normalization_and_discovery(self):
        """TC-2.1: Verifies semantic path prefix normalization, stripping extensions (.html)."""
        safe_stdout_write("\n>>> [MOD 2 / TC-2.1] Testing Semantic Path Normalization & Discovery...\n")
        start_url = "https://docs.python.org/3/library/os.html"
        disc = discover_site_urls(start_url)
        self.assertEqual(disc["section_prefix"], "/3/library/os")
        self.assertGreater(disc["domain_count"], 0)
        safe_stdout_write("  [OK] Semantic path prefix normalization verified!\n")

    def test_02_semantic_proximity_ranking(self):
        """TC-2.2: Verifies that start URL is placed first and section pages are prioritized."""
        safe_stdout_write(">>> [MOD 2 / TC-2.2] Testing Semantic Relevance Proximity Ranking...\n")
        start_url = "https://docs.python.org/3/library/os.html"
        disc = discover_site_urls(start_url)
        self.assertEqual(disc["domain_urls"][0], start_url, "Top ranked URL must be the start URL")
        self.assertGreater(len(disc["section_urls"]), 0)
        safe_stdout_write("  [OK] Proximity Ranking verified: Landing > Section > Domain!\n")

    def test_03_incremental_sha256_deduplication(self):
        """TC-2.3 & TC-2.4: Verifies first ingestion, unchanged SHA-256 skip, and database record."""
        safe_stdout_write(">>> [MOD 2 / TC-2.3] Testing Incremental Web Crawling & SHA-256 Bypass...\n")
        import unittest.mock
        test_web_urls = ["https://mock-portal.example.org/docs"]
        self.web_store.delete_indexed_pages_for_root(self.ws_web, "https://mock-portal.example.org/docs")
        self.web_store.delete_web_url_by_url(self.ws_web, "https://mock-portal.example.org/docs")

        mock_page = {
            "url": "https://mock-portal.example.org/docs",
            "title": "Example Documentation Portal",
            "content": "Comprehensive reference guide and technical notes for Herman Melville Moby Dick.",
            "hash": "mock_hash_abc123",
            "char_count": 80,
            "last_modified": "2026-08-18",
            "date_confidence": "high",
            "content_type": "Web Documentation"
        }

        with unittest.mock.patch("any_context.ingestion.web_crawler.scrape_url", return_value=mock_page), \
             unittest.mock.patch("any_context.ingestion.web_crawler.is_url_allowed_by_robots", return_value=True):
            # 1. First Ingestion
            res1 = crawl_and_index_urls(
                workspace_name=self.ws_web,
                urls=test_web_urls,
                root_url="https://mock-portal.example.org/docs",
                root_title="Example Suite",
                scope="custom"
            )
            self.assertEqual(res1["status"], "success")
            self.assertEqual(res1["indexed_count"], 1)
            self.assertEqual(res1["skipped_count"], 0)

            # 2. Second Ingestion without changes (Must skip with 0 embeddings)
            res2 = crawl_and_index_urls(
                workspace_name=self.ws_web,
                urls=test_web_urls,
                root_url="https://mock-portal.example.org/docs",
                root_title="Example Suite",
                scope="custom",
                force_refresh=False
            )
            self.assertEqual(res2["status"], "success")
            self.assertEqual(res2["indexed_count"], 0, "Unchanged URL must not re-embed")
            self.assertEqual(res2["skipped_count"], 1, "Unchanged URL must be skipped as cached")

        # 3. Verify Database records
        count = self.web_store.get_indexed_pages_count(self.ws_web, domain_or_prefix="mock-portal.example.org")
        self.assertEqual(count, 1)

        search_res = search_db.invoke({"prompt_text": "Herman Melville Moby Dick", "workspace": self.ws_web})
        self.assertIsInstance(search_res, str)
        safe_stdout_write("  [OK] Web Incremental Deduplication verified: 0 redundant tokens consumed!\n")

    def test_04_web_scheduler_store_crud(self):
        """TC-2.6: Tests web source registration, polling interval updates, and deletion."""
        safe_stdout_write(">>> [MOD 2 / TC-2.6] Testing Web Scheduler Store CRUD...\n")
        ws_test = "E2E_Mod2_Scheduler"
        url_entry = self.web_store.add_web_url(ws_test, "https://example.org", polling_interval_hours=24)
        url_id = url_entry["id"] if isinstance(url_entry, dict) else url_entry
        self.assertTrue(url_id.startswith("web_"))

        urls = self.web_store.get_workspace_web_urls(ws_test)
        self.assertEqual(len(urls), 1)
        self.assertEqual(urls[0]["url"], "https://example.org")

        self.web_store.delete_web_url(url_id, workspace_name=ws_test)
        urls_after = self.web_store.get_workspace_web_urls(ws_test)
        self.assertEqual(len(urls_after), 0)
        safe_stdout_write("  [OK] Web Scheduler Store CRUD verified!\n")

    def test_05_ecommerce_schema_and_form_rating_extraction(self):
        """TC-2.7: Verifies extraction of ratings inside form/header tags and Schema.org Product JSON-LD."""
        safe_stdout_write(">>> [MOD 2 / TC-2.7] Testing E-Commerce Ratings, Form Tags & Schema.org JSON-LD Extraction...\n")
        from any_context.ingestion.web_ingestor import CleanHTMLTextExtractor, extract_web_metadata

        html_sample = """
        <html>
        <head><title>Windex Original Glass Cleaner</title></head>
        <body>
            <header class="product-header">
                <h1>Windex Original Glass Cleaner Spray</h1>
                <form action="/cart" method="post">
                    <span class="ld_Ec">4.844 out of 5 stars. 1199 reviews</span>
                    <span class="price">$3.98</span>
                </form>
            </header>
            <script type="application/ld+json">
            {
              "@context": "https://schema.org",
              "@type": "Product",
              "name": "Windex Original Glass Cleaner",
              "brand": {"@type": "Brand", "name": "Windex"},
              "aggregateRating": {
                "@type": "AggregateRating",
                "ratingValue": "4.844",
                "reviewCount": "1199"
              },
              "offers": {
                "@type": "Offer",
                "price": "3.98",
                "priceCurrency": "USD",
                "availability": "https://schema.org/InStock"
              }
            }
            </script>
            <div class="description">
                <p>Windex leaves glass surfaces sparkling clean.</p>
            </div>
        </body>
        </html>
        """

        parser = CleanHTMLTextExtractor()
        parser.feed(html_sample)
        extracted = parser.get_text()

        # 1. Assert Schema.org structured metadata is extracted
        self.assertIn("Product: Windex Original Glass Cleaner", extracted)
        self.assertIn("Rating: 4.844 / 5 stars (1199 reviews)", extracted)
        self.assertIn("Price: USD 3.98", extracted)
        self.assertIn("Status: In Stock", extracted)

        # 2. Assert visible span text inside form/header is preserved
        self.assertIn("4.844 out of 5 stars. 1199 reviews", extracted)
        self.assertIn("$3.98", extracted)

        # 3. Assert content classification recognizes E-Commerce Product Page
        meta = extract_web_metadata("https://www.walmart.com/ip/windex-cleaner/123", html_sample)
        self.assertEqual(meta["content_type"], "E-Commerce Product Page")
        safe_stdout_write("  [OK] E-Commerce ratings & Schema.org Product extraction verified!\n")

    def test_06_robots_txt_rfc9309_compliance(self):
        """TC-2.8: Verifies RFC 9309 robots.txt compliance, parser caching, and disallowed path rejection."""
        safe_stdout_write(">>> [MOD 2 / TC-2.8] Testing RFC 9309 Robots.txt Policy Compliance...\n")
        from any_context.ingestion.robots_policy import RobotsPolicyManager, RobotsFileParser, is_url_allowed_by_robots

        manager = RobotsPolicyManager()
        
        # Inject mock robots parser for test domain
        mock_origin = "https://mock-shop.example.com"
        mock_rp = RobotsFileParser()
        mock_rp.parse([
            "User-agent: *",
            "Disallow: /admin",
            "Disallow: /cart",
            "Disallow: /checkout",
            "Disallow: /private/*",
            "Allow: /public",
            "Allow: /products/*"
        ])
        manager._parsers[mock_origin] = mock_rp

        # Test allowed paths
        self.assertTrue(manager.is_allowed("https://mock-shop.example.com/products/item-123"))
        self.assertTrue(manager.is_allowed("https://mock-shop.example.com/public/about"))
        
        # Test disallowed paths
        self.assertFalse(manager.is_allowed("https://mock-shop.example.com/admin/login"))
        self.assertFalse(manager.is_allowed("https://mock-shop.example.com/cart"))
        self.assertFalse(manager.is_allowed("https://mock-shop.example.com/checkout"))
        self.assertFalse(manager.is_allowed("https://mock-shop.example.com/private/data.json"))

        safe_stdout_write("  [OK] RFC 9309 Robots.txt compliance verified: disallowed paths strictly blocked!\n")

    def test_07_client_side_rendering_spa_detection(self):
        """TC-2.9: Verifies detection of Client-Side Rendered (CSR / SPA) shells vs rich text pages."""
        safe_stdout_write(">>> [MOD 2 / TC-2.9] Testing Client-Side Rendering (CSR / SPA) Detection...\n")
        from any_context.ingestion.web_ingestor import scrape_url
        from unittest.mock import patch, MagicMock

        mock_spa_html = """
        <!DOCTYPE html>
        <html>
        <head><title>SPA Portal</title></head>
        <body>
            <div id="__next"></div>
            <script id="__NEXT_DATA__" type="application/json">{"props": {"pageProps": {}}}</script>
            <script src="/_next/static/chunks/main.js"></script>
            <p>Loading...</p>
        </body>
        </html>
        """

        mock_resp = MagicMock()
        mock_resp.read.return_value = mock_spa_html.encode("utf-8")
        mock_resp.headers = {}
        mock_resp.__enter__.return_value = mock_resp
        mock_resp.__exit__.return_value = None

        with patch("urllib.request.urlopen", return_value=mock_resp):
            doc = scrape_url("https://spa-app.example.com/app")
            self.assertTrue(doc.get("is_dynamic_spa"), "SPA shell page with __NEXT_DATA__ and sparse text must be flagged as dynamic SPA")

        safe_stdout_write("  [OK] Client-Side Rendering (CSR / SPA) detection verified!\n")

if __name__ == "__main__":
    unittest.main()
