use any_context_core_rs::ingestion::chunkers::pdf::TextSpan;
use any_context_core_rs::ingestion::chunkers::spatial_form::SpatialFormChunker;

#[test]
fn test_spatial_form_healthcare_prescription() {
    let chunker = SpatialFormChunker::default();
    let medical_doc = "\
Hospital: Metropolitan Medical Center
Patient Name: John Doe
Date of Birth: 1980-05-12
Medical Record Number: MRN-88492
Prescribed Medication: Amoxicillin 500mg
Dosage: 1 capsule orally every 8 hours
Prescribing Physician: Dr. Maria Silva
License Number: MD-449102
";

    let fields = chunker.extract_fields_from_dense_text(medical_doc);
    assert!(!fields.is_empty(), "Should extract fields from medical record");

    let labels: Vec<&str> = fields.iter().map(|f| f.label.as_str()).collect();
    assert!(labels.contains(&"Patient Name"));
    assert!(labels.contains(&"Date of Birth"));
    assert!(labels.contains(&"Prescribed Medication"));
    assert!(labels.contains(&"Prescribing Physician"));

    let patient_field = fields.iter().find(|f| f.label == "Patient Name").unwrap();
    assert_eq!(patient_field.value, "John Doe");

    let med_field = fields.iter().find(|f| f.label == "Prescribed Medication").unwrap();
    assert_eq!(med_field.value, "Amoxicillin 500mg");

    // Test formatted chunk generation
    let chunks = chunker.chunk_dense_text("prescription.pdf", "/docs/prescription.pdf", 1, medical_doc);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("- **Patient Name**: John Doe"));
    assert!(chunks[0].text.contains("- **Prescribed Medication**: Amoxicillin 500mg"));
}

#[test]
fn test_spatial_form_legal_court_filing() {
    let chunker = SpatialFormChunker::default();
    let legal_doc = "\
Court: United States District Court
Case Number: 2026-CV-8891
Filing Date: 2026-03-15
Plaintiff: Acme Global Technologies LLC
Defendant: Stark Dynamics International
Presiding Judge: Hon. Sarah Jenkins
Docket Status: Motion for Summary Judgment Pending
";

    let fields = chunker.extract_fields_from_dense_text(legal_doc);
    let case_num = fields.iter().find(|f| f.label == "Case Number").expect("Case Number field");
    assert_eq!(case_num.value, "2026-CV-8891");

    let plaintiff = fields.iter().find(|f| f.label == "Plaintiff").expect("Plaintiff field");
    assert_eq!(plaintiff.value, "Acme Global Technologies LLC");

    let defendant = fields.iter().find(|f| f.label == "Defendant").expect("Defendant field");
    assert_eq!(defendant.value, "Stark Dynamics International");
}

#[test]
fn test_spatial_form_military_defense_manifest() {
    let chunker = SpatialFormChunker::default();
    let defense_doc = "\
Unit Command: 3rd Battalion Echo Company
Asset Tag: MIL-SPEC-9021
Serial Code: SEC-8891-B
Equipment Category: Tactical Communication Array
Readiness Condition: Operational Tier 1
Assigned Officer: Cpt. Vance Reynolds
Deployment Zone: Sector 4-G
";

    let fields = chunker.extract_fields_from_dense_text(defense_doc);
    let asset = fields.iter().find(|f| f.label == "Asset Tag").expect("Asset Tag field");
    assert_eq!(asset.value, "MIL-SPEC-9021");

    let status = fields.iter().find(|f| f.label == "Readiness Condition").expect("Readiness field");
    assert_eq!(status.value, "Operational Tier 1");
}

#[test]
fn test_spatial_form_accounting_financial_invoice() {
    let chunker = SpatialFormChunker::default();
    let invoice_doc = "\
Invoice Number: INV-2026-0042
Issue Date: 2026-04-01
Due Date: 2026-05-01
Customer ID: CUST-7719
Subtotal Amount: $14,250.00
Sales Tax: $1,425.00
Grand Total Due: $15,675.00
Payment Terms: Net 30
";

    let fields = chunker.extract_fields_from_dense_text(invoice_doc);
    let inv_num = fields.iter().find(|f| f.label == "Invoice Number").expect("Invoice Number");
    assert_eq!(inv_num.value, "INV-2026-0042");

    let total = fields.iter().find(|f| f.label == "Grand Total Due").expect("Grand Total");
    assert_eq!(total.value, "$15,675.00");
}

