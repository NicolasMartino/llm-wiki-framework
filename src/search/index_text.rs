pub(crate) const SEARCH_IGNORE_START: &str = "<!-- llm-wiki-search-ignore-start -->";
pub(crate) const SEARCH_IGNORE_END: &str = "<!-- llm-wiki-search-ignore-end -->";

/// Masks content that is useful for humans and eval harnesses but should not
/// influence retrieval. The returned string preserves byte length and character
/// boundary positions so index spans can still be applied to the source file.
pub(crate) fn mask_search_ignored_spans(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    let mut ignored = false;

    loop {
        let marker = if ignored {
            SEARCH_IGNORE_END
        } else {
            SEARCH_IGNORE_START
        };
        let Some(index) = rest.find(marker) else {
            if ignored {
                push_masked(&mut output, rest);
            } else {
                output.push_str(rest);
            }
            break;
        };

        let (before, marker_and_after) = rest.split_at(index);
        if ignored {
            push_masked(&mut output, before);
            let (marker_text, after) = marker_and_after.split_at(marker.len());
            push_masked(&mut output, marker_text);
            rest = after;
            ignored = false;
        } else {
            output.push_str(before);
            let (marker_text, after) = marker_and_after.split_at(marker.len());
            push_masked(&mut output, marker_text);
            rest = after;
            ignored = true;
        }
    }

    debug_assert_eq!(output.len(), input.len());
    output
}

fn push_masked(output: &mut String, input: &str) {
    for ch in input.chars() {
        output.push(mask_char(ch));
    }
}

fn mask_char(ch: char) -> char {
    match ch {
        '\n' | '\r' => ch,
        _ => match ch.len_utf8() {
            1 => ' ',
            2 => '\u{00a0}',
            3 => '\u{2003}',
            4 => '\u{10000}',
            _ => unreachable!("UTF-8 characters are 1 to 4 bytes"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{SEARCH_IGNORE_END, SEARCH_IGNORE_START, mask_search_ignored_spans};

    #[test]
    fn masks_ignored_spans_while_preserving_visible_text_and_byte_shape() {
        let input = format!(
            "visible\n{SEARCH_IGNORE_START}\nGPU shader compiler roadmap 🙂\n{SEARCH_IGNORE_END}\nkept"
        );

        let masked = mask_search_ignored_spans(&input);

        assert_eq!(masked.len(), input.len());
        assert!(masked.starts_with("visible\n"));
        assert!(masked.ends_with("\nkept"));
        assert!(!masked.contains("GPU shader compiler roadmap"));
        assert!(!masked.contains(SEARCH_IGNORE_START));
        assert!(!masked.contains(SEARCH_IGNORE_END));
        for (index, _) in input.char_indices() {
            assert!(masked.is_char_boundary(index));
        }
    }

    #[test]
    fn masks_to_end_when_ignore_span_is_unclosed() {
        let input = format!("kept {SEARCH_IGNORE_START} hidden");
        let masked = mask_search_ignored_spans(&input);

        assert_eq!(masked.len(), input.len());
        assert!(masked.starts_with("kept "));
        assert!(!masked.contains("hidden"));
    }
}
