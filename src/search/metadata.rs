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

pub fn parse_wiki_metadata(input: &str) -> WikiMetadata {
    let mut metadata = WikiMetadata::default();
    let mut lines = input.lines().peekable();

    for line in lines.by_ref() {
        if let Some(title) = line.trim().strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                metadata.title = Some(title.to_string());
            }
            break;
        }
    }

    while matches!(lines.peek(), Some(line) if line.trim().is_empty()) {
        lines.next();
    }

    let mut current_key: Option<String> = None;
    for line in lines {
        if let Some((key, value)) = metadata_pair(line) {
            metadata.fields.insert(key.clone(), value);
            current_key = Some(key);
            continue;
        }

        if is_continuation(line)
            && let Some(key) = &current_key
            && let Some(value) = metadata.fields.get_mut(key)
        {
            let continuation = line.trim();
            if !continuation.is_empty() {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(continuation);
            }
            continue;
        }

        break;
    }

    metadata
}

fn metadata_pair(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("- ")?;
    let (key, value) = rest.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), value.trim().to_string()))
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
}
