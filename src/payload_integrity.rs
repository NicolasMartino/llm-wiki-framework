use serde_json::Value;

use crate::manifest::hash::sha256_hex;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum PayloadIntegrityError {
    HeadroomCompressionMarker,
    ReadContentByteLenMismatch { expected: usize, observed: usize },
    ReadContentSha256Mismatch { expected: String, observed: String },
    SearchResultsOmitted { result_count: u64 },
}

impl std::fmt::Display for PayloadIntegrityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeadroomCompressionMarker => {
                write!(formatter, "payload contains a Headroom compression marker")
            }
            Self::ReadContentByteLenMismatch { expected, observed } => write!(
                formatter,
                "read content byte_len mismatch: expected {expected}, observed {observed}"
            ),
            Self::ReadContentSha256Mismatch { expected, observed } => write!(
                formatter,
                "read content sha256 mismatch: expected {expected}, observed {observed}"
            ),
            Self::SearchResultsOmitted { result_count } => write!(
                formatter,
                "search payload omitted results despite result_count={result_count}"
            ),
        }
    }
}

pub(crate) fn read_content_integrity_error(
    content: &str,
    expected_sha256: &str,
    expected_byte_len: usize,
) -> Option<PayloadIntegrityError> {
    if has_headroom_marker(content) {
        return Some(PayloadIntegrityError::HeadroomCompressionMarker);
    }

    let observed_byte_len = content.len();
    if observed_byte_len != expected_byte_len {
        return Some(PayloadIntegrityError::ReadContentByteLenMismatch {
            expected: expected_byte_len,
            observed: observed_byte_len,
        });
    }

    let observed_sha256 = sha256_hex(content.as_bytes());
    (observed_sha256 != expected_sha256).then(|| PayloadIntegrityError::ReadContentSha256Mismatch {
        expected: expected_sha256.to_string(),
        observed: observed_sha256,
    })
}

pub(crate) fn json_payload_integrity_error(value: &Value) -> Option<PayloadIntegrityError> {
    if json_has_headroom_marker(value) {
        return Some(PayloadIntegrityError::HeadroomCompressionMarker);
    }
    search_results_omitted(value)
}

fn search_results_omitted(value: &Value) -> Option<PayloadIntegrityError> {
    let result_count = value
        .get("result_count")
        .and_then(Value::as_u64)
        .or_else(|| positive_project_result_count(value));

    match (result_count, value.get("results")) {
        (Some(result_count), None) if result_count > 0 => {
            Some(PayloadIntegrityError::SearchResultsOmitted { result_count })
        }
        _ => None,
    }
}

fn positive_project_result_count(value: &Value) -> Option<u64> {
    value
        .get("projects")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|project| project.get("result_count").and_then(Value::as_u64))
        .find(|count| *count > 0)
}

fn json_has_headroom_marker(value: &Value) -> bool {
    match value {
        Value::String(text) => has_headroom_marker(text),
        Value::Array(items) => items.iter().any(json_has_headroom_marker),
        Value::Object(object) => object.values().any(json_has_headroom_marker),
        _ => false,
    }
}

fn has_headroom_marker(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("<<ccr:")
        || (trimmed.starts_with('[')
            && trimmed.contains(" items compressed ")
            && trimmed.contains(" compressed to "))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        PayloadIntegrityError, json_payload_integrity_error, read_content_integrity_error,
    };
    use crate::manifest::hash::sha256_hex;

    #[test]
    fn detects_headroom_markers_in_read_content() {
        let error = read_content_integrity_error("<<ccr:72660c4ccf86,html,36.9KB>>", "sha", 36)
            .expect("integrity error");

        assert_eq!(error, PayloadIntegrityError::HeadroomCompressionMarker);
    }

    #[test]
    fn validates_read_content_length_and_hash() {
        let content = "real wiki text";
        assert_eq!(
            read_content_integrity_error(content, &sha256_hex(content.as_bytes()), content.len()),
            None
        );

        assert!(matches!(
            read_content_integrity_error(
                content,
                &sha256_hex(content.as_bytes()),
                content.len() + 1
            ),
            Some(PayloadIntegrityError::ReadContentByteLenMismatch { .. })
        ));

        assert!(matches!(
            read_content_integrity_error(content, "bad-sha", content.len()),
            Some(PayloadIntegrityError::ReadContentSha256Mismatch { .. })
        ));
    }

    #[test]
    fn detects_search_result_omission() {
        let error = json_payload_integrity_error(&json!({
            "selected_mode": "hybrid",
            "result_count": 6
        }))
        .expect("integrity error");

        assert_eq!(
            error,
            PayloadIntegrityError::SearchResultsOmitted { result_count: 6 }
        );
    }

    #[test]
    fn detects_project_result_omission() {
        let error = json_payload_integrity_error(&json!({
            "projects": [{"project_id": "fixture", "result_count": 2}]
        }))
        .expect("integrity error");

        assert_eq!(
            error,
            PayloadIntegrityError::SearchResultsOmitted { result_count: 2 }
        );
    }

    #[test]
    fn accepts_explicit_results_array() {
        assert_eq!(
            json_payload_integrity_error(&json!({
                "result_count": 1,
                "results": []
            })),
            None
        );
    }

    #[test]
    fn detects_nested_compression_envelope() {
        assert_eq!(
            json_payload_integrity_error(&json!({
                "stderr": "[233 items compressed to 4. Retrieve more: hash=abc]"
            })),
            Some(PayloadIntegrityError::HeadroomCompressionMarker)
        );
    }
}
