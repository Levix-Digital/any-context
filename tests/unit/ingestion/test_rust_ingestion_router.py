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
        self.assertTrue(self.router.supports_file("src/index.ts"))
        self.assertTrue(self.router.supports_file("src/App.tsx"))
        self.assertTrue(self.router.supports_file("server.js"))
        self.assertTrue(self.router.supports_file("components/Widget.jsx"))
        self.assertTrue(self.router.supports_file("UserService.java"))
        self.assertTrue(self.router.supports_file("OrderController.cs"))
        self.assertTrue(self.router.supports_file("main.go"))
        self.assertTrue(self.router.supports_file("src/lib.rs"))
        self.assertTrue(self.router.supports_file("kernel.c"))
        self.assertTrue(self.router.supports_file("include/header.h"))
        self.assertTrue(self.router.supports_file("engine.cpp"))
        self.assertTrue(self.router.supports_file("include/engine.hpp"))
        self.assertTrue(self.router.supports_file("MainActivity.kt"))
        self.assertTrue(self.router.supports_file("build.gradle.kts"))
        self.assertTrue(self.router.supports_file("AppCoordinator.swift"))
        self.assertTrue(self.router.supports_file("models/user.rb"))
        self.assertTrue(self.router.supports_file("index.php"))
        self.assertTrue(self.router.supports_file("view.phtml"))
        self.assertTrue(self.router.supports_file("init.lua"))
        self.assertTrue(self.router.supports_file("main.dart"))
        self.assertTrue(self.router.supports_file("pom.xml"))
        self.assertTrue(self.router.supports_file("package.json"))
        self.assertTrue(self.router.supports_file("events.jsonl"))
        self.assertTrue(self.router.supports_file("records.ndjson"))
        self.assertTrue(self.router.supports_file("docker-compose.yml"))
        self.assertTrue(self.router.supports_file("manifest.yaml"))
        self.assertTrue(self.router.supports_file("Cargo.toml"))
        self.assertTrue(self.router.supports_file("data.csv"))
        self.assertTrue(self.router.supports_file("catalog.tsv"))
        self.assertTrue(self.router.supports_file("data.xlsx"))
        self.assertTrue(self.router.supports_file("legacy.xls"))
        self.assertTrue(self.router.supports_file("budget.ods"))
        self.assertTrue(self.router.supports_file("statement.ofx"))
        self.assertTrue(self.router.supports_file("report.pdf"))
        self.assertTrue(self.router.supports_file("image.png"))
        self.assertFalse(self.router.supports_file("binary.bin"))

    def test_typescript_ast_chunking(self):
        code = (
            "import React from 'react';\n\n"
            "export interface UserProps {\n"
            "  name: string;\n"
            "  role: string;\n"
            "}\n\n"
            "export const UserCard: React.FC<UserProps> = ({ name, role }) => {\n"
            "  return <div>{name} ({role})</div>;\n"
            "};\n\n"
            "export function formatUserName(user: UserProps): string {\n"
            "  return user.name.toUpperCase();\n"
            "}\n"
        )
        chunks = self.router.chunk_text("components/UserCard.tsx", code)
        self.assertGreaterEqual(len(chunks), 2)
        self.assertTrue(any("const UserCard" in c["text"] for c in chunks))
        self.assertTrue(any("function formatUserName" in c["text"] for c in chunks))

    def test_java_ast_chunking(self):
        code = (
            "package com.example.shop;\n\n"
            "import org.springframework.stereotype.Service;\n\n"
            "/** Order processing service. */\n"
            "@Service\n"
            "public class OrderService {\n"
            "    public Order findById(Long id) {\n"
            "        return new Order(id);\n"
            "    }\n"
            "}\n"
        )
        chunks = self.router.chunk_text("OrderService.java", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class OrderService" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "java")

    def test_csharp_ast_chunking(self):
        code = (
            "namespace Api.Controllers;\n\n"
            "using Microsoft.AspNetCore.Mvc;\n\n"
            "[ApiController]\n"
            "[Route(\"api/[controller]\")]\n"
            "public class ProductsController : ControllerBase\n"
            "{\n"
            "    [HttpGet]\n"
            "    public IActionResult List() => Ok();\n"
            "}\n"
        )
        chunks = self.router.chunk_text("ProductsController.cs", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class ProductsController" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "csharp")

    def test_go_ast_chunking(self):
        code = (
            "package main\n\n"
            "import \"fmt\"\n\n"
            "type Config struct {\n"
            "    Port int\n"
            "}\n\n"
            "func (c *Config) Addr() string {\n"
            "    return fmt.Sprintf(\":%d\", c.Port)\n"
            "}\n\n"
            "func StartServer(cfg *Config) error {\n"
            "    return nil\n"
            "}\n"
        )
        chunks = self.router.chunk_text("server.go", code)
        self.assertGreaterEqual(len(chunks), 3)
        self.assertTrue(any("struct Config" in c["text"] for c in chunks))
        self.assertTrue(any("method (c *Config) Addr" in c["text"] for c in chunks))
        self.assertTrue(any("function StartServer" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "go")

    def test_rust_ast_chunking(self):
        code = (
            "pub struct Router {\n"
            "    routes: Vec<String>,\n"
            "}\n\n"
            "impl Router {\n"
            "    pub fn new() -> Self {\n"
            "        Self { routes: Vec::new() }\n"
            "    }\n"
            "}\n\n"
            "pub fn init_router() -> Router {\n"
            "    Router::new()\n"
            "}\n"
        )
        chunks = self.router.chunk_text("router.rs", code)
        self.assertGreaterEqual(len(chunks), 2)
        self.assertTrue(any("struct Router" in c["text"] for c in chunks))
        self.assertTrue(any("impl Router" in c["text"] for c in chunks))
        self.assertTrue(any("fn init_router" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "rust")

    def test_cpp_ast_chunking(self):
        code = (
            "#include <iostream>\n\n"
            "namespace core {\n"
            "    class App {\n"
            "    public:\n"
            "        void start();\n"
            "    };\n"
            "}\n\n"
            "int main() {\n"
            "    return 0;\n"
            "}\n"
        )
        chunks = self.router.chunk_text("main.cpp", code)
        self.assertGreaterEqual(len(chunks), 2)
        self.assertTrue(any("namespace core > class App" in c["text"] for c in chunks))
        self.assertTrue(any("function main" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "cpp")

    def test_c_ast_chunking(self):
        code = (
            "#include <stdio.h>\n\n"
            "struct Buffer {\n"
            "    char* data;\n"
            "    size_t len;\n"
            "};\n\n"
            "int write_buffer(struct Buffer* buf, const char* msg) {\n"
            "    return 0;\n"
            "}\n"
        )
        chunks = self.router.chunk_text("buffer.c", code)
        self.assertGreaterEqual(len(chunks), 2)
        self.assertTrue(any("struct Buffer" in c["text"] for c in chunks))
        self.assertTrue(any("function write_buffer" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "c")

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
        router = IngestionRouter(max_chunk_chars=250)
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
        method_chunk = next((c for c in chunks if (c["header_path"] or "").startswith("class InvoiceService > def create_invoice")), None)
        self.assertIsNotNone(method_chunk)
        self.assertIn("create_invoice", method_chunk["text"])
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

    def test_continuous_table_code_splitting_strictly_bounded(self):
        # Continuous table-driven function without any \n\n (e.g. Go test / TypeScript declarations)
        continuous_lines = ["func TestHugeSuite(t *testing.T) {"]
        for i in range(100):
            continuous_lines.append(f"    assert.Equal(t, calculateVal({i}), {i * 10}, \"case {i} must match\")")
        continuous_lines.append("}")
        code = "\n".join(continuous_lines)
        self.assertGreater(len(code), 5000)

        router = IngestionRouter(max_chunk_chars=600)
        chunks = router.chunk_text("suite_test.go", code)
        self.assertGreater(len(chunks), 5)
        for c in chunks:
            self.assertLessEqual(len(c["text"]), 750)  # text includes breadcrumb header + chunk <= max_chunk_chars

    def test_embedding_token_limit_registry(self):
        from any_context.tools.search_tools import get_embedding_token_limit
        from any_context.config.app_settings import AppSettings

        self.assertEqual(get_embedding_token_limit("text-embedding-3-small"), 8191)
        self.assertEqual(get_embedding_token_limit("text-embedding-004"), 2048)
        self.assertEqual(get_embedding_token_limit("nomic-embed-text"), 2048)
        self.assertEqual(get_embedding_token_limit("all-minilm-l6-v2"), 512)
        self.assertEqual(get_embedding_token_limit("custom-unknown-provider"), 2048)

        # Explicit override in AppSettings
        custom_settings = AppSettings()
        custom_settings.models.max_embed_tokens = 4096
        self.assertEqual(get_embedding_token_limit(settings=custom_settings), 4096)

    def test_kotlin_ast_chunking(self):
        code = (
            "package com.example.service\n\n"
            "class UserService(private val repo: UserRepository) {\n"
            "    fun findById(id: Long): User? {\n"
            "        return repo.findById(id)\n"
            "    }\n"
            "}\n"
        )
        chunks = self.router.chunk_text("UserService.kt", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class UserService" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "kotlin")

    def test_swift_ast_chunking(self):
        code = (
            "import Foundation\n\n"
            "struct OrderItem: Codable {\n"
            "    let id: UUID\n"
            "    let title: String\n\n"
            "    func calculateTax(rate: Double) -> Double {\n"
            "        return 10.0 * rate\n"
            "    }\n"
            "}\n"
        )
        chunks = self.router.chunk_text("OrderItem.swift", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("struct OrderItem" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "swift")

    def test_ruby_ast_chunking(self):
        code = (
            "class BillingAccount < ApplicationRecord\n"
            "  def total_spent\n"
            "    orders.sum(&:amount)\n"
            "  end\n"
            "end\n"
        )
        chunks = self.router.chunk_text("billing_account.rb", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class BillingAccount" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "ruby")

    def test_php_ast_chunking(self):
        code = (
            "<?php\n\n"
            "namespace App\\Services;\n\n"
            "class OrderService {\n"
            "    public function createOrder(): int {\n"
            "        return 42;\n"
            "    }\n"
            "}\n"
        )
        chunks = self.router.chunk_text("OrderService.php", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class OrderService" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "php")

    def test_lua_ast_chunking(self):
        code = (
            "local M = {}\n\n"
            "function M:calculate_score(points)\n"
            "    return points * 1.5\n"
            "end\n\n"
            "return M\n"
        )
        chunks = self.router.chunk_text("score.lua", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("function M:calculate_score" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "lua")

    def test_dart_ast_chunking(self):
        code = (
            "import 'package:flutter/material.dart';\n\n"
            "class AppWidget extends StatelessWidget {\n"
            "  @override\n"
            "  Widget build(BuildContext context) {\n"
            "    return Container();\n"
            "  }\n"
            "}\n"
        )
        chunks = self.router.chunk_text("app_widget.dart", code)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("class AppWidget" in c["text"] for c in chunks))
        self.assertEqual(chunks[-1]["content_type"], "dart")

    def test_xml_chunking(self):
        xml = (
            "<project>\n"
            "  <modelVersion>4.0.0</modelVersion>\n"
            "  <groupId>com.example</groupId>\n"
            "  <artifactId>demo-app</artifactId>\n"
            "  <version>1.0.0</version>\n"
            "  <dependencies>\n"
            "    <dependency>\n"
            "      <groupId>org.slf4j</groupId>\n"
            "      <artifactId>slf4j-api</artifactId>\n"
            "      <version>2.0.0</version>\n"
            "    </dependency>\n"
            "  </dependencies>\n"
            "</project>\n"
        )
        chunks = self.router.chunk_text("pom.xml", xml)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("pom.xml > project" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "xml")

    def test_json_chunking(self):
        json_str = (
            "{\n"
            "  \"name\": \"any-context\",\n"
            "  \"version\": \"0.30.7\",\n"
            "  \"description\": \"Universal Context Engine\",\n"
            "  \"author\": \"Levix Digital\"\n"
            "}\n"
        )
        chunks = self.router.chunk_text("package.json", json_str)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("package.json > root" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "json")

    def test_jsonl_chunking(self):
        jsonl_str = (
            "{\"id\": 1, \"event\": \"login\", \"user\": \"alice\"}\n"
            "{\"id\": 2, \"event\": \"purchase\", \"amount\": 99.5}\n"
            "{\"id\": 3, \"event\": \"logout\", \"user\": \"alice\"}\n"
        )
        chunks = self.router.chunk_text("audit.jsonl", jsonl_str)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("audit.jsonl > lines" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "json")

    def test_yaml_chunking(self):
        yaml_str = (
            "version: '3.8'\n"
            "services:\n"
            "  postgres:\n"
            "    image: postgres:15-alpine\n"
            "    ports:\n"
            "      - '5432:5432'\n"
            "    environment:\n"
            "      POSTGRES_DB: appdb\n"
        )
        chunks = self.router.chunk_text("docker-compose.yml", yaml_str)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertTrue(any("docker-compose.yml" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "yaml")

    def test_yaml_multidoc_chunking(self):
        yaml_str = (
            "apiVersion: v1\n"
            "kind: ConfigMap\n"
            "metadata:\n"
            "  name: app-config\n"
            "data:\n"
            "  APP_ENV: production\n"
            "---\n"
            "apiVersion: apps/v1\n"
            "kind: Deployment\n"
            "metadata:\n"
            "  name: api-server\n"
            "spec:\n"
            "  replicas: 2\n"
        )
        chunks = self.router.chunk_text("k8s-manifest.yaml", yaml_str)
        self.assertEqual(len(chunks), 2)
        self.assertTrue(any("ConfigMap: app-config" in c["text"] for c in chunks))
        self.assertTrue(any("Deployment: api-server" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "yaml")
        self.assertEqual(chunks[1]["content_type"], "yaml")

    def test_toml_chunking(self):
        toml_str = (
            "[package]\n"
            "name = \"any-context\"\n"
            "version = \"0.30.8\"\n\n"
            "[dependencies]\n"
            "quick-xml = \"0.37\"\n"
            "serde_json = \"1.0\"\n"
            "toml = \"0.8\"\n\n"
            "[[bin]]\n"
            "name = \"actx\"\n"
            "path = \"src/main.rs\"\n"
        )
        router = IngestionRouter(max_chunk_chars=80)
        chunks = router.chunk_text("Cargo.toml", toml_str)
        self.assertGreaterEqual(len(chunks), 2)
        self.assertTrue(any("[package]" in c["text"] for c in chunks))
        self.assertTrue(any("[dependencies]" in c["text"] for c in chunks))
        self.assertTrue(any("[[bin]]: actx" in c["text"] for c in chunks))
        self.assertEqual(chunks[0]["content_type"], "toml")

    def test_csv_chunking_with_header_propagation(self):
        csv_str = "ID,Name,Salary,Department\n" + "\n".join(
            f"{i},Employee_{i},{50000 + i * 1000},Engineering" for i in range(1, 40)
        )
        router = IngestionRouter(max_chunk_chars=200)
        chunks = router.chunk_text("employees.csv", csv_str)
        self.assertGreater(len(chunks), 1)
        for c in chunks:
            self.assertEqual(c["content_type"], "csv")
            self.assertIn("| ID | Name | Salary | Department |", c["text"])
            self.assertIn("[ID, Name, Salary, Department]", c["header_path"])

    def test_tsv_chunking(self):
        tsv_str = "ProductID\tDescription\tUnitPrice\n101\tMechanical Keyboard\t129.99\n102\tWireless Mouse\t49.99\n"
        chunks = self.router.chunk_text("inventory.tsv", tsv_str)
        self.assertEqual(len(chunks), 1)
        self.assertEqual(chunks[0]["content_type"], "tsv")
        self.assertIn("[ProductID, Description, UnitPrice]", chunks[0]["header_path"])
        self.assertIn("| 101 | Mechanical Keyboard | 129.99 |", chunks[0]["text"])

    def test_ofx_financial_chunking(self):
        ofx_str = (
            "OFXHEADER:100\nDATA:OFXSGML\nVERSION:102\n\n"
            "<OFX>\n<BANKMSGSRSV1><STMTTRNRS><STMTRS>\n"
            "<BANKID>001\n<ACCTID>12345-6\n<CURDEF>USD\n"
            "<LEDGERBAL><BALAMT>15420.50</LEDGERBAL>\n"
            "<BANKTRANLIST>\n"
            "<STMTTRN>\n<TRNTYPE>DEBIT\n<DTPOSTED>20260901\n<TRNAMT>-150.00\n<FITID>TX001\n<MEMO>Office Supplies\n</STMTTRN>\n"
            "<STMTTRN>\n<TRNTYPE>CREDIT\n<DTPOSTED>20260905\n<TRNAMT>4200.00\n<FITID>TX002\n<MEMO>Consulting Fee\n</STMTTRN>\n"
            "</BANKTRANLIST>\n"
            "</STMTRS></STMTTRNRS></BANKMSGSRSV1>\n</OFX>\n"
        )
        chunks = self.router.chunk_text("statement.ofx", ofx_str)
        self.assertEqual(len(chunks), 1)
        self.assertEqual(chunks[0]["content_type"], "ofx")
        self.assertIn("Bank: 001 | Acct: 12345-6", chunks[0]["header_path"])
        self.assertIn("| 2026-09-01 | DEBIT | -150.00 | TX001 | Office Supplies |", chunks[0]["text"])
        self.assertIn("| 2026-09-05 | CREDIT | 4200.00 | TX002 | Consulting Fee |", chunks[0]["text"])

    def test_xlsx_chunking(self):
        import io
        import tempfile
        import zipfile

        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("[Content_Types].xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                        '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">\n'
                        '  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>\n'
                        '  <Default Extension="xml" ContentType="application/xml"/>\n'
                        '  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>\n'
                        '  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>\n'
                        '</Types>')
            zf.writestr("_rels/.rels", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">\n'
                        '  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>\n'
                        '</Relationships>')
            zf.writestr("xl/_rels/workbook.xml.rels", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                        '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">\n'
                        '  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>\n'
                        '</Relationships>')
            zf.writestr("xl/workbook.xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                        '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">\n'
                        '  <sheets>\n'
                        '    <sheet name="Finance" sheetId="1" r:id="rId1"/>\n'
                        '  </sheets>\n'
                        '</workbook>')
            zf.writestr("xl/worksheets/sheet1.xml", '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
                        '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">\n'
                        '  <sheetData>\n'
                        '    <row r="1">\n'
                        '      <c r="A1" t="inlineStr"><is><t>Category</t></is></c>\n'
                        '      <c r="B1" t="inlineStr"><is><t>Budget</t></is></c>\n'
                        '    </row>\n'
                        '    <row r="2">\n'
                        '      <c r="A2" t="inlineStr"><is><t>Cloud</t></is></c>\n'
                        '      <c r="B2"><v>5000</v></c>\n'
                        '    </row>\n'
                        '  </sheetData>\n'
                        '</worksheet>')

        data = buf.getvalue()
        with tempfile.NamedTemporaryFile(suffix=".xlsx", delete=False) as f:
            f.write(data)
            f_path = f.name

        try:
            chunks = self.router.chunk_file(f_path)
            self.assertEqual(len(chunks), 1)
            self.assertEqual(chunks[0]["content_type"], "excel")
            self.assertIn('Sheet: "Finance"', chunks[0]["header_path"])
            self.assertIn("[Category, Budget]", chunks[0]["header_path"])
            self.assertIn("| Cloud | 5000 |", chunks[0]["text"])
        finally:
            if os.path.exists(f_path):
                os.remove(f_path)

    def test_pdf_support_and_text_chunking(self):
        self.assertTrue(self.router.supports_file("architecture.pdf"))
        self.assertTrue(self.router.supports_file("report.PDF"))

        text = "Chapter 1: System Design\nThis chapter introduces the native Rust Core of AnyContext."
        chunks = self.router.chunk_text("manual.pdf", text)
        self.assertGreaterEqual(len(chunks), 1)
        self.assertEqual(chunks[0]["content_type"], "pdf")
        self.assertIn("manual.pdf > Content", chunks[0]["header_path"])
        self.assertIn("System Design", chunks[0]["text"])

    def test_pdf_2d_spatial_layout_cmr(self):
        cmr_path = r"C:\Users\guilh\OneDrive\Documents\Documentos\Outros\Shipment Checklist\2026\01\01\CMR for Single Pickup Report.pdf"
        if os.path.exists(cmr_path):
            chunks = self.router.chunk_file(cmr_path)
            self.assertGreaterEqual(len(chunks), 1)
            chunk_text = chunks[0]["text"]
            # Must not be concatenated without separation
            self.assertNotIn("IKEA CALGARYBISON TRANSPORT INC.", chunk_text)
            self.assertIn("IKEA CALGARY", chunk_text)
            self.assertIn("BISON TRANSPORT INC.", chunk_text)
            # Reconstructed as markdown table row
            self.assertIn("| IKEA CALGARY | BISON TRANSPORT INC. |", chunk_text)

    def test_image_support_and_visual_chunking(self):
        for ext in ["png", "jpg", "jpeg", "webp", "PNG", "JPG"]:
            self.assertTrue(self.router.supports_file(f"diagram.{ext}"))

        chunks = self.router.chunk_bytes("system_architecture.png", b"fake image buffer")
        self.assertEqual(len(chunks), 1)
        self.assertEqual(chunks[0]["content_type"], "visual_diagram")
        self.assertIn("system_architecture.png > Visual Diagram", chunks[0]["header_path"])
        self.assertIn("Visual Image & Diagram Specification", chunks[0]["text"])


if __name__ == "__main__":
    unittest.main()
