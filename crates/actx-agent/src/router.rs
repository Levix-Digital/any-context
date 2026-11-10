//! # 🧭 Route of a question (IMP-038, ADR-0161)
//!
//! The route is chosen from the search mode only, never from the question's words: automatic
//! mode and pinned Fast start with the quick search, pinned Deep starts with Deep Search.
//! Automatic mode escalates to Deep on evidence after the quick search (see `escalation`).
//! The one exception is the live status question, recognized by its terms until status
//! becomes an agent tool (FEAT-082).

use crate::config::SearchMode;

/// Where a question is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// One retrieval and one answer call; in automatic mode it may escalate to Deep.
    Fast,
    /// Deep Search: decomposition, reflection turns and synthesis.
    Deep,
    /// Live synchronization and indexing status of the workspace.
    Status,
}

impl Route {
    pub fn as_str(&self) -> &'static str {
        match self {
            Route::Fast => "fast",
            Route::Deep => "deep",
            Route::Status => "status",
        }
    }
}

/// The route of one question and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecision {
    pub route: Route,
    /// The user pinned the mode (`/search fast` or `/search deep`); false in automatic mode.
    pub pinned: bool,
    /// Short reason for the routing line of the interfaces.
    pub reason: &'static str,
}

/// Terms that ask about the live state of the workspace rather than how to use a feature.
/// Removed when status becomes an agent tool (FEAT-082).
const STATUS_TERMS: &[&str] = &[
    "indexed", "indexing", "indexado", "indexada", "indexação", "indexando",
    "syncing", "sync status", "sincronizando", "sincronização", "sincronizado",
    "status", "progress", "progresso", "how many chunks", "quantos chunks",
];

/// Whether the query asks for the live synchronization or indexing status.
pub fn is_status_query(query: &str) -> bool {
    let lower = query.to_lowercase();
    STATUS_TERMS.iter().any(|term| lower.contains(term))
}

/// Routes a question under the search mode.
pub fn route(query: &str, search_mode: SearchMode) -> RouteDecision {
    if is_status_query(query) {
        return RouteDecision {
            route: Route::Status,
            pinned: false,
            reason: "live status question",
        };
    }
    mode_route(search_mode)
}

/// The route of the search mode alone, for a question that is not a status question (or when
/// no status retriever is configured).
pub fn mode_route(search_mode: SearchMode) -> RouteDecision {
    match search_mode {
        SearchMode::Auto => RouteDecision {
            route: Route::Fast,
            pinned: false,
            reason: "automatic mode starts with the quick search and moves to deep search when the evidence is not enough",
        },
        SearchMode::Fast => RouteDecision {
            route: Route::Fast,
            pinned: true,
            reason: "quick search pinned with /search fast",
        },
        SearchMode::Deep => RouteDecision {
            route: Route::Deep,
            pinned: true,
            reason: "deep search pinned with /search deep",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_mode_starts_fast_whatever_the_words() {
        for q in [
            "Compare a arquitetura end-to-end do módulo A versus o módulo B e explique os trade-offs e o ciclo de vida completo de cada um?",
            "where is the port defined?",
            "¿Cuál es la diferencia entre los dos contratos firmados en 2024 y qué cláusulas cambiaron?",
        ] {
            let d = route(q, SearchMode::Auto);
            assert_eq!(d.route, Route::Fast, "{q}");
            assert!(!d.pinned);
        }
    }

    #[test]
    fn pinned_modes_are_respected() {
        let fast = route("explain the whole architecture end-to-end", SearchMode::Fast);
        assert_eq!((fast.route, fast.pinned), (Route::Fast, true));
        let deep = route("what port?", SearchMode::Deep);
        assert_eq!((deep.route, deep.pinned), (Route::Deep, true));
    }

    #[test]
    fn status_questions_take_the_status_route_in_every_mode() {
        for mode in [SearchMode::Auto, SearchMode::Fast, SearchMode::Deep] {
            assert_eq!(route("what is the sync status?", mode).route, Route::Status);
            assert_eq!(route("quantos chunks já foram indexados?", mode).route, Route::Status);
        }
        assert!(!is_status_query("how do I add a folder?"));
    }
}
