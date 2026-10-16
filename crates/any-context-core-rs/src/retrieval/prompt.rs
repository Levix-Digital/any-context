//! Canonical Context Formatter, U-Shaped Reordering & Structured XML Prompt Engine.
//!
//! Implements strict Hexagonal Architecture for RAG prompt generation:
//! - U-Shaped Reordering to eliminate the "Lost in the Middle" Transformer attention degradation.
//! - XML Tag Boundaries (<chunk id="..." scope="..." version="...">) preventing cross-attention fusion.
//! - Zero Hallucination Gate providing instant (<5ms / $0.00) calibrated negative responses when context is absent.
//! - Unified canonical formatting for both Deterministic 1-Hop RAG and Agentic Deep Search tool results.

use crate::retrieval::pipeline::HybridSearchResult;

/// Core context formatter for deterministic RAG and agentic tool results.
pub struct ContextFormatter;

impl ContextFormatter {
    /// Reorders a list of items from best-to-worst into a U-Shaped distribution:
    ///
    /// Input (sorted by score):  `[R1, R2, R3, R4, R5, R6]`
    /// Output (U-Shaped curve):  `[R1, R3, R5, R6, R4, R2]`
    ///
    /// - Rank 1 sits at the very beginning (Primacy Bias).
    /// - Rank 2 sits at the very end, directly adjacent to the user query (Recency Bias).
    /// - Lower-ranked distractors are pushed into the middle valley of least attention.
    pub fn u_shaped_reorder<T: Clone>(items: &[T]) -> Vec<T> {
        let len = items.len();
        if len <= 2 {
            return items.to_vec();
        }

        let mut top_left = Vec::new();
        let mut bottom_right = Vec::new();

        for (idx, item) in items.iter().enumerate() {
            if idx % 2 == 0 {
                // Odd positions in 1-based index (0, 2, 4...): R1, R3, R5...
                top_left.push(item.clone());
            } else {
                // Even positions in 1-based index (1, 3, 5...): R2, R4, R6...
                bottom_right.push(item.clone());
            }
        }

        // Invert bottom_right so R2 is at the very end
        bottom_right.reverse();

        top_left.extend(bottom_right);
        top_left
    }

    /// Infers clean logical scope and version tags from file path and name.
    pub fn infer_scope_and_version(file_path: &str, file_name: &str) -> (String, String) {
        let path_lower = file_path.to_lowercase();
        let name_lower = file_name.to_lowercase();
        let combined = format!("{}/{}", path_lower, name_lower);

        // Version heuristic
        let version = if combined.contains("v2") || combined.contains("v2.") || combined.contains("v2_") {
            "v2".to_string()
        } else if combined.contains("v1") || combined.contains("v1.") || combined.contains("v1_") {
            "v1".to_string()
        } else if combined.contains("v3") {
            "v3".to_string()
        } else if combined.contains("legacy") || combined.contains("deprecated") {
            "legacy".to_string()
        } else {
            "current".to_string()
        };

        // Scope heuristic based on directory hints
        let scope = if combined.contains("payment") || combined.contains("billing") || combined.contains("stripe") {
            "billing".to_string()
        } else if combined.contains("auth") || combined.contains("security") || combined.contains("jwt") || combined.contains("oauth") {
            "auth".to_string()
        } else if combined.contains("config") || combined.contains("settings") || combined.contains("env") {
            "config".to_string()
        } else if combined.contains("api") || combined.contains("routes") || combined.contains("endpoints") {
            "api".to_string()
        } else if combined.contains("db") || combined.contains("database") || combined.contains("schema") || combined.contains("migration") {
            "database".to_string()
        } else if combined.contains("webhook") || combined.contains("queue") || combined.contains("worker") {
            "worker".to_string()
        } else if combined.contains("test") || combined.contains("spec") {
            "test".to_string()
        } else {
            // Default to parent folder or filename stem
            std::path::Path::new(file_path)
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("general")
                .to_string()
        };

        (scope, version)
    }

