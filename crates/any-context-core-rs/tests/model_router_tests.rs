use any_context_core_rs::ingestion::model_router::{
    DocumentLayoutExtractor, IngestionModelRouter, VisionExecutionMode,
};
use any_context_core_rs::ingestion::quality_gate::{DocumentAiTarget, QualityGate};
use any_context_core_rs::models::ChunkPayload;
use async_trait::async_trait;

fn make_chunk(id: &str, text: &str, content_type: &str) -> ChunkPayload {
    ChunkPayload {
        id: id.to_string(),
        text: text.to_string(),
        file_name: "doc.pdf".to_string(),
        file_path: "/docs/doc.pdf".to_string(),
        header_path: None,
        start_line: 1,
        end_line: text.lines().count().max(1),
        content_type: content_type.to_string(),
        chunk_index: 0,
    }
}

struct MockCustomExtractor;

#[async_trait]
impl DocumentLayoutExtractor for MockCustomExtractor {
    async fn extract_page(
        &self,
        _file_path: &str,
        page_num: usize,
        content: &str,
    ) -> Result<String, String> {
        Ok(format!(
            "### LayoutLMv3 Custom Extractor (Page {})\nExtracted Data:\n- Content: {}",
            page_num,
            content.trim()
        ))
    }
}

#[tokio::test]
async fn test_model_router_triage_flow() {
    let router = IngestionModelRouter::default();

    let chunks = vec![
        make_chunk("c1", "pub fn calculate_sum(a: i32, b: i32) -> i32 { a + b }", "code"),
        make_chunk("c2", "--------------------------------------------------", "text"),
        make_chunk("c3", "| 1 Sender: Acme Corp | 2 Consignee: Beta Ltd |\n| 3 Gross Weight: 5000 kg | 4 Destination: Malmo |", "pdf"),
        make_chunk("c4", "   \t\n  ", "text"),
    ];

    let (approved, candidates, dropped_count) = router.triage_chunks(chunks);

    assert_eq!(approved.len(), 1, "Only valid code chunk c1 should pass directly");
    assert_eq!(approved[0].id, "c1");

    assert_eq!(candidates.len(), 1, "Dense form c3 should be routed as Document AI candidate");
    assert_eq!(candidates[0].0.id, "c3");

    assert_eq!(dropped_count, 2, "Noise chunks c2 and c4 should be dropped");
}

#[tokio::test]
async fn test_model_router_fallback_spatial_heuristic() {
    let router = IngestionModelRouter::new(QualityGate::default())
        .with_vision_mode(VisionExecutionMode::Disabled);

    let form_content = "\
| Sender: Nordic Logistics AB | Consignee: IKEA Distribution |
| Order No: 991823 | Gross Weight: 12000 kg |";

    let candidate_chunk = make_chunk("cmr_p1", form_content, "pdf");
    let candidates = vec![(
        candidate_chunk,
        DocumentAiTarget::DenseComplexForm { template_ratio: 0.9 },
    )];

    let processed = router.process_document_ai_candidates(candidates, None).await;

    assert!(!processed.is_empty(), "Candidates must be processed by 2D spatial fallback");
    assert!(
        processed[0].text.contains("- **Sender**: Nordic Logistics AB"),
        "Should contain reconstructed Sender field"
    );
    assert!(
        processed[0].text.contains("- **Consignee**: IKEA Distribution"),
        "Should contain reconstructed Consignee field"
    );
}

#[tokio::test]
async fn test_model_router_with_custom_layout_extractor() {
    let router = IngestionModelRouter::new(QualityGate::default())
        .with_layout_extractor(Box::new(MockCustomExtractor));

    let form_content = "Patient Name: Jane Doe\nDiagnosis: Hypertension";
    let candidate_chunk = make_chunk("med_p1", form_content, "pdf");
    let candidates = vec![(
        candidate_chunk,
        DocumentAiTarget::DenseComplexForm { template_ratio: 0.8 },
    )];

    let processed = router.process_document_ai_candidates(candidates, None).await;

    assert_eq!(processed.len(), 1);
    assert!(
        processed[0].text.contains("LayoutLMv3 Custom Extractor"),
        "Should use registered custom layout extractor"
    );
    assert!(
        processed[0].text.contains("Jane Doe"),
        "Should retain original extracted field values"
    );
}

#[tokio::test]
async fn test_model_router_visual_diagram_offline_preservation() {
    let router = IngestionModelRouter::new(QualityGate::default())
        .with_vision_mode(VisionExecutionMode::Disabled);

    let diagram_content = "// Context: arch.png > [Visual Diagram]\n```mermaid\ngraph TD\nA-->B\n```";
    let candidate_chunk = make_chunk("diag_1", diagram_content, "visual_diagram");
    let candidates = vec![(
        candidate_chunk.clone(),
        DocumentAiTarget::VisualDiagram { width: 800, height: 600 },
    )];

    let processed = router.process_document_ai_candidates(candidates, None).await;

    assert_eq!(processed.len(), 1);
    assert_eq!(
        processed[0].text, diagram_content,
        "Offline visual diagram should preserve raw diagram content safely"
    );
}
