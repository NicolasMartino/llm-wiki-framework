pub fn sanitize_fts_query(raw: &str) -> String {
    let mut terms: Vec<String> = Vec::new();
    for word in fts_words(raw) {
        if !terms.contains(&word) {
            terms.push(word);
        }
    }
    terms.join(" ")
}

pub fn query_terms(raw: &str) -> Vec<String> {
    sanitize_fts_query(raw)
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

/// The fallback query for a search whose all-words query found too few pages:
/// each name of the query kept as one phrase, the phrases joined by OR. A
/// hyphenated name such as `headroom-wrap-command` otherwise becomes three
/// words that common pages hold apart.
///
/// `None` when the query has fewer than two names, since a single phrase can
/// only match pages the all-words query already found.
pub fn fts_phrase_fallback_query(raw: &str) -> Option<String> {
    let phrases = query_names(raw);
    (phrases.len() > 1).then(|| {
        phrases
            .iter()
            .map(|phrase| format!("\"{phrase}\""))
            .collect::<Vec<_>>()
            .join(" OR ")
    })
}

/// A name ends at whitespace, a comma, a semicolon, a quote or a backtick;
/// any other punctuation inside it (hyphens, slashes, dots) joins its words.
fn query_names(raw: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in
        raw.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '"' | '\'' | '`'))
    {
        let phrase = fts_words(name).collect::<Vec<_>>().join(" ");
        if !phrase.is_empty() && !names.contains(&phrase) {
            names.push(phrase);
        }
    }
    names
}

/// The query's words as FTS5 reads them: runs of letters and digits,
/// lowercased. The all-words and the phrase query both split this way, so
/// their words stay the same.
fn fts_words(raw: &str) -> impl Iterator<Item = String> + '_ {
    raw.split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.chars().flat_map(char::to_lowercase).collect())
}

#[cfg(test)]
mod tests {
    use super::{fts_phrase_fallback_query, sanitize_fts_query};

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

    #[test]
    fn a_comma_ends_a_name_and_a_hyphen_joins_its_words() {
        assert_eq!(
            fts_phrase_fallback_query("qmd-rs,search-all").as_deref(),
            Some("\"qmd rs\" OR \"search all\"")
        );
    }

    #[test]
    fn a_path_is_one_name() {
        assert_eq!(
            fts_phrase_fallback_query("wiki/plans/headroom-wrap-command.plan.md"),
            None
        );
        assert_eq!(
            fts_phrase_fallback_query("wiki/plans/headroom-wrap-command.plan.md wrap").as_deref(),
            Some("\"wiki plans headroom wrap command plan md\" OR \"wrap\"")
        );
    }

    #[test]
    fn names_without_joins_fall_back_to_a_plain_or_of_words() {
        assert_eq!(
            fts_phrase_fallback_query("`Three`; phase \"ingest\" phase").as_deref(),
            Some("\"three\" OR \"phase\" OR \"ingest\"")
        );
        assert_eq!(fts_phrase_fallback_query("-- ,;"), None);
    }
}
