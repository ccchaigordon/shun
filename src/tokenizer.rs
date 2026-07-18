/* ============================================================================
 * Shun: tokenizer.rs
 * ============================================================================
 *
 * This file normalizes document and query text into consistent as well as
 * positioned searchable terms. Indexing and query processing must use the same
 * rules so equivalent text produces equivalent terms.
 *
 * Technical identifiers are preserved as complete lowercase terms and expanded
 * into useful snake-case, camel-case, and kebab-case components.
 * Generated variants retain the same source position and line number.
 *
 * ============================================================================
 */

/// This represents one searchable term generated from a source token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Token {
    pub(crate) term: String,
    pub(crate) position: usize,
    pub(crate) line: usize,
}

/// This contains expanded searchable terms and the unexpanded document length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenizedText {
    pub(crate) tokens: Vec<Token>,
    pub(crate) source_token_count: usize,
}

/// This converts input text into normalized terms for indexing and search.
/// Parameters: text is borrowed document content or a query string. The input is
/// not modified.
/// Returns: Complete identifiers and their unique components in source order.
/// Positions are zero-based and shared by variants from the same source token,
/// line numbers are one-based.
pub(crate) fn tokenize(text: &str) -> TokenizedText {
    let mut tokens = Vec::new();
    let mut source_token = String::new();
    let mut source_line = 1;
    let mut current_line = 1;
    let mut position = 0;

    for character in text.chars() {
        if is_identifier_character(character) {
            if source_token.is_empty() {
                source_line = current_line;
            }
            source_token.push(character);
        } else {
            if append_variants(&source_token, position, source_line, &mut tokens) {
                position += 1;
            }
            source_token.clear();
        }

        if character == '\n' {
            current_line += 1;
        }
    }

    if append_variants(&source_token, position, source_line, &mut tokens) {
        position += 1;
    }

    TokenizedText {
        tokens,
        source_token_count: position,
    }
}

/// This identifies characters that can belong to a technical identifier.
/// Parameters: character is the Unicode scalar being inspected.
/// Returns: true for letters, numbers, underscores, and hyphens.
fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-')
}

/// This appends a normalized identifier and its unique component variants.
/// Parameters: source is one raw source token. position and line identify its
/// location, tokens receives the generated records.
/// Returns: true when the source contains a searchable term.
fn append_variants(source: &str, position: usize, line: usize, tokens: &mut Vec<Token>) -> bool {
    let source = source.trim_matches(['_', '-']);
    if source.is_empty() {
        return false;
    }

    let mut variants = vec![source.to_lowercase()];
    for segment in source
        .split(['_', '-'])
        .filter(|segment| !segment.is_empty())
    {
        for component in split_camel_case(segment) {
            let component = component.to_lowercase();
            if !variants.contains(&component) {
                variants.push(component);
            }
        }
    }

    tokens.extend(variants.into_iter().map(|term| Token {
        term,
        position,
        line,
    }));
    true
}

/// This splits camel-case and acronym boundaries without discarding Unicode text.
/// Parameters: identifier is one identifier segment without snake or kebab separators.
/// Returns: Borrowed component slices in their original order.
fn split_camel_case(identifier: &str) -> Vec<&str> {
    let characters: Vec<(usize, char)> = identifier.char_indices().collect();
    if characters.is_empty() {
        return Vec::new();
    }

    let mut boundaries = vec![0];
    for index in 1..characters.len() {
        let previous = characters[index - 1].1;
        let current = characters[index].1;
        let next = characters.get(index + 1).map(|(_, character)| *character);
        let starts_word = previous.is_lowercase() && current.is_uppercase();
        let ends_acronym = previous.is_uppercase()
            && current.is_uppercase()
            && next.is_some_and(char::is_lowercase);

        if starts_word || ends_acronym {
            boundaries.push(characters[index].0);
        }
    }
    boundaries.push(identifier.len());

    boundaries
        .windows(2)
        .map(|range| &identifier[range[0]..range[1]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This verifies complete identifier preservation plus snake, camel, and kebab splitting.
    /// Parameters: none. The test uses fixed technical identifiers.
    /// Returns: Nothing. The test panics if terms, positions, lines, or count differ.
    #[test]
    fn preserves_and_splits_technical_identifiers() {
        let tokenized = tokenize("build_search_index SearchIndexBuilder database-timeout");

        assert_eq!(tokenized.source_token_count, 3);
        assert_eq!(
            tokenized.tokens,
            vec![
                Token {
                    term: "build_search_index".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "build".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "search".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "index".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "searchindexbuilder".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "search".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "index".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "builder".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "database-timeout".into(),
                    position: 2,
                    line: 1
                },
                Token {
                    term: "database".into(),
                    position: 2,
                    line: 1
                },
                Token {
                    term: "timeout".into(),
                    position: 2,
                    line: 1
                },
            ]
        );
    }

    /// This verifies acronym splitting and source locations across line boundaries.
    /// Parameters: none. The test uses a fixed two-line source string.
    /// Returns: Nothing. The test panics if terms or source locations differ.
    #[test]
    fn tracks_source_positions_and_lines() {
        let tokenized = tokenize("HTTPServer starts\nSearchIndex");

        assert_eq!(tokenized.source_token_count, 3);
        assert_eq!(
            tokenized.tokens,
            vec![
                Token {
                    term: "httpserver".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "http".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "server".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "starts".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "searchindex".into(),
                    position: 2,
                    line: 2
                },
                Token {
                    term: "search".into(),
                    position: 2,
                    line: 2
                },
                Token {
                    term: "index".into(),
                    position: 2,
                    line: 2
                },
            ]
        );
    }

    /// This verifies prose punctuation, Unicode case boundaries, numbers, and empty separators.
    /// Parameters: none. The test uses a fixed mixed prose and identifier string.
    /// Returns: Nothing. The test panics if empty terms consume positions or output differs.
    #[test]
    fn handles_prose_unicode_and_empty_separators() {
        let tokenized = tokenize("Rust's ÜberParser v2 ___ --");

        assert_eq!(tokenized.source_token_count, 4);
        assert_eq!(
            tokenized.tokens,
            vec![
                Token {
                    term: "rust".into(),
                    position: 0,
                    line: 1
                },
                Token {
                    term: "s".into(),
                    position: 1,
                    line: 1
                },
                Token {
                    term: "überparser".into(),
                    position: 2,
                    line: 1
                },
                Token {
                    term: "über".into(),
                    position: 2,
                    line: 1
                },
                Token {
                    term: "parser".into(),
                    position: 2,
                    line: 1
                },
                Token {
                    term: "v2".into(),
                    position: 3,
                    line: 1
                },
            ]
        );
    }
}
