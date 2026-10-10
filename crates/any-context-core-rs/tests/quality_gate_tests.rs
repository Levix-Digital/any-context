use any_context_core_rs::ingestion::quality_gate::{
    DocumentAiTarget, QualityDecision, QualityGate, QualityRejectionReason,
};
use any_context_core_rs::models::ChunkPayload;
use std::time::Instant;

fn make_chunk(content: &str, file_name: &str, content_type: &str) -> ChunkPayload {
    ChunkPayload {
        id: "chunk-test-1".to_string(),
        text: content.to_string(),
        file_name: file_name.to_string(),
        file_path: format!("/path/to/{}", file_name),
        header_path: None,
        start_line: 1,
        end_line: content.lines().count().max(1),
        content_type: content_type.to_string(),
        chunk_index: 0,
    }
}

#[test]
fn test_quality_gate_empty_and_whitespace() {
    let gate = QualityGate::default();

    let chunk_empty = make_chunk("", "empty.txt", "text");
    assert_eq!(
        gate.evaluate_chunk(&chunk_empty),
        QualityDecision::Reject {
            reason: QualityRejectionReason::EmptyOrTooShort,
            score: 0.0
        }
    );

    let chunk_ws = make_chunk("   \t\n   \n", "ws.txt", "text");
    assert_eq!(
        gate.evaluate_chunk(&chunk_ws),
        QualityDecision::Reject {
            reason: QualityRejectionReason::EmptyOrTooShort,
            score: 0.0
        }
    );

    let chunk_short = make_chunk("abc", "short.txt", "text");
    assert_eq!(
        gate.evaluate_chunk(&chunk_short),
        QualityDecision::Reject {
            reason: QualityRejectionReason::EmptyOrTooShort,
            score: 0.0
        }
    );
}

