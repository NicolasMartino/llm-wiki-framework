pub fn sanitize_fts_query(raw: &str) -> String {
    let mut terms = Vec::new();
    let mut current = String::new();

    for ch in raw.chars() {
        if ch.is_alphanumeric() {
            current.extend(ch.to_lowercase());
        } else {
            push_term(&mut terms, &mut current);
        }
    }
    push_term(&mut terms, &mut current);

    terms.join(" ")
}

pub fn query_terms(raw: &str) -> Vec<String> {
    sanitize_fts_query(raw)
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn push_term(terms: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    if !terms.iter().any(|term| term == current) {
        terms.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_fts_query;

    #[test]
    fn command_like_tokens_are_split_into_fts_safe_terms() {
        assert_eq!(sanitize_fts_query("qmd-rs"), "qmd rs");
        assert_eq!(sanitize_fts_query("search-all"), "search all");
        assert_eq!(
            sanitize_fts_query("`llm-wiki search-all`"),
            "llm wiki search all"
        );
    }

    #[test]
    fn paths_are_normalized_without_parser_punctuation() {
        assert_eq!(
            sanitize_fts_query("wiki/proposals/search-backend-selection.proposal.md"),
            "wiki proposals search backend selection proposal md"
        );
    }

    #[test]
    fn repeated_terms_are_deduplicated_in_original_order() {
        assert_eq!(sanitize_fts_query("Search search SEARCH"), "search");
    }
}
