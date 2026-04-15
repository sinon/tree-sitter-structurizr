//! Shared identifier-shape helpers for bounded LSP editing features.

/// Returns whether one flat bindable identifier matches the supported bounded
/// Structurizr shape.
///
/// The current local policy follows upstream's broad character set while
/// tightening one edge case: all-digit identifiers are rejected explicitly so
/// the editor surface can fail clearly instead of relying on a later generic
/// upstream parser error.
#[must_use]
pub fn is_valid_bindable_identifier(value: &str) -> bool {
    matches!(consume_bindable_identifier_segment(value), Some(""))
}

/// Consumes one identifier token from the front of a line and returns the
/// remaining suffix.
///
/// This mirrors the grammar's identifier surface: one or more bindable
/// segments separated by dots for hierarchical references.
#[must_use]
pub fn consume_identifier(line: &str) -> Option<&str> {
    let mut rest = consume_bindable_identifier_segment(line)?;
    while let Some(after_dot) = rest.strip_prefix('.') {
        rest = consume_bindable_identifier_segment(after_dot)?;
    }
    Some(rest)
}

/// Returns whether the trimmed line is exactly one supported identifier token.
#[must_use]
pub fn is_identifier_line(line: &str) -> bool {
    matches!(consume_identifier(line.trim()), Some(""))
}

fn consume_bindable_identifier_segment(line: &str) -> Option<&str> {
    let mut end = 0;
    let mut has_non_digit = false;

    for (index, ch) in line.char_indices() {
        let is_valid = if index == 0 {
            ch.is_ascii_alphanumeric() || ch == '_'
        } else {
            ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
        };
        if !is_valid {
            break;
        }
        if !ch.is_ascii_digit() {
            has_non_digit = true;
        }
        end = index + ch.len_utf8();
    }

    (end > 0 && has_non_digit).then_some(&line[end..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("abc", true)]
    #[case("1abc", true)]
    #[case("_abc", true)]
    #[case("abc-DEF", true)]
    #[case("1-abc", true)]
    #[case("111", false)]
    #[case("-abc", false)]
    #[case("abc.def", false)]
    #[case("", false)]
    fn bindable_identifier_shape_matches_policy(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(is_valid_bindable_identifier(input), expected);
    }

    #[rstest]
    #[case("api", Some(""))]
    #[case("1api", Some(""))]
    #[case("system.api", Some(""))]
    #[case("system.1api", Some(""))]
    #[case("1system.1api", Some(""))]
    #[case("111", None)]
    #[case("system.111", None)]
    #[case("system.", None)]
    #[case("-system", None)]
    #[case("api = deploymentEnvironment", Some(" = deploymentEnvironment"))]
    fn identifier_consumption_tracks_grammar_shape(
        #[case] input: &str,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(consume_identifier(input), expected);
    }
}
