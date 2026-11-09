//! Nothing-found and weak-evidence gate (IMP-046, ADR-0160).
//!
//! The answer path reads the evidence of a retrieval from the reranker bands of the delivered
//! chunks: nothing, weak (every chunk in the maybe band) or sufficient. Chunks without a band
//! (a pinned file-name match, or a pass the reranker did not judge) count as sufficient.

use super::pipeline::HybridSearchResult;
use super::relevance::RelevanceBand;

pub use actx_agent::WEAK_EVIDENCE_WARNING;

/// Rule added to the answer prompt when the evidence is weak.
pub const WEAK_EVIDENCE_RULE: &str = "WEAK EVIDENCE: none of the excerpts below was judged clearly relevant to the question; they are only possibly related. Use only what they state directly, say plainly which parts of the question they do not answer, and do not fill the gaps.";

/// What the delivered chunks of a retrieval can support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    /// No chunk was delivered.
    Nothing,
    /// Every delivered chunk is in the maybe band (or below it, when the relevance cut is off).
    Weak,
    /// A relevant chunk, a pinned match, or chunks the reranker did not band.
    Sufficient,
}

impl EvidenceClass {
    /// The class of a set of bands, one per delivered chunk.
    pub fn of_bands<I: IntoIterator<Item = Option<RelevanceBand>>>(bands: I) -> Self {
        let mut any = false;
        for band in bands {
            any = true;
            if matches!(band, None | Some(RelevanceBand::Relevant)) {
                return EvidenceClass::Sufficient;
            }
        }
        if any { EvidenceClass::Weak } else { EvidenceClass::Nothing }
    }
}

/// The evidence class of the delivered results.
pub fn classify_evidence(results: &[HybridSearchResult]) -> EvidenceClass {
    EvidenceClass::of_bands(results.iter().map(|r| r.relevance_band))
}

/// The honest "nothing found" answer, given without calling the language model: the workspace,
/// its monitored sources and what the user can do.
pub fn nothing_found_message(
    workspace: &str,
    monitored_folders: &[String],
    monitored_urls: &[String],
    query: &str,
) -> String {
    let mut sources = Vec::new();
    if !monitored_folders.is_empty() {
        sources.push(format!("local folders: [{}]", monitored_folders.join(", ")));
    }
    if !monitored_urls.is_empty() {
        sources.push(format!("web sources: [{}]", monitored_urls.join(", ")));
    }
    let monitored = if sources.is_empty() {
        " The workspace has no sources yet.".to_string()
    } else {
        format!(" The workspace monitors {}.", sources.join(" and "))
    };

    format!(
        "Nothing relevant to '{}' was found in the workspace '{}'.{}\n\n\
        💡 Suggestions:\n\
        • Rephrase the question with other terms, names or file names.\n\
        • Add the sources that hold the answer with `/folder <path>` or `/web <url>`.",
        query.trim(),
        workspace,
        monitored
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use RelevanceBand::{Maybe, Relevant};

    #[test]
    fn no_chunk_is_nothing() {
        assert_eq!(EvidenceClass::of_bands(Vec::<Option<RelevanceBand>>::new()), EvidenceClass::Nothing);
        assert_eq!(classify_evidence(&[]), EvidenceClass::Nothing);
    }

    #[test]
    fn only_maybe_chunks_are_weak() {
        assert_eq!(EvidenceClass::of_bands([Some(Maybe), Some(Maybe)]), EvidenceClass::Weak);
    }

    #[test]
    fn one_relevant_or_pinned_chunk_is_sufficient() {
        assert_eq!(EvidenceClass::of_bands([Some(Maybe), Some(Relevant)]), EvidenceClass::Sufficient);
        // A pinned file-name match has no band.
        assert_eq!(EvidenceClass::of_bands([None, Some(Maybe)]), EvidenceClass::Sufficient);
    }

    #[test]
    fn irrelevant_chunks_kept_without_the_cut_are_not_sufficient() {
        assert_eq!(EvidenceClass::of_bands([Some(RelevanceBand::Irrelevant), Some(Maybe)]), EvidenceClass::Weak);
    }

    #[test]
    fn chunks_without_bands_are_not_judged() {
        assert_eq!(EvidenceClass::of_bands([None, None]), EvidenceClass::Sufficient);
    }

    #[test]
    fn the_message_names_the_sources_and_the_suggestions() {
        let msg = nothing_found_message("Finance", &["C:\\docs".to_string()], &[], " qual o saldo? ");
        assert!(msg.contains("'qual o saldo?'"));
        assert!(msg.contains("'Finance'"));
        assert!(msg.contains("local folders: [C:\\docs]"));
        assert!(msg.contains("Rephrase"));
        assert!(msg.contains("`/folder <path>`") && msg.contains("`/web <url>`"));
        assert!(!msg.contains("web-search"));
    }

    #[test]
    fn a_workspace_without_sources_says_so() {
        let msg = nothing_found_message("Empty", &[], &[], "q");
        assert!(msg.contains("has no sources yet"));
    }
}
