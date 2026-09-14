use eyre::Result;
use eyre::bail;

/// Returns whether a value starts with an HTTP or HTTPS scheme.
#[must_use]
pub fn starts_with_http(value: &str) -> bool {
    value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || value
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
}

/// Validates a command-line URL without interpreting it as shell syntax.
///
/// # Errors
///
/// Returns an error if the value is not a single HTTP or HTTPS URL.
pub fn require_http_url(value: &str) -> Result<String> {
    let value =
        value.trim_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    if !starts_with_http(value) || value.chars().any(char::is_whitespace) {
        bail!("expected one HTTP or HTTPS URL");
    }
    Ok(value.to_string())
}

/// Extracts the first HTTP or HTTPS URL from shared text.
#[must_use]
pub fn extract_url(value: &str) -> Option<String> {
    let value =
        value.trim_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    if starts_with_http(value) && !value.chars().any(char::is_whitespace) {
        return Some(value.to_string());
    }

    let start = value
        .char_indices()
        .find_map(|(index, _)| starts_with_http(&value[index..]).then_some(index))?;
    let remainder = &value[start..];
    let end = remainder
        .char_indices()
        .find_map(|(index, character)| {
            (character.is_whitespace() || matches!(character, '"' | '\'' | '<' | '>' | '\u{feff}'))
                .then_some(index)
        })
        .unwrap_or(remainder.len());

    let candidate = remainder[..end].trim_end_matches(['.', ',', ';', '!', ')', ']', '}']);
    (!candidate.is_empty()).then(|| candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_url() {
        assert_eq!(
            extract_url("https://x.com/example/status/123"),
            Some("https://x.com/example/status/123".to_string())
        );
    }

    #[test]
    fn extracts_a_url_from_share_text() {
        assert_eq!(
            extract_url("Take a look: https://x.com/example/status/123). More text"),
            Some("https://x.com/example/status/123".to_string())
        );
    }

    #[test]
    fn rejects_non_http_text() {
        assert_eq!(extract_url("no link here"), None);
        assert!(require_http_url("file:///C:/video.mp4").is_err());
    }
}
