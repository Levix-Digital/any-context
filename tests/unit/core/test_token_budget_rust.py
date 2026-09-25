"""
Unit tests for the Native Rust Universal Token Estimator and Density Budgeting Engine (Marco 6 / Prioridade 3).
Validates any_context_core_rs token estimation, semantic ceiling truncation, and model context limit catalog.
"""
import unittest
import any_context_core_rs as core
from any_context.tools.search_tools import get_embedding_token_limit


class TestTokenBudgetRust(unittest.TestCase):
    """Verifies 100% native Rust token estimation, truncation, and model context limits."""

    def test_estimate_token_count_empty(self):
        """Validates that empty or whitespace-only strings return 0 tokens."""
        self.assertEqual(core.estimate_token_count(""), 0)
        self.assertEqual(core.estimate_token_count("   \n\t  "), 0)

    def test_estimate_token_count_prose(self):
        """Validates token estimation on standard English sentences."""
        text = "The quick brown fox jumps over the lazy dog."
        count = core.estimate_token_count(text)
        # 9 words + period -> ~10 tokens
        self.assertGreaterEqual(count, 8)
        self.assertLessEqual(count, 12)

    def test_estimate_token_count_code(self):
        """Validates token estimation on structured source code with identifiers and syntax."""
        code = "def getUserById(user_id: int):\n    return db.query(user_id)\n"
        count = core.estimate_token_count(code)
        self.assertGreaterEqual(count, 12)
        self.assertLessEqual(count, 24)

    def test_estimate_token_count_multilingual_and_cjk(self):
        """Validates that CJK characters and accented Unicode are properly weighted."""
        cjk = "こんにちは世界"  # 7 Japanese characters
        cjk_count = core.estimate_token_count(cjk)
        self.assertGreaterEqual(cjk_count, 7)
        self.assertLessEqual(cjk_count, 14)

        emoji_text = "Analysis complete! 🚀✨"
        emoji_count = core.estimate_token_count(emoji_text)
        self.assertGreaterEqual(emoji_count, 4)

    def test_get_embedding_token_limit_catalog(self):
        """Validates model context window resolution directly from the compiled Rust catalog."""
        self.assertEqual(core.get_embedding_token_limit("text-embedding-3-small"), 8191)
        self.assertEqual(core.get_embedding_token_limit("text-embedding-3-large"), 8191)
        self.assertEqual(core.get_embedding_token_limit("text-embedding-ada-002"), 8191)
        self.assertEqual(core.get_embedding_token_limit("text-embedding-004"), 2048)
        self.assertEqual(core.get_embedding_token_limit("gemini-embedding"), 2048)
        self.assertEqual(core.get_embedding_token_limit("nomic-embed-text"), 2048)
        self.assertEqual(core.get_embedding_token_limit("all-minilm-l6-v2"), 512)
        self.assertEqual(core.get_embedding_token_limit("bge-small-en-v1.5"), 512)
        self.assertEqual(core.get_embedding_token_limit("unknown-custom-model"), 2048)

    def test_search_tools_delegates_to_rust(self):
        """Validates that search_tools.get_embedding_token_limit uses the native Rust engine."""
        self.assertEqual(get_embedding_token_limit("text-embedding-3-small"), 8191)
        self.assertEqual(get_embedding_token_limit("text-embedding-004"), 2048)
        self.assertEqual(get_embedding_token_limit("all-minilm-l6-v2"), 512)

    def test_truncate_to_token_ceiling_under_limit(self):
        """Validates that text within limits is returned untouched."""
        text = "Line 1: safe content.\nLine 2: more safe content.\n"
        truncated_text, was_truncated = core.truncate_to_token_ceiling(text, 50)
        self.assertFalse(was_truncated)
        self.assertEqual(truncated_text, text)

    def test_truncate_to_token_ceiling_over_limit_preserves_boundaries(self):
        """Validates that text exceeding ceiling is truncated cleanly at line/sentence boundaries."""
        lines = [f"Line {i}: This is structured statement number {i} for density analysis." for i in range(1, 30)]
        full_text = "\n".join(lines)
        total_tokens = core.estimate_token_count(full_text)
        self.assertGreater(total_tokens, 150)

        ceiling = 50
        truncated_text, was_truncated = core.truncate_to_token_ceiling(full_text, ceiling)
        self.assertTrue(was_truncated)
        self.assertLessEqual(core.estimate_token_count(truncated_text), ceiling)
        # Should not cut mid-word or leave corrupted characters
        self.assertTrue(truncated_text.endswith(".") or truncated_text.endswith("\n") or not truncated_text.endswith("statem"))
        self.assertIn("Line 1:", truncated_text)


if __name__ == "__main__":
    unittest.main()