    /// Envelopes chunks into structured XML tags with identification and scope metadata.
    pub fn format_xml_chunks(results: &[HybridSearchResult]) -> String {
        if results.is_empty() {
            return "<retrieved_context>\n  <!-- No relevant chunks retrieved -->\n</retrieved_context>".to_string();
        }

        // Apply U-Shaped Reordering
        let reordered = Self::u_shaped_reorder(results);
        let mut out = String::new();
        out.push_str("<retrieved_context>\n");

        for (display_rank, chunk) in reordered.iter().enumerate() {
            let (scope, version) = Self::infer_scope_and_version(&chunk.file_path, &chunk.file_name);
            let pos_label = match display_rank {
                0 => "POSITION 1 (HIGH-ATTENTION ZONE - PRIMACY)",
                r if r == reordered.len() - 1 => "LAST POSITION (HIGH-ATTENTION ZONE - RECENCY)",
                _ => "MIDDLE POSITION",
            };

            let clean_content_type = if chunk.content_type.is_empty() {
                "Document"
            } else {
                &chunk.content_type
            };

            let mod_info = match &chunk.last_modified {
                Some(lm) if !lm.is_empty() => format!(" | Modified: {}", lm),
                _ => String::new(),
            };

            let chunk_id_attr = if chunk.chunk_id.is_empty() {
                format!("chk_{}", display_rank + 1)
            } else {
                chunk.chunk_id.clone()
            };

            out.push_str(&format!(
                "  <!-- {} -->\n",
                pos_label
            ));
            out.push_str(&format!(
                "  <chunk id=\"{}\" rank=\"{}\" score=\"{:.2}\" source=\"{}\" file=\"{}\" scope=\"{}\" version=\"{}\" type=\"{}\">\n",
                chunk_id_attr,
                display_rank + 1,
                chunk.score,
                chunk.file_path,
                chunk.file_name,
                scope,
                version,
                clean_content_type
            ));
            out.push_str(&format!(
                "    [Context: {} | Type: {}{}]\n",
                chunk.file_name,
                clean_content_type,
                mod_info
            ));

            // Indent chunk content for clean XML readability
            for line in chunk.text.lines() {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }

            out.push_str("  </chunk>\n\n");
        }

        out.push_str("</retrieved_context>");
        out
    }

    /// Assembles the complete prompt payload for 1-hop Deterministic RAG.
    pub fn format_deterministic_prompt(
        chunks: &[HybridSearchResult],
        query: &str,
        system_prompt: &str,
        workspace: &str,
    ) -> String {
        let xml_context = Self::format_xml_chunks(chunks);

        format!(
r#"<system_instructions>
{}

STRICT CONTEXT ISOLATION RULES (ANTI-HALLUCINATION):
1. Each <chunk> tag is a self-contained excerpt with "scope", "version" and "source" attributes.
2. NEVER mix numbers, timeouts, ports or rules from two <chunk> blocks whose "scope" or "version" differ.
3. Answer strictly from the documentary evidence in <retrieved_context>.
4. Cite the source files or IDs in your answer (e.g. docs/config.toml or [chk_1]).
5. If the exact answer is not in the right scope, state precisely that the information was not found in the workspace '{workspace}'.
6. Answer in the language of the user's question.
</system_instructions>

{}

<user_query>
{}
</user_query>"#,
            system_prompt.trim(),
            xml_context,
            query.trim()
        )
    }

    /// Formats the payload returned by the `search_db` tool in Agentic Deep Search.
    ///
    /// Combines U-Shaped XML chunks with an explicit `<query_anchor>` at the end to re-anchor
    /// the model's attention in multi-turn reflective reasoning.
    pub fn format_tool_result_payload(chunks: &[HybridSearchResult], query: &str) -> String {
        let xml_context = Self::format_xml_chunks(chunks);

        format!(
r#"<tool_result name="search_db">
{}

  <query_anchor>
    Original query reminder: "{}"
    Instruction: Synthesize the results keeping version and scope attributes strictly separate, citing the corresponding source files.
  </query_anchor>
</tool_result>"#,
            xml_context,
            query.trim()
        )
    }

