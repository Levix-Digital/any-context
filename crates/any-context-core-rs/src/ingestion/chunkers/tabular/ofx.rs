use std::path::Path;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct OfxChunker {
    pub max_chunk_chars: usize,
}

#[derive(Debug, Default, Clone)]
struct OfxAccountMetadata {
    bank_id: String,
    acct_id: String,
    acct_type: String,
    currency: String,
    balance: String,
    dt_start: String,
    dt_end: String,
}

#[derive(Debug, Default, Clone)]
struct OfxTransaction {
    trn_type: String,
    dt_posted: String,
    amount: String,
    fit_id: String,
    check_num: String,
    memo: String,
}

impl OfxChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self { max_chunk_chars }
    }

    /// Helper to extract tag value handling both SGML (<TAG>value\n) and XML (<TAG>value</TAG>).
    fn extract_tag_value(block: &str, tag_name: &str) -> Option<String> {
        let open_tag = format!("<{}>", tag_name);
        let open_tag_upper = format!("<{}>", tag_name.to_uppercase());

        let idx = if let Some(p) = block.find(&open_tag) {
            p + open_tag.len()
        } else if let Some(p) = block.find(&open_tag_upper) {
            p + open_tag_upper.len()
        } else {
            let lower_block = block.to_lowercase();
            let lower_tag = open_tag.to_lowercase();
            let p = lower_block.find(&lower_tag)?;
            p + lower_tag.len()
        };

        let rest = &block[idx..];
        // Stop at next tag start '<' or end of line '\r' or '\n'
        let end_idx = rest.find(['<', '\r', '\n']).unwrap_or(rest.len());
        let val = rest[..end_idx].trim();
        if val.is_empty() {
            None
        } else {
            Some(val.to_string())
        }
    }

    /// Formats OFX raw date (e.g. 20260901120000[-3:BRT]) into clean ISO date (YYYY-MM-DD).
    fn format_date(raw_date: &str) -> String {
        let digits: String = raw_date.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 8 {
            format!("{}-{}-{}", &digits[0..4], &digits[4..6], &digits[6..8])
        } else if !raw_date.is_empty() {
            raw_date.to_string()
        } else {
            "N/A".to_string()
        }
    }

    fn parse_account_metadata(content: &str) -> OfxAccountMetadata {
        let mut meta = OfxAccountMetadata::default();
        if let Some(v) = Self::extract_tag_value(content, "BANKID") {
            meta.bank_id = v;
        }
        if let Some(v) = Self::extract_tag_value(content, "ACCTID") {
            meta.acct_id = v;
        }
        if let Some(v) = Self::extract_tag_value(content, "ACCTTYPE") {
            meta.acct_type = v;
        }
        if let Some(v) = Self::extract_tag_value(content, "CURDEF") {
            meta.currency = v;
        }
        if let Some(v) = Self::extract_tag_value(content, "BALAMT") {
            meta.balance = v;
        }
        if let Some(v) = Self::extract_tag_value(content, "DTSTART") {
            meta.dt_start = Self::format_date(&v);
        }
        if let Some(v) = Self::extract_tag_value(content, "DTEND") {
            meta.dt_end = Self::format_date(&v);
        }
        meta
    }

    fn parse_transactions(content: &str) -> Vec<OfxTransaction> {
        let mut txs = Vec::new();
        let upper_content = content.to_uppercase();
        let mut start_search = 0;

        while let Some(trn_start) = upper_content[start_search..].find("<STMTTRN>") {
            let actual_start = start_search + trn_start;
            let after_open = actual_start + 9;

            // Find either </STMTTRN> or next <STMTTRN> or </BANKTRANLIST>
            let end_offset = upper_content[after_open..]
                .find("</STMTTRN>")
                .map(|p| p + 10)
                .or_else(|| upper_content[after_open..].find("<STMTTRN>"))
                .or_else(|| upper_content[after_open..].find("</BANKTRANLIST>"))
                .unwrap_or_else(|| upper_content.len() - after_open);

            let trn_block = &content[actual_start..after_open + end_offset];
            start_search = after_open + end_offset;

            let mut tx = OfxTransaction::default();
            if let Some(v) = Self::extract_tag_value(trn_block, "TRNTYPE") {
                tx.trn_type = v;
            }
            if let Some(v) = Self::extract_tag_value(trn_block, "DTPOSTED") {
                tx.dt_posted = Self::format_date(&v);
            }
            if let Some(v) = Self::extract_tag_value(trn_block, "TRNAMT") {
                tx.amount = v;
            }
            if let Some(v) = Self::extract_tag_value(trn_block, "FITID") {
                tx.fit_id = v;
            }
            if let Some(v) = Self::extract_tag_value(trn_block, "CHECKNUM") {
                tx.check_num = v;
            }
            if let Some(v) = Self::extract_tag_value(trn_block, "MEMO") {
                tx.memo = v.replace('|', "\\|").replace('\n', " ");
            } else if let Some(v) = Self::extract_tag_value(trn_block, "NAME") {
                tx.memo = v.replace('|', "\\|").replace('\n', " ");
            }

            if !tx.dt_posted.is_empty() || !tx.amount.is_empty() || !tx.memo.is_empty() {
                txs.push(tx);
            }
        }

        txs
    }

    pub fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string();

        let meta = Self::parse_account_metadata(clean);
        let transactions = Self::parse_transactions(clean);

        // Build account descriptor string
        let mut acct_desc = Vec::new();
        if !meta.bank_id.is_empty() {
            acct_desc.push(format!("Bank: {}", meta.bank_id));
        }
        if !meta.acct_id.is_empty() {
            acct_desc.push(format!("Acct: {}", meta.acct_id));
        }
        if !meta.currency.is_empty() {
            acct_desc.push(format!("Cur: {}", meta.currency));
        }
        if !meta.dt_start.is_empty() && !meta.dt_end.is_empty() {
            acct_desc.push(format!("Period: {} to {}", meta.dt_start, meta.dt_end));
        }
        if !meta.balance.is_empty() {
            acct_desc.push(format!("Balance: {}", meta.balance));
        }
        let acct_summary = if acct_desc.is_empty() {
            "Financial Statement".to_string()
        } else {
            acct_desc.join(" | ")
        };

        let table_header = "| Date | Type | Amount | ID | Description / Memo |\n|---|---|---|---|---|";

        // If no transactions found (e.g. metadata-only OFX), emit a summary chunk
        if transactions.is_empty() {
            let header_path = format!("{} > {}", file_name, acct_summary);
            let text = format!("// Context: {}\n{}\n\n(No transactions recorded in this statement)", header_path, clean);
            return Ok(vec![ChunkPayload::new(
                format!("{}_chunk_0", file_name),
                text,
                file_name,
                file_path.to_string(),
                Some(header_path),
                1,
                clean.lines().count(),
                "ofx".to_string(),
                0,
            )]);
        }

        // Check if all transactions fit into a single chunk
        let num_txs = transactions.len();
        let mut chunks = Vec::new();
        let mut current_rows = Vec::new();
        let mut current_start_idx = 1;
        let mut current_chars = table_header.len() + acct_summary.len() + 100;

        for (idx, tx) in transactions.iter().enumerate() {
            let tx_idx = idx + 1;
            let formatted_row = format!(
                "| {} | {} | {} | {} | {} |",
                if tx.dt_posted.is_empty() { "N/A" } else { &tx.dt_posted },
                if tx.trn_type.is_empty() { "TRAN" } else { &tx.trn_type },
                if tx.amount.is_empty() { "0.00" } else { &tx.amount },
                if tx.fit_id.is_empty() { "-" } else { &tx.fit_id },
                if tx.memo.is_empty() { "-" } else { &tx.memo },
            );
            let row_len = formatted_row.len() + 1;

            if !current_rows.is_empty() && (current_chars + row_len > self.max_chunk_chars) {
                let end_idx = tx_idx - 1;
                let header_path = format!(
                    "{} > {} > transactions {}..{}",
                    file_name, acct_summary, current_start_idx, end_idx
                );
                let text = format!(
                    "// Context: {}\n{}\n{}",
                    header_path,
                    table_header,
                    current_rows.join("\n")
                );

                chunks.push(ChunkPayload::new(
                    format!("{}_chunk_{}", file_name, chunks.len()),
                    text,
                    file_name.clone(),
                    file_path.to_string(),
                    Some(header_path),
                    current_start_idx,
                    end_idx,
                    "ofx".to_string(),
                    chunks.len(),
                ));

                current_rows.clear();
                current_start_idx = tx_idx;
                current_chars = table_header.len() + acct_summary.len() + 100;
            }

            current_rows.push(formatted_row);
            current_chars += row_len;
        }

        if !current_rows.is_empty() {
            let end_idx = num_txs;
            let header_path = format!(
                "{} > {} > transactions {}..{}",
                file_name, acct_summary, current_start_idx, end_idx
            );
            let text = format!(
                "// Context: {}\n{}\n{}",
                header_path,
                table_header,
                current_rows.join("\n")
            );

            chunks.push(ChunkPayload::new(
                format!("{}_chunk_{}", file_name, chunks.len()),
                text,
                file_name.clone(),
                file_path.to_string(),
                Some(header_path),
                current_start_idx,
                end_idx,
                "ofx".to_string(),
                chunks.len(),
            ));
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofx_sgml_parsing() {
        let chunker = OfxChunker::new(1800);
        let ofx = r#"
OFXHEADER:100
DATA:OFXSGML
VERSION:102
SECURITY:NONE
ENCODING:USASCII
CHARSET:1252
COMPRESSION:NONE
OLDFILEUID:NONE
NEWFILEUID:NONE

<OFX>
<BANKMSGSRSV1>
<STMTTRNRS>
<STMTRS>
<CURDEF>USD
<BANKACCTFROM>
<BANKID>123456789
<ACCTID>987654321
<ACCTTYPE>CHECKING
</BANKACCTFROM>
<BANKTRANLIST>
<DTSTART>20260901000000
<DTEND>20260930235959
<STMTTRN>
<TRNTYPE>DEBIT
<DTPOSTED>20260902120000
<TRNAMT>-45.00
<FITID>20260902001
<MEMO>Coffee Shop
</STMTTRN>
<STMTTRN>
<TRNTYPE>CREDIT
<DTPOSTED>20260905100000
<TRNAMT>2500.00
<FITID>20260905001
<MEMO>Salary Deposit
</STMTTRN>
</BANKTRANLIST>
<LEDGERBAL>
<BALAMT>8450.00
<DTASOF>20260930235959
</LEDGERBAL>
</STMTRS>
</STMTTRNRS>
</BANKMSGSRSV1>
</OFX>
"#;

        let chunks = chunker.chunk("extrato.ofx", ofx).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "ofx");
        assert!(chunks[0].text.contains("// Context: extrato.ofx > Bank: 123456789 | Acct: 987654321"));
        assert!(chunks[0].text.contains("| 2026-09-02 | DEBIT | -45.00 | 20260902001 | Coffee Shop |"));
        assert!(chunks[0].text.contains("| 2026-09-05 | CREDIT | 2500.00 | 20260905001 | Salary Deposit |"));
    }

    #[test]
    fn test_ofx_xml_parsing() {
        let chunker = OfxChunker::new(1800);
        let ofx = r#"<?xml version="1.0" encoding="utf-8"?>
<OFX>
  <BANKMSGSRSV1>
    <STMTTRNRS>
      <STMTRS>
        <CURDEF>EUR</CURDEF>
        <BANKACCTFROM>
          <BANKID>DEUTDE88</BANKID>
          <ACCTID>DE1234567890</ACCTID>
        </BANKACCTFROM>
        <BANKTRANLIST>
          <STMTTRN>
            <TRNTYPE>DEBIT</TRNTYPE>
            <DTPOSTED>20260910</DTPOSTED>
            <TRNAMT>-19.99</TRNAMT>
            <FITID>TX001</FITID>
            <NAME>Software Subscription</NAME>
          </STMTTRN>
        </BANKTRANLIST>
        <LEDGERBAL>
          <BALAMT>1200.50</BALAMT>
        </LEDGERBAL>
      </STMTRS>
    </STMTTRNRS>
  </BANKMSGSRSV1>
</OFX>"#;

        let chunks = chunker.chunk("statement.ofx", ofx).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Bank: DEUTDE88 | Acct: DE1234567890"));
        assert!(chunks[0].text.contains("| 2026-09-10 | DEBIT | -19.99 | TX001 | Software Subscription |"));
    }
}
