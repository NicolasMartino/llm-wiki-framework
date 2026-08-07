use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WikiMetadata {
    pub title: Option<String>,
    pub fields: BTreeMap<String, String>,
}

impl WikiMetadata {
    pub fn document_class(&self) -> Option<&str> {
        self.field("Document Class")
    }

    pub fn status(&self) -> Option<&str> {
        self.field("Status")
    }

    pub fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

/// Parse the leading metadata of a wiki document.
///
/// The canonical emitted format is an `# H1` followed by a contiguous block of
/// `- Key: Value` bullets. To stay robust against externally authored notes,
/// this parser also accepts:
///
/// - bare `Key: Value` lines (no leading bullet),
/// - bold `**Key:** Value` (or `- **Key:** Value`) lines,
/// - a metadata block placed immediately *before* the H1 (some tools front-load
///   the header), and
/// - a leading `---` YAML-style frontmatter block.
///
/// The metadata block stays anchored to the H1 (immediately before or after,
/// modulo blank lines) so prose elsewhere in the document is never matched.
pub fn parse_wiki_metadata(input: &str) -> WikiMetadata {
    let mut metadata = WikiMetadata::default();
    let lines: Vec<&str> = input.lines().collect();

    // 1. Optional `---` frontmatter block at the very top.
    let mut start = 0;
    if lines.first().map(|l| l.trim()) == Some("---")
        && let Some(end) = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, l)| l.trim() == "---")
            .map(|(i, _)| i)
    {
        parse_block(&lines[1..end], &mut metadata.fields);
        start = end + 1;
    }

    // 2. Locate the H1 title (first `# ` line at/after `start`).
    let title_idx = lines
        .iter()
        .enumerate()
        .skip(start)
        .find(|(_, l)| l.trim().starts_with("# "))
        .map(|(i, _)| i);
    if let Some(i) = title_idx
        && let Some(title) = lines[i].trim().strip_prefix("# ")
    {
        let title = title.trim();
        if !title.is_empty() {
            metadata.title = Some(title.to_string());
        }
    }

    // 3. Metadata block immediately before the H1 (skip blanks between).
    if let Some(i) = title_idx {
        let mut end = i;
        while end > start && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        let mut begin = end;
        while begin > start
            && (metadata_pair(lines[begin - 1]).is_some() || is_continuation(lines[begin - 1]))
        {
            begin -= 1;
        }
        if begin < end {
            parse_block(&lines[begin..end], &mut metadata.fields);
        }
    }

    // 4. Metadata block immediately after the H1 (canonical bullet form). When
    //    there is no H1, scan from the document start so front-loaded headers
    //    without a title still parse.
    let after_start = title_idx.map(|i| i + 1).unwrap_or(start);
    let mut j = after_start;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    let mut k = j;
    while k < lines.len()
        && (metadata_pair(lines[k]).is_some() || (k > j && is_continuation(lines[k])))
    {
        k += 1;
    }
    if j < k {
        parse_block(&lines[j..k], &mut metadata.fields);
    }

    metadata
}

fn parse_block(lines: &[&str], fields: &mut BTreeMap<String, String>) {
    let mut current_key: Option<String> = None;
    for line in lines {
        if let Some((key, value)) = metadata_pair(line) {
            fields.insert(key.clone(), value);
            current_key = Some(key);
            continue;
        }

        if is_continuation(line)
            && let Some(key) = &current_key
            && let Some(value) = fields.get_mut(key)
        {
            let continuation = line.trim();
            if !continuation.is_empty() {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(continuation);
            }
        }
    }
}

fn metadata_pair(line: &str) -> Option<(String, String)> {
    // Indented lines are continuations, not new pairs.
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim_end();
    let rest = trimmed.strip_prefix("- ").unwrap_or(trimmed);
    let (key, value) = rest.split_once(':')?;
    let key = key
        .trim()
        .trim_start_matches('*')
        .trim_end_matches('*')
        .trim();
    if key.is_empty() {
        return None;
    }
    let value = value.trim();
    let value = value.strip_prefix("**").unwrap_or(value).trim();
    Some((key.to_string(), value.to_string()))
}