    /// Zero Hallucination Gate: Evaluates if retrieval returned viable chunks.
    /// If no chunks are found or scores are completely negligible, returns a calibrated,
    /// honest negative response without making unnecessary LLM calls.
    pub fn evaluate_zero_hallucination(
        chunks: &[HybridSearchResult],
        workspace: &str,
        monitored_folders: &[String],
        monitored_urls: &[String],
        query: &str,
    ) -> Option<String> {
        let has_viable_chunks = !chunks.is_empty() && chunks.iter().any(|c| c.score >= 0.005);

        if has_viable_chunks {
            return None;
        }

        let mut sources_desc = Vec::new();
        if !monitored_folders.is_empty() {
            sources_desc.push(format!("local folders: [{}]", monitored_folders.join(", ")));
        }
        if !monitored_urls.is_empty() {
            sources_desc.push(format!("web sources: [{}]", monitored_urls.join(", ")));
        }

        let monitored_summary = if !sources_desc.is_empty() {
            format!(" The workspace monitors {}.", sources_desc.join(" and "))
        } else {
            String::new()
        };

        Some(format!(
            "No information or documents matching the query '{}' were found in the workspace '{}'.{}\n\n\
            💡 Suggestions:\n\
            • Index more folders or files with `/folder <path>` or `actx --sync`.\n\
            • Turn on web search with `/web-search on` to consult external sources.\n\
            • Try rephrasing the question with other terms or file names.",
            query.trim(),
            workspace,
            monitored_summary
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_chunk(id: &str, score: f64, text: &str) -> HybridSearchResult {
        HybridSearchResult {
            chunk_id: id.to_string(),
            file_name: "test.md".to_string(),
            file_path: "docs/test.md".to_string(),
            workspace: "Default".to_string(),
            text: text.to_string(),
            content_type: "Markdown".to_string(),
            content_hash: "hash".to_string(),
            score,
            dense_score: None,
            sparse_score: None,
            token_count: 50,
            matched_subqueries: Vec::new(),
            last_modified: None,
        }
    }

    #[test]
    fn test_u_shaped_reorder_empty_and_short() {
        let empty: Vec<i32> = vec![];
        assert_eq!(ContextFormatter::u_shaped_reorder(&empty), Vec::<i32>::new());

        let one = vec![1];
        assert_eq!(ContextFormatter::u_shaped_reorder(&one), vec![1]);

        let two = vec![1, 2];
        assert_eq!(ContextFormatter::u_shaped_reorder(&two), vec![1, 2]);
    }

    #[test]
    fn test_u_shaped_reorder_even_length() {
        // Input: [1, 2, 3, 4, 5, 6]
        // Odds (0, 2, 4): 1, 3, 5
        // Evens (1, 3, 5): 2, 4, 6 -> reversed: 6, 4, 2
        // Output: [1, 3, 5, 6, 4, 2]
        let input = vec![1, 2, 3, 4, 5, 6];
        let expected = vec![1, 3, 5, 6, 4, 2];
        assert_eq!(ContextFormatter::u_shaped_reorder(&input), expected);
    }

    #[test]
    fn test_u_shaped_reorder_odd_length() {
        // Input: [1, 2, 3, 4, 5]
        // Odds (0, 2, 4): 1, 3, 5
        // Evens (1, 3): 2, 4 -> reversed: 4, 2
        // Output: [1, 3, 5, 4, 2]
        let input = vec![1, 2, 3, 4, 5];
        let expected = vec![1, 3, 5, 4, 2];
        assert_eq!(ContextFormatter::u_shaped_reorder(&input), expected);
    }

    #[test]
    fn test_infer_scope_and_version() {
        let (scope1, ver1) = ContextFormatter::infer_scope_and_version("src/billing/payment_v2.rs", "payment_v2.rs");
        assert_eq!(scope1, "billing");
        assert_eq!(ver1, "v2");

        let (scope2, ver2) = ContextFormatter::infer_scope_and_version("legacy/auth_v1.md", "auth_v1.md");
        assert_eq!(scope2, "auth");
        assert_eq!(ver2, "v1");

        let (scope3, ver3) = ContextFormatter::infer_scope_and_version("docs/architecture.md", "architecture.md");
        assert_eq!(scope3, "docs");
        assert_eq!(ver3, "current");
    }

    #[test]
    fn test_format_xml_chunks_structure() {
        let chunks = vec![
            dummy_chunk("chk_1", 0.95, "Primeira regra da v2"),
            dummy_chunk("chk_2", 0.90, "Segunda regra da v2"),
            dummy_chunk("chk_3", 0.85, "Terceira regra da v2"),
            dummy_chunk("chk_4", 0.80, "Quarta regra"),
        ];

        let xml = ContextFormatter::format_xml_chunks(&chunks);
        assert!(xml.starts_with("<retrieved_context>"));
        assert!(xml.ends_with("</retrieved_context>"));
        assert!(xml.contains("<chunk id=\"chk_1\""));
        assert!(xml.contains("<chunk id=\"chk_2\""));
        assert!(xml.contains("POSITION 1 (HIGH-ATTENTION ZONE - PRIMACY)"));
        assert!(xml.contains("LAST POSITION (HIGH-ATTENTION ZONE - RECENCY)"));
    }

    #[test]
    fn test_format_deterministic_prompt() {
        let chunks = vec![dummy_chunk("chk_1", 0.95, "Timeout é de 30s")];
        let prompt = ContextFormatter::format_deterministic_prompt(
            &chunks,
            "Qual o timeout?",
            "Você é o assistente.",
            "Work",
        );

        assert!(prompt.contains("<system_instructions>"));
        assert!(prompt.contains("<retrieved_context>"));
        assert!(prompt.contains("<user_query>\nQual o timeout?\n</user_query>"));
        assert!(prompt.contains("Timeout é de 30s"));
    }

    #[test]
    fn test_format_tool_result_payload() {
        let chunks = vec![dummy_chunk("chk_1", 0.95, "Configuração de banco")];
        let payload = ContextFormatter::format_tool_result_payload(&chunks, "onde fica o banco?");

        assert!(payload.contains("<tool_result name=\"search_db\">"));
        assert!(payload.contains("<query_anchor>"));
        assert!(payload.contains("onde fica o banco?"));
        assert!(payload.contains("</tool_result>"));
    }

    #[test]
    fn test_evaluate_zero_hallucination() {
        let folders = vec!["C:\\docs".to_string()];
        let urls = vec![];

        // 1. Empty chunks
        let res1 = ContextFormatter::evaluate_zero_hallucination(&[], "Finance", &folders, &urls, "qual o saldo?");
        assert!(res1.is_some());
        let msg = res1.unwrap();
        assert!(msg.contains("Finance"));
        assert!(msg.contains("C:\\docs"));

        // 2. Chunks below minimum score threshold
        let low_score_chunks = vec![dummy_chunk("chk_0", 0.001, "Irrelevant junk")];
        let res2 = ContextFormatter::evaluate_zero_hallucination(&low_score_chunks, "Finance", &folders, &urls, "qual o saldo?");
        assert!(res2.is_some());

        // 3. Viable chunk
        let good_chunks = vec![dummy_chunk("chk_1", 0.85, "Saldo disponível")];
        let res3 = ContextFormatter::evaluate_zero_hallucination(&good_chunks, "Finance", &folders, &urls, "qual o saldo?");
        assert!(res3.is_none());
    }
}