#[test]
fn test_spatial_form_ikea_logistics_cmr_waybill() {
    let chunker = SpatialFormChunker::default();
    // Typical fragmented CMR Waybill structure
    let cmr_doc = "\
| 1 Sender: Nordic Freight Lines AB | 2 Consignee: IKEA Distribution Center Almhult |
| 3 Place of Delivery: Almhult Warehouse 4 | 4 Place and date: Malmo 2026-02-18 |
| 16 Carrier: Scandinavian Logistics Express | 17 Successive Carriers: None |
| 24 Gross Weight: 14500 kg | 25 Volume in m3: 48.5 |
";

    let fields = chunker.extract_fields_from_dense_text(cmr_doc);
    assert!(!fields.is_empty(), "Should extract fields from fragmented CMR table");

    let sender = fields.iter().find(|f| f.label.contains("Sender")).expect("Sender field");
    assert_eq!(sender.value, "Nordic Freight Lines AB");

    let consignee = fields.iter().find(|f| f.label.contains("Consignee")).expect("Consignee field");
    assert_eq!(consignee.value, "IKEA Distribution Center Almhult");

    let weight = fields.iter().find(|f| f.label.contains("Gross Weight")).expect("Gross Weight field");
    assert_eq!(weight.value, "14500 kg");
}

#[test]
fn test_spatial_form_markdown_table_key_value_pairs() {
    let chunker = SpatialFormChunker::default();
    let table_doc = "\
| Property | Value |
| --- | --- |
| System Architecture | Hexagonal Clean Code |
| Target Latency | Sub-10ms |
| Supported Format | Multi-Domain PDF and Forms |
| Security Mode | Zero Cloud / Low Spec |
";

    let fields = chunker.extract_fields_from_dense_text(table_doc);
    assert_eq!(fields.len(), 4);

    assert_eq!(fields[0].label, "System Architecture");
    assert_eq!(fields[0].value, "Hexagonal Clean Code");

    assert_eq!(fields[1].label, "Target Latency");
    assert_eq!(fields[1].value, "Sub-10ms");

    assert_eq!(fields[2].label, "Supported Format");
    assert_eq!(fields[2].value, "Multi-Domain PDF and Forms");

    assert_eq!(fields[3].label, "Security Mode");
    assert_eq!(fields[3].value, "Zero Cloud / Low Spec");
}

#[test]
fn test_spatial_form_2d_coordinate_voronoi_pairing() {
    let chunker = SpatialFormChunker::default();

    // Simulate 2D TextSpans:
    // Span 0: Label "PATIENT ID:" at (x=20.0, y=100.0, w=70.0, h=10.0)
    // Span 1: Value "PID-99214" at (x=95.0, y=100.0, w=60.0, h=10.0) -> Horizontal right neighbor
    // Span 2: Label "NOTES:" at (x=20.0, y=130.0, w=50.0, h=10.0)
    // Span 3: Value "Patient tolerates therapy well" at (x=20.0, y=145.0, w=200.0, h=10.0) -> Vertical bottom neighbor
    let spans = vec![
        TextSpan {
            text: "PATIENT ID:".to_string(),
            x: 20.0,
            y: 100.0,
            width: 70.0,
            font_size: 10.0,
        },
        TextSpan {
            text: "PID-99214".to_string(),
            x: 95.0,
            y: 100.0,
            width: 60.0,
            font_size: 10.0,
        },
        TextSpan {
            text: "NOTES:".to_string(),
            x: 20.0,
            y: 130.0,
            width: 50.0,
            font_size: 10.0,
        },
        TextSpan {
            text: "Patient tolerates therapy well".to_string(),
            x: 20.0,
            y: 145.0,
            width: 200.0,
            font_size: 10.0,
        },
    ];

    let fields = chunker.extract_fields_from_spans(&spans);
    assert_eq!(fields.len(), 2, "Both label-value pairs should be discovered");

    let pid_field = fields.iter().find(|f| f.label == "PATIENT ID").expect("PATIENT ID pair");
    assert_eq!(pid_field.value, "PID-99214");

    let notes_field = fields.iter().find(|f| f.label == "NOTES").expect("NOTES pair");
    assert_eq!(notes_field.value, "Patient tolerates therapy well");

    // Verify chunk generation from spans
    let chunks = chunker.chunk_spans("/path/doc.pdf", 1, &spans);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("- **PATIENT ID**: PID-99214"));
    assert!(chunks[0].text.contains("- **NOTES**: Patient tolerates therapy well"));
}

#[test]
fn test_spatial_form_does_not_invert_arbitrary_table_columns() {
    let chunker = SpatialFormChunker::default();

    // 2-column table with tax box numbers and financial values
    let t4_doc = "\
| Box 14 Employment income | Box 22 Income tax deducted |
| --- | --- |
| 58792.60 | 7976.60 |
";

    let fields = chunker.extract_fields_from_dense_text(t4_doc);
    // Should NOT invert 58792.60 as key and 7976.60 as value!
    assert!(fields.is_empty(), "Arbitrary multi-column tables must not be parsed into fake key-value pairs");

    // Formatted chunk generation preserves the table cleanly
    let chunks = chunker.chunk_dense_text("t4.pdf", "/docs/t4.pdf", 1, t4_doc);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("58792.60"));
    assert!(chunks[0].text.contains("7976.60"));
    assert!(!chunks[0].text.contains("- **58792.60**: 7976.60"));
}