#[test]
fn test_quality_gate_low_entropy_noise() {
    let gate = QualityGate::default();

    // Repeated dashes delimiter
    let chunk_dashes = make_chunk("----------------------------------------------------------------", "dashes.txt", "text");
    match gate.evaluate_chunk(&chunk_dashes) {
        QualityDecision::Reject { reason, .. } => {
            assert_eq!(reason, QualityRejectionReason::LowEntropyRepeatedPattern);
        }
        other => panic!("Expected LowEntropyRepeatedPattern, got {:?}", other),
    }

    // Repeated equals sign delimiter
    let chunk_equals = make_chunk("================================================================", "equals.txt", "text");
    match gate.evaluate_chunk(&chunk_equals) {
        QualityDecision::Reject { reason, .. } => {
            assert_eq!(reason, QualityRejectionReason::LowEntropyRepeatedPattern);
        }
        other => panic!("Expected LowEntropyRepeatedPattern, got {:?}", other),
    }

    // Repeated asterisks
    let chunk_stars = make_chunk("****************************************************************", "stars.txt", "text");
    match gate.evaluate_chunk(&chunk_stars) {
        QualityDecision::Reject { reason, .. } => {
            assert_eq!(reason, QualityRejectionReason::LowEntropyRepeatedPattern);
        }
        other => panic!("Expected LowEntropyRepeatedPattern, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_degenerate_code_fragments() {
    let gate = QualityGate::default();

    let fragments = vec!["}", "};", "  }\n", "];", "]", "{", ")", ")};"];
    for frag in fragments {
        let chunk = make_chunk(frag, "frag.rs", "code");
        match gate.evaluate_chunk(&chunk) {
            QualityDecision::Reject { reason, .. } => {
                assert!(
                    reason == QualityRejectionReason::DegenerateSyntaxFragment
                        || reason == QualityRejectionReason::EmptyOrTooShort,
                    "Expected DegenerateSyntaxFragment or EmptyOrTooShort for '{}', got {:?}",
                    frag,
                    reason
                );
            }
            other => panic!("Expected rejection for fragment '{}', got {:?}", frag, other),
        }
    }
}

#[test]
fn test_quality_gate_minified_or_obfuscated_blob() {
    let gate = QualityGate::default();

    // Create a 2500 character single line
    let minified_line = "var a=1,b=2,c=3;".repeat(160);
    assert!(minified_line.len() > 2000);

    let chunk = make_chunk(&minified_line, "bundle.min.js", "code");
    match gate.evaluate_chunk(&chunk) {
        QualityDecision::Reject { reason, .. } => {
            assert_eq!(reason, QualityRejectionReason::MinifiedOrCompressedLine);
        }
        other => panic!("Expected MinifiedOrCompressedLine, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_binary_or_garbage() {
    let gate = QualityGate::default();

    // Text with binary control bytes
    let garbage: String = (0..50).map(|i| (i % 8) as u8 as char).collect();
    let garbage_text = format!("Some normal text followed by binary junk: {}", garbage);

    let chunk = make_chunk(&garbage_text, "corrupt.bin", "text");
    match gate.evaluate_chunk(&chunk) {
        QualityDecision::Reject { reason, .. } => {
            assert_eq!(reason, QualityRejectionReason::HighEntropyBinaryNoise);
        }
        other => panic!("Expected HighEntropyBinaryNoise, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_scanned_page_or_diagram_detection() {
    let gate = QualityGate::default();

    // Scanned page marker (pdf_scan content_type)
    let scanned_content = "[Scanned Page: contains image-based or rasterized content]\nImage reference: page_001.png";
    let chunk_scanned = make_chunk(scanned_content, "doc.pdf", "pdf_scan");
    assert_eq!(
        gate.evaluate_chunk(&chunk_scanned),
        QualityDecision::NeedsDocumentAi {
            target: DocumentAiTarget::ScannedPdfPage { page_num: 1 }
        }
    );

    // Visual diagram with resolution header
    let diagram_content = "// Context: system_arch.png > [Visual Diagram]\n[Resolution: 1920x1080 px]\n```mermaid\ngraph TD\nA[Client] --> B[API Gateway]\n```";
    let chunk_diagram = make_chunk(diagram_content, "arch.png", "visual_diagram");
    assert_eq!(
        gate.evaluate_chunk(&chunk_diagram),
        QualityDecision::NeedsDocumentAi {
            target: DocumentAiTarget::VisualDiagram { width: 1920, height: 1080 }
        }
    );
}

#[test]
fn test_quality_gate_markdown_tables_pass_with_high_score() {
    let gate = QualityGate::default();

    // High-fidelity structured Markdown table from 2D spatial reconstruction
    let table_content = "\
| Header 1 | Header 2 | Header 3 |
| --- | --- | --- |
| Row 1 Col 1 | Row 1 Col 2 | Row 1 Col 3 |
| Row 2 Col 1 | Row 2 Col 2 | Row 2 Col 3 |
| Row 3 Col 1 | Row 3 Col 2 | Row 3 Col 3 |";

    let chunk = make_chunk(table_content, "report.pdf", "pdf");
    match gate.evaluate_chunk(&chunk) {
        QualityDecision::Pass { score } => {
            assert!(score > 0.6, "Well-formed Markdown table should pass with high score: {}", score);
        }
        other => panic!("Expected Pass for well-formed markdown table, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_disjoint_dense_form_detection() {
    let gate = QualityGate::default();

    // Dense disjoint form without structured Markdown header delimiters
    let disjoint_form = "\
| IKEA Distribution | Calgary Terminal | Dock 4B |
| Delivery Note: 8819 | Order Ref: 9942 | Carrier: Bison |
| Status: In Transit | Driver ID: D-991 | Seal: S-1102 |
| Tare: 12000kg | Gross: 24000kg | Net: 12000kg |";

    let chunk = make_chunk(disjoint_form, "cmr_raw.pdf", "pdf");
    match gate.evaluate_chunk(&chunk) {
        QualityDecision::NeedsDocumentAi { target } => match target {
            DocumentAiTarget::DenseComplexForm { template_ratio } => {
                assert!(template_ratio > 0.5, "Should have high template ratio: {}", template_ratio);
            }
            other => panic!("Expected DenseComplexForm, got {:?}", other),
        },
        other => panic!("Expected NeedsDocumentAi for disjoint dense form in pdf, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_legitimate_code_and_docs_pass() {
    let gate = QualityGate::default();

    // Legitimate Rust Code
    let rust_code = r#"
pub struct UserProfile {
    pub id: u64,
    pub username: String,
    pub email: String,
}

impl UserProfile {
    pub fn new(id: u64, username: &str, email: &str) -> Self {
        Self {
            id,
            username: username.to_string(),
            email: email.to_string(),
        }
    }
}
"#;
    let chunk_rs = make_chunk(rust_code, "user.rs", "code");
    match gate.evaluate_chunk(&chunk_rs) {
        QualityDecision::Pass { score } => {
            assert!(score > 0.5, "Legitimate Rust code should receive high quality score: {}", score);
        }
        other => panic!("Expected Pass for valid Rust code, got {:?}", other),
    }

    // Legitimate Python Code
    let py_code = r#"
import asyncio
from typing import List

async def fetch_all_urls(urls: List[str]) -> List[str]:
    tasks = [fetch_single(url) for url in urls]
    return await asyncio.gather(*tasks)
"#;
    let chunk_py = make_chunk(py_code, "crawler.py", "code");
    match gate.evaluate_chunk(&chunk_py) {
        QualityDecision::Pass { score } => {
            assert!(score > 0.5, "Legitimate Python code should pass: {}", score);
        }
        other => panic!("Expected Pass for valid Python code, got {:?}", other),
    }

    // Legitimate Markdown Documentation
    let md_text = r#"
# Ingestion Architecture Overview

The AnyContext native ingestion pipeline employs an 8-step Software Engineering Lifecycle Protocol.
It guarantees deterministic sub-millisecond quality gate checks, 2D spatial layout extraction for forms,
and seamless integration with LanceDB and SQLite.
"#;
    let chunk_md = make_chunk(md_text, "README.md", "markdown");
    match gate.evaluate_chunk(&chunk_md) {
        QualityDecision::Pass { score } => {
            assert!(score > 0.5, "Legitimate Markdown doc should pass: {}", score);
        }
        other => panic!("Expected Pass for valid Markdown, got {:?}", other),
    }
}

#[test]
fn test_quality_gate_sub_millisecond_performance() {
    let gate = QualityGate::default();
    let samples = vec![
        make_chunk("fn test() { let x = 10; println!(\"{}\", x); }", "test.rs", "code"),
        make_chunk("--------------------------------------------------", "dashes.txt", "text"),
        make_chunk("def add(a, b): return a + b", "math.py", "code"),
        make_chunk("   \n\t  ", "ws.txt", "text"),
        make_chunk("| Col1 | Col2 |\n|---|---|\n| A | B |\n| C | D |", "table.md", "markdown"),
    ];

    let iterations = 2_000;
    let start = Instant::now();
    for i in 0..iterations {
        let chunk = &samples[i % samples.len()];
        let _ = gate.evaluate_chunk(chunk);
    }
    let elapsed = start.elapsed();
    let avg_micros = elapsed.as_micros() as f64 / iterations as f64;

    println!(
        "QualityGate Performance: {} iterations in {:?} (avg: {:.2} µs / evaluation)",
        iterations, elapsed, avg_micros
    );

    // Strict performance SLA: Must be strictly under 100 microseconds (0.1ms) per chunk
    assert!(
        avg_micros < 100.0,
        "Quality gate evaluation took {:.2} µs, exceeding 100 µs (0.1ms) SLA",
        avg_micros
    );
}
