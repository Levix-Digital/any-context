//! The LLM's not-enough signal (IMP-038, ADR-0161).
//!
//! In automatic mode, when the quick search found sufficient evidence, the answer prompt asks
//! the LLM to reply only with `NOT_ENOUGH_SIGNAL` when the excerpts do not hold the answer.
//! The agent then drops the attempt and escalates to Deep Search. `SignalFilter` holds back the
//! start of a streamed answer until it can no longer be the signal, so no token of a dropped
//! attempt reaches the user.

/// The fixed reply that asks for a deeper search. Never shown or stored.
pub const NOT_ENOUGH_SIGNAL: &str = "[[ACTX_NEEDS_DEEPER_SEARCH]]";

/// Escalation reason shown when the LLM gave the signal.
pub const NOT_ENOUGH_REASON: &str = "the quick search excerpts do not answer the question";

/// Rule appended to the quick search prompt when the answer can escalate.
pub fn not_enough_rule() -> String {
    format!(
        "\n\n<escalation_rule>\nIf the excerpts above do not contain the information needed to answer the user query, reply with exactly {NOT_ENOUGH_SIGNAL} and nothing else; a deeper search will follow. Otherwise answer normally and never write that marker.\n</escalation_rule>"
    )
}

/// The quick search prompt with the not-enough rule appended.
pub fn with_not_enough_rule(prompt: &str) -> String {
    format!("{prompt}{}", not_enough_rule())
}

/// Removes every occurrence of the signal from an answer that did not start with it.
pub fn strip_signal(text: &str) -> String {
    if text.contains(NOT_ENOUGH_SIGNAL) {
        text.replace(NOT_ENOUGH_SIGNAL, "")
    } else {
        text.to_string()
    }
}

/// Whether a complete answer is the not-enough signal.
pub fn is_signal(answer: &str) -> bool {
    answer.trim_start().starts_with(NOT_ENOUGH_SIGNAL)
}

/// What the filter decided after a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterStep {
    /// Still undecided: nothing to show yet.
    Hold,
    /// The answer is not the signal: show this text (may be empty).
    Pass(String),
    /// The answer is the signal: drop the attempt and escalate.
    Signal,
}

/// Streaming check for the signal at the start of an answer.
#[derive(Debug, Default)]
pub struct SignalFilter {
    held: String,
    decided: bool,
}

impl SignalFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one streamed token.
    pub fn push(&mut self, token: &str) -> FilterStep {
        if self.decided {
            return FilterStep::Pass(token.to_string());
        }
        self.held.push_str(token);
        let start = self.held.trim_start();
        if start.starts_with(NOT_ENOUGH_SIGNAL) {
            self.decided = true;
            return FilterStep::Signal;
        }
        if start.is_empty() || NOT_ENOUGH_SIGNAL.starts_with(start) {
            return FilterStep::Hold;
        }
        self.decided = true;
        FilterStep::Pass(std::mem::take(&mut self.held))
    }

    /// The text still held when the stream ends (a short answer that looked like a signal prefix).
    pub fn finish(&mut self) -> String {
        self.decided = true;
        std::mem::take(&mut self.held)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(tokens: &[&str]) -> (String, bool) {
        let mut f = SignalFilter::new();
        let mut shown = String::new();
        for t in tokens {
            match f.push(t) {
                FilterStep::Hold => {}
                FilterStep::Pass(s) => shown.push_str(&s),
                FilterStep::Signal => return (shown, true),
            }
        }
        shown.push_str(&f.finish());
        (shown, false)
    }

    #[test]
    fn the_whole_signal_escalates() {
        assert_eq!(run(&[NOT_ENOUGH_SIGNAL]), (String::new(), true));
    }

    #[test]
    fn a_signal_split_over_tokens_and_after_whitespace_escalates() {
        assert_eq!(run(&["\n ", "[[ACTX_", "NEEDS_DEEPER", "_SEARCH]]", "\n"]), (String::new(), true));
    }

    #[test]
    fn a_normal_answer_passes_at_the_first_mismatch() {
        let mut f = SignalFilter::new();
        assert_eq!(f.push("The"), FilterStep::Pass("The".into()));
        assert_eq!(f.push(" port"), FilterStep::Pass(" port".into()));
    }

    #[test]
    fn a_prefix_that_diverges_is_shown_whole() {
        assert_eq!(run(&["[[", "note]] the port is 8080"]), ("[[note]] the port is 8080".into(), false));
    }

    #[test]
    fn a_short_answer_inside_a_signal_prefix_is_shown_at_the_end() {
        assert_eq!(run(&["[["]), ("[[".into(), false));
    }

    #[test]
    fn the_signal_later_in_an_answer_is_not_a_signal_and_is_stripped() {
        let (shown, signal) = run(&["Partial answer. ", NOT_ENOUGH_SIGNAL]);
        assert!(!signal);
        assert_eq!(strip_signal(&shown), "Partial answer. ");
        assert!(!is_signal(&shown));
        assert!(is_signal(&format!("  {NOT_ENOUGH_SIGNAL}")));
    }

    #[test]
    fn the_rule_names_the_signal() {
        assert!(with_not_enough_rule("prompt").starts_with("prompt\n\n<escalation_rule>"));
        assert!(not_enough_rule().contains(NOT_ENOUGH_SIGNAL));
    }
}
