/* ============================================================================
 * Shun: search.rs
 * ============================================================================
 *
 * This file processes developer queries against Shun's in-memory inverted
 * index. What it does includes:
 *
 * 1. grouping normalized variants by source query token
 * 2. combining posting lists with OR or AND semantics
 * 3. ranking matches by term frequency
 * 4. selecting the earliest matching source line for snippet generation.
 *
 * ============================================================================
 */

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::index::{DocumentId, SearchIndex};
use crate::tokenizer::tokenize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchMode {
    Any,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchResult {
    pub(crate) document_id: DocumentId,
    pub(crate) score: usize,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) line: usize,
}

#[derive(Debug, Default)]
struct ResultBuilder {
    score: usize,
    matched_terms: BTreeSet<String>,
    matched_groups: BTreeSet<usize>,
    lines: BTreeSet<usize>,
}

/// This searches an index with normalized OR or AND matching.
/// Parameters: index contains repository postings, query is raw user text, and
/// mode selects whether any or all source query tokens must match.
/// Returns: Results ordered by descending term-frequency score and then path.
pub(crate) fn search(index: &SearchIndex, query: &str, mode: MatchMode) -> Vec<SearchResult> {
    let query_groups = query_groups(query);
    if query_groups.is_empty() {
        return Vec::new();
    }

    let mut result_builders: BTreeMap<DocumentId, ResultBuilder> = BTreeMap::new();

    for (group_id, terms) in query_groups.iter().enumerate() {
        for term in terms {
            for posting in index.postings_for(term) {
                let builder = result_builders.entry(posting.document_id).or_default();
                if builder.matched_terms.insert(term.clone()) {
                    builder.score += posting.term_frequency;
                }
                builder.matched_groups.insert(group_id);
                builder.lines.extend(&posting.lines);
            }
        }
    }

    let mut results: Vec<SearchResult> = result_builders
        .into_iter()
        .filter(|(_, builder)| {
            mode == MatchMode::Any || builder.matched_groups.len() == query_groups.len()
        })
        .map(|(document_id, builder)| SearchResult {
            document_id,
            score: builder.score,
            matched_terms: builder.matched_terms.into_iter().collect(),
            line: *builder
                .lines
                .first()
                .expect("a posting match must contain a source line"),
        })
        .collect();

    results.sort_by(|left, right| compare_results(index, left, right));
    results
}

/// This returns one trimmed source line for display as a compact search snippet.
/// Parameters: content is the complete source text and line is one-based.
/// Returns: The requested line without surrounding whitespace, or None when absent.
pub(crate) fn snippet_for(content: &str, line: usize) -> Option<&str> {
    line.checked_sub(1)
        .and_then(|line_index| content.lines().nth(line_index))
        .map(str::trim)
}

/// This groups normalized variants by their original source-query position.
/// Parameters: query is raw text accepted by the search command.
/// Returns: Unique normalized variants per source token in query order.
fn query_groups(query: &str) -> Vec<Vec<String>> {
    let mut groups: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();

    for token in tokenize(query).tokens {
        groups.entry(token.position).or_default().insert(token.term);
    }

    groups
        .into_values()
        .map(|terms| terms.into_iter().collect())
        .collect()
}

/// This orders higher scores first and resolves equal scores by repository path.
/// Parameters: index resolves document IDs while left and right are compared results.
/// Returns: The deterministic ordering used for terminal search output.
fn compare_results(index: &SearchIndex, left: &SearchResult, right: &SearchResult) -> Ordering {
    right.score.cmp(&left.score).then_with(|| {
        index.documents[left.document_id]
            .relative_path
            .cmp(&index.documents[right.document_id].relative_path)
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::classifier::FileCategory;
    use crate::scanner::ScannedDocument;
    use crate::tokenizer::tokenize;

    use super::*;

    /// This creates deterministic scanner output for query tests without file-system access.
    /// Parameters: path identifies the fixture and content supplies searchable terms.
    /// Returns: A source-code document ready for in-memory indexing.
    fn scanned_document(path: &str, content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);

        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            relative_path: PathBuf::from(path),
            category: FileCategory::SourceCode,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    /// This verifies OR retrieval, summed term-frequency ranking, and path tie-breaking.
    /// Parameters: none. The test searches three fixed indexed documents.
    /// Returns: Nothing. The test panics if result order, scores, or terms differ.
    #[test]
    fn ranks_any_term_matches_by_frequency_then_path() {
        let documents = vec![
            scanned_document("src/a.rs", "search search index"),
            scanned_document("src/b.rs", "ranking search"),
            scanned_document("src/c.rs", "ranking"),
        ];
        let index = SearchIndex::build(&documents);

        let results = search(&index, "search ranking", MatchMode::Any);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].document_id, 0);
        assert_eq!(results[0].score, 2);
        assert_eq!(results[0].matched_terms, ["search"]);
        assert_eq!(results[1].document_id, 1);
        assert_eq!(results[1].score, 2);
        assert_eq!(results[1].matched_terms, ["ranking", "search"]);
        assert_eq!(results[2].document_id, 2);
        assert_eq!(results[2].score, 1);

        let repeated_results = search(&index, "search search", MatchMode::Any);
        assert_eq!(repeated_results[0].score, 2);
    }

    /// This verifies AND matching across source query tokens and line selection.
    /// Parameters: none. The test uses one partial and one complete document match.
    /// Returns: Nothing. The test panics unless only the all-term match is returned.
    #[test]
    fn requires_every_query_group_in_all_mode() {
        let documents = vec![
            scanned_document("src/a.rs", "search only"),
            scanned_document("src/b.rs", "ranking\nsearch"),
        ];
        let index = SearchIndex::build(&documents);

        let results = search(&index, "search ranking", MatchMode::All);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, 1);
        assert_eq!(results[0].score, 2);
        assert_eq!(results[0].line, 1);
    }

    /// This verifies that identifier variants are alternatives within one query token.
    /// Parameters: none. The query uses camel case while the document uses prose terms.
    /// Returns: Nothing. The test panics if component terms cannot satisfy the query group.
    #[test]
    fn matches_identifier_variants_within_one_query_group() {
        let documents = vec![scanned_document("src/index.rs", "search index")];
        let index = SearchIndex::build(&documents);

        let results = search(&index, "SearchIndex", MatchMode::All);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matched_terms, ["index", "search"]);
        assert_eq!(results[0].score, 2);
    }

    /// This verifies empty-query handling and one-based trimmed snippet extraction.
    /// Parameters: none. The test uses whitespace-only input and fixed multiline content.
    /// Returns: Nothing. The test panics if empty input matches or line lookup is incorrect.
    #[test]
    fn handles_empty_queries_and_line_snippets() {
        let index = SearchIndex::build(&[]);

        assert!(search(&index, "  --  ", MatchMode::Any).is_empty());
        assert_eq!(
            snippet_for("first\n    matched line\nthird", 2),
            Some("matched line")
        );
        assert_eq!(snippet_for("first", 0), None);
        assert_eq!(snippet_for("first", 2), None);
    }
}
