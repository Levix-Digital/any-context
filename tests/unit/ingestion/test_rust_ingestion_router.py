import os
import tempfile
import unittest
from any_context.ingestion.router import IngestionRouter


class TestRustIngestionRouter(unittest.TestCase):
    def setUp(self):
        self.router = IngestionRouter(max_chunk_chars=1800, overlap_chars=200)

    def test_router_is_rust_accelerated(self):
        self.assertTrue(self.router.is_rust_accelerated, "Rust core extension should be available and active")

    def test_router_supports_file(self):
        self.assertTrue(self.router.supports_file("README.md"))
        self.assertTrue(self.router.supports_file("/docs/architecture.markdown"))
        self.assertTrue(self.router.supports_file("spec.rst"))
        self.assertTrue(self.router.supports_file("guide.mdown"))
        self.assertTrue(self.router.supports_file("main.py"))
        self.assertTrue(self.router.supports_file("src/utils.pyw"))
        self.assertTrue(self.router.supports_file("stubs.pyi"))
        self.assertFalse(self.router.supports_file("report.pdf"))
        self.assertFalse(self.router.supports_file("data.xlsx"))

    def test_python_ast_functions_and_classes(self):
        code = (
            "import os\n"
            "import sys\n\n"
            "def calculate_tax(amount: float, rate: float) -> float:\n"
            "    \"\"\"Calculate total tax for invoice.\"\"\"\n"
            "    return amount * rate\n\n"
            "class InvoiceService:\n"
            "    \"\"\"Service managing enterprise invoices.\"\"\"\n"
            "    tax_rate: float = 0.18\n\n"
            "    @classmethod\n"
            "    def default_service(cls):\n"
            "        return cls()\n\n"
            "    def create_invoice(self, client_id: str, items: list) -> dict:\n"
            "        total = sum(i['price'] for i in items)\n"
            "        tax = calculate_tax(total, self.tax_rate)\n"
            "        return {'client_id': client_id, 'total': total + tax}\n"
        )
        router = IngestionRouter(max_chunk_chars=180)
        chunks = router.chunk_text("services/invoice.py", code)
        self.assertGreaterEqual(len(chunks), 3)

        # Free function chunk
        fn_chunk = next((c for c in chunks if c["header_path"] == "def calculate_tax"), None)
        self.assertIsNotNone(fn_chunk)
        self.assertIn("// Context: invoice.py > def calculate_tax", fn_chunk["text"])
        self.assertIn("Calculate total tax for invoice", fn_chunk["text"])

        # Class Overview chunk
        cls_overview = next((c for c in chunks if "InvoiceService (Overview)" in (c["header_path"] or "")), None)
        self.assertIsNotNone(cls_overview)
        self.assertIn("Service managing enterprise invoices", cls_overview["text"])

        # Class Method chunk
        method_chunk = next((c for c in chunks if c["header_path"] == "class InvoiceService > def create_invoice"), None)
        self.assertIsNotNone(method_chunk)
        self.assertIn("// Context: invoice.py > class InvoiceService > def create_invoice", method_chunk["text"])
        self.assertIn("calculate_tax(total, self.tax_rate)", method_chunk["text"])

    def test_markdown_hierarchy_breadcrumbs(self):
        content = (
            "# System Architecture\n"
            "This is the high level overview of AnyContext.\n\n"
            "## Storage Engine\n"
            "LanceDB provides columnar storage with Apache Arrow.\n\n"
            "### Columnar Vectors\n"
            "Sub-5ms vector queries across 500,000 chunks.\n"
        )
        chunks = self.router.chunk_text("docs/architecture.md", content)
        self.assertEqual(len(chunks), 3)

        # Chunk 0: Level 1
        self.assertEqual(chunks[0]["header_path"], "# System Architecture")
        self.assertIn("// Context: # System Architecture", chunks[0]["text"])
        self.assertIn("high level overview", chunks[0]["text"])

        # Chunk 1: Level 2
        self.assertEqual(chunks[1]["header_path"], "# System Architecture > ## Storage Engine")
        self.assertIn("// Context: # System Architecture > ## Storage Engine", chunks[1]["text"])
        self.assertIn("LanceDB provides columnar storage", chunks[1]["text"])

        # Chunk 2: Level 3
        self.assertEqual(chunks[2]["header_path"], "# System Architecture > ## Storage Engine > ### Columnar Vectors")
        self.assertIn("Sub-5ms vector queries", chunks[2]["text"])

    def test_code_block_hashes_not_treated_as_headings(self):
        content = (
            "# Python Examples\n"
            "Here is sample code:\n\n"
            "```python\n"
            "# This is a python comment, not a heading\n"
            "def test():\n"
            "    pass\n"
            "```\n"
        )
        chunks = self.router.chunk_text("examples.md", content)
        self.assertEqual(len(chunks), 1)
        self.assertEqual(chunks[0]["header_path"], "# Python Examples")
        self.assertIn("# This is a python comment", chunks[0]["text"])

    def test_empty_and_whitespace_markdown(self):
        chunks = self.router.chunk_text("empty.md", "   \n\n  \t  ")
        self.assertEqual(chunks, [])

    def test_chunk_file_from_disk(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False, encoding="utf-8") as f:
            f.write("# Temporary File\nTesting disk reading from Rust engine.\n")
            temp_path = f.name

        try:
            chunks = self.router.chunk_file(temp_path)
            self.assertEqual(len(chunks), 1)
            self.assertEqual(chunks[0]["header_path"], "# Temporary File")
            self.assertIn("Testing disk reading from Rust", chunks[0]["text"])
        finally:
            if os.path.exists(temp_path):
                os.remove(temp_path)


if __name__ == "__main__":
    unittest.main()
