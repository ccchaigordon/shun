/* ============================================================================
 * Shun: tokenizer.rs
 * ============================================================================
 *
 * This file normalizes document and query text into consistent searchable terms.
 * Indexing and query processing must use the same tokenization rules so
 * equivalent text produces equivalent terms.
 *
 * My current implementation keeps Unicode letters and numbers, treats all other
 * characters as separators, removes empty terms, and converts retained terms to
 * lowercase.
 *
 * TODO: More advanced behavior such as stop-word removal or stemming will be
 * added here later.
 *
 * ============================================================================
 */

/// This converts input text into lowercase alphanumeric terms for indexing and search.
/// Parameters: text is the borrowed document content or query string to normalize. The
/// input is not modified.
/// Returns: Owned lowercase terms in their original order.

pub(crate) fn tokenize(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokenizes_mixed_case_and_punctuation() {
        assert_eq!(
            tokenize("Rust, Search-engine_v2!"),
            vec!["rust", "search", "engine", "v2"]
        );
    }
}