fn is_continuation(line: &str) -> bool {
    line.starts_with("  ") || line.starts_with('\t')
}

#[cfg(test)]
mod tests {
    use super::parse_wiki_metadata;

    #[test]
    fn parses_h1_and_leading_metadata_block() {
        let metadata = parse_wiki_metadata(
            "# Search Backend Selection\n\n- Document Class: Decision\n- Status: Accepted\n- Sources: wiki/evals/search-backend-selection.eval.md\n  wiki/proposals/search-backend-selection.proposal.md\n\n## Choice\nBody",
        );

        assert_eq!(metadata.title.as_deref(), Some("Search Backend Selection"));
        assert_eq!(metadata.document_class(), Some("Decision"));
        assert_eq!(metadata.status(), Some("Accepted"));
        assert_eq!(
            metadata.field("Sources"),
            Some(
                "wiki/evals/search-backend-selection.eval.md wiki/proposals/search-backend-selection.proposal.md"
            )
        );
    }

    #[test]
    fn missing_optional_fields_are_absent() {
        let metadata = parse_wiki_metadata("# Untyped\n\nBody");

        assert_eq!(metadata.title.as_deref(), Some("Untyped"));
        assert_eq!(metadata.document_class(), None);
        assert_eq!(metadata.status(), None);
    }

    #[test]
    fn ignores_later_bullets_outside_metadata_block() {
        let metadata = parse_wiki_metadata(
            "# Title\n\
             \n\
             - Document Class: Plan\n\
             \n\
             ## Later\n\
             \n\
             - Status: Wrong",
        );

        assert_eq!(metadata.document_class(), Some("Plan"));
        assert_eq!(metadata.status(), None);
    }

    #[test]
    fn parses_bare_metadata_block_before_h1() {
        // Real-world externally authored format (the `keto-diet` testbed): a bare
        // `Key: Value` block placed *before* the H1, no bullets.
        let metadata = parse_wiki_metadata(
            "Document Class: Checklist\n\
             Status: Active\n\
             Date: 2026-06-25\n\
             Category: Literature intake\n\
             Scope: Defines how sources are screened.\n\
             \n\
             # Review Source Screening Checklist\n\
             \n\
             Use this checklist before creating a note.",
        );

        assert_eq!(
            metadata.title.as_deref(),
            Some("Review Source Screening Checklist")
        );
        assert_eq!(metadata.document_class(), Some("Checklist"));
        assert_eq!(metadata.status(), Some("Active"));
        assert_eq!(metadata.field("Category"), Some("Literature intake"));
    }

    #[test]
    fn parses_bold_metadata_variant() {
        let metadata = parse_wiki_metadata(
            "# Bold Header\n\n**Document Class:** Spec\n**Status:** Draft\n\nBody",
        );

        assert_eq!(metadata.document_class(), Some("Spec"));
        assert_eq!(metadata.status(), Some("Draft"));
    }

    #[test]
    fn parses_yaml_frontmatter_block() {
        let metadata = parse_wiki_metadata(
            "---\nDocument Class: Decision\nStatus: Accepted\n---\n\n# Frontmatter Doc\n\nBody",
        );

        assert_eq!(metadata.title.as_deref(), Some("Frontmatter Doc"));
        assert_eq!(metadata.document_class(), Some("Decision"));
        assert_eq!(metadata.status(), Some("Accepted"));
    }

    #[test]
    fn bare_block_does_not_swallow_prose_paragraph() {
        // A non-metadata paragraph directly under the H1 must not be parsed as
        // metadata just because one line happens to contain a colon.
        let metadata = parse_wiki_metadata(
            "# Prose Doc\n\nThis is prose that mentions a ratio of 4:1 in passing.\n\nMore body.",
        );

        assert_eq!(metadata.document_class(), None);
        assert_eq!(metadata.status(), None);
    }
}
