/* ============================================================================
 * Shun: search.rs
 * ============================================================================
 *
 * This file processes developer queries against Shun's in-memory inverted
 * index. What it does includes:
 *
 * 1. grouping normalized variants by source query token
 * 2. combining posting lists with OR or AND semantics
 * 3. ranking matches with BM25 and repository-aware boosts
 * 4. selecting the earliest matching source line for snippet generation.
 *
 * ============================================================================
 */

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::classifier::FileCategory;
use crate::index::{DocumentId, IndexedDocument, SearchIndex};
use crate::tokenizer::tokenize;

const BM25_K1: f64 = 1.2;
const BM25_B: f64 = 0.75;
const FILE_NAME_BOOST: f64 = 2.0;
const PATH_BOOST: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchMode {
    Any,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScoreFactors {
    pub(crate) bm25: f64,
    pub(crate) file_name_boost: f64,
    pub(crate) path_boost: f64,
    pub(crate) category_boost: f64,
}

impl ScoreFactors {
    pub(crate) fn total(self) -> f64 {
        self.bm25 + self.file_name_boost + self.path_boost + self.category_boost
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchResult {
    pub(crate) document_id: DocumentId,
    pub(crate) score: f64,
    pub(crate) score_factors: ScoreFactors,
    pub(crate) matched_terms: Vec<String>,
    pub(crate) line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SearchFilters {
    pub(crate) categories: BTreeSet<FileCategory>,
    pub(crate) path_contains: Option<String>,
    pub(crate) extensions: BTreeSet<String>,
    pub(crate) limit: Option<usize>,
}

impl SearchFilters {
    pub(crate) fn new(
        categories: impl IntoIterator<Item = FileCategory>,
        path_contains: Option<String>,
        extensions: impl IntoIterator<Item = String>,
        limit: Option<usize>,
    ) -> Self {
        Self {
            categories: categories.into_iter().collect(),
            path_contains: path_contains
                .map(|path| path.trim().to_lowercase())
                .filter(|path| !path.is_empty()),
            extensions: extensions
                .into_iter()
                .map(|extension| {
                    extension
                        .trim()
                        .trim_start_matches('.')
                        .to_ascii_lowercase()
                })
                .filter(|extension| !extension.is_empty())
                .collect(),
            limit,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.categories.is_empty()
            || self.path_contains.is_some()
            || !self.extensions.is_empty()
            || self.limit.is_some()
    }
}

#[derive(Debug, Default)]
struct ResultBuilder {
    term_frequencies: BTreeMap<String, usize>,
    matched_groups: BTreeSet<usize>,
    lines: BTreeSet<usize>,
}

/// This searches an index with normalized OR or AND matching.
/// Parameters: index contains repository postings, query is raw user text, and
/// mode selects whether any or all source query tokens must match.
/// Returns: Results ordered by descending BM25-plus-boost score and then path.
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
                builder
                    .term_frequencies
                    .insert(term.clone(), posting.term_frequency);
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
        .map(|(document_id, builder)| {
            let document = &index.documents[document_id];
            let score_factors =
                score_factors(index, document, &builder.term_frequencies, &query_groups);

            SearchResult {
                document_id,
                score: score_factors.total(),
                score_factors,
                matched_terms: builder.term_frequencies.into_keys().collect(),
                line: *builder
                    .lines
                    .first()
                    .expect("a posting match must contain a source line"),
            }
        })
        .collect();

    results.sort_by(|left, right| compare_results(index, left, right));
    results
}

pub(crate) fn filter_results(
    index: &SearchIndex,
    results: Vec<SearchResult>,
    filters: &SearchFilters,
) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(|result| {
            let document = &index.documents[result.document_id];
            let category_matches =
                filters.categories.is_empty() || filters.categories.contains(&document.category);
            let path_matches = filters.path_contains.as_ref().is_none_or(|expected| {
                document
                    .relative_path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(expected)
            });
            let extension_matches = filters.extensions.is_empty()
                || document
                    .relative_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .is_some_and(|extension| filters.extensions.contains(&extension));

            category_matches && path_matches && extension_matches
        })
        .take(filters.limit.unwrap_or(usize::MAX))
        .collect()
}

fn score_factors(
    index: &SearchIndex,
    document: &IndexedDocument,
    term_frequencies: &BTreeMap<String, usize>,
    query_groups: &[Vec<String>],
) -> ScoreFactors {
    let file_name_terms = document
        .relative_path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(normalized_terms)
        .unwrap_or_default();
    let path_terms = document
        .relative_path
        .parent()
        .map(|path| normalized_terms(&path.to_string_lossy()))
        .unwrap_or_default();

    ScoreFactors {
        bm25: term_frequencies
            .iter()
            .map(|(term, term_frequency)| bm25_term_score(index, document, term, *term_frequency))
            .sum(),
        file_name_boost: metadata_match_count(query_groups, &file_name_terms) as f64
            * FILE_NAME_BOOST,
        path_boost: metadata_match_count(query_groups, &path_terms) as f64 * PATH_BOOST,
        category_boost: category_boost(document.category),
    }
}

fn bm25_term_score(
    index: &SearchIndex,
    document: &IndexedDocument,
    term: &str,
    term_frequency: usize,
) -> f64 {
    if index.documents.is_empty() || index.average_document_length == 0.0 {
        return 0.0;
    }

    let document_count = index.documents.len() as f64;
    let document_frequency = index.document_frequency.get(term).copied().unwrap_or(0) as f64;
    let inverse_document_frequency =
        (1.0 + (document_count - document_frequency + 0.5) / (document_frequency + 0.5)).ln();
    let term_frequency = term_frequency as f64;
    let length_ratio = document.token_count as f64 / index.average_document_length;
    let frequency_weight = term_frequency * (BM25_K1 + 1.0)
        / (term_frequency + BM25_K1 * (1.0 - BM25_B + BM25_B * length_ratio));

    inverse_document_frequency * frequency_weight
}

fn normalized_terms(value: &str) -> BTreeSet<String> {
    tokenize(value)
        .tokens
        .into_iter()
        .map(|token| token.term)
        .collect()
}

fn metadata_match_count(query_groups: &[Vec<String>], metadata_terms: &BTreeSet<String>) -> usize {
    query_groups
        .iter()
        .filter(|group| group.iter().any(|term| metadata_terms.contains(term)))
        .count()
}

fn category_boost(category: FileCategory) -> f64 {
    match category {
        FileCategory::SourceCode => 0.30,
        FileCategory::Documentation => 0.25,
        FileCategory::Configuration => 0.20,
        FileCategory::Test => 0.15,
        FileCategory::Example => 0.10,
        FileCategory::ProjectMetadata => 0.05,
        FileCategory::Unknown => 0.0,
    }
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
    right.score.total_cmp(&left.score).then_with(|| {
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

    /// This verifies OR retrieval, BM25 ranking, and path tie-breaking.
    /// Parameters: none. The test searches three fixed indexed documents.
    /// Returns: Nothing. The test panics if result order, scores, or terms differ.
    #[test]
    fn ranks_any_term_matches_with_bm25() {
        let documents = vec![
            scanned_document("src/a.rs", "search search index"),
            scanned_document("src/b.rs", "ranking search"),
            scanned_document("src/c.rs", "ranking"),
        ];
        let index = SearchIndex::build(&documents);

        let results = search(&index, "search ranking", MatchMode::Any);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].document_id, 1);
        assert_eq!(results[0].matched_terms, ["ranking", "search"]);
        assert_eq!(results[1].document_id, 2);
        assert_eq!(results[1].matched_terms, ["ranking"]);
        assert_eq!(results[2].document_id, 0);
        assert_eq!(results[2].matched_terms, ["search"]);
        assert!(results[0].score > results[1].score);
        assert!(results[1].score > results[2].score);
        assert!(results.iter().all(|result| result.score_factors.bm25 > 0.0));

        let repeated_results = search(&index, "search search", MatchMode::Any);
        let single_results = search(&index, "search", MatchMode::Any);
        assert_eq!(repeated_results[0].score, single_results[0].score);
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
        assert!(results[0].score_factors.bm25 > 0.0);
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
        assert!(results[0].score_factors.bm25 > 0.0);
    }

    #[test]
    fn applies_file_name_path_and_category_boosts() {
        let mut documents = vec![
            scanned_document("src/index.rs", "index"),
            scanned_document("src/other.rs", "index"),
            scanned_document("index/guide.md", "index"),
        ];
        documents[2].category = FileCategory::Documentation;
        let index = SearchIndex::build(&documents);

        let results = search(&index, "index", MatchMode::Any);

        assert_eq!(results[0].document_id, 0);
        assert_eq!(results[0].score_factors.file_name_boost, 2.0);
        assert_eq!(results[0].score_factors.path_boost, 0.0);
        assert_eq!(results[0].score_factors.category_boost, 0.30);
        assert_eq!(results[1].document_id, 2);
        assert_eq!(results[1].score_factors.file_name_boost, 0.0);
        assert_eq!(results[1].score_factors.path_boost, 1.0);
        assert_eq!(results[1].score_factors.category_boost, 0.25);
    }

    #[test]
    fn calculates_standard_bm25_term_score() {
        let documents = vec![
            scanned_document("src/a.rs", "search"),
            scanned_document("src/b.rs", "other"),
        ];
        let index = SearchIndex::build(&documents);

        let results = search(&index, "search", MatchMode::Any);

        assert_eq!(results.len(), 1);
        assert!((results[0].score_factors.bm25 - 2.0_f64.ln()).abs() < 1e-12);
        assert!((results[0].score - (2.0_f64.ln() + 0.30)).abs() < 1e-12);
    }

    #[test]
    fn filters_ranked_results_by_metadata_and_limit() {
        let mut documents = vec![
            scanned_document("docs/guide.md", "search search"),
            scanned_document("docs/notes.txt", "search"),
            scanned_document("src/main.rs", "search search search"),
        ];
        documents[0].category = FileCategory::Documentation;
        documents[1].category = FileCategory::Documentation;
        let index = SearchIndex::build(&documents);
        let results = search(&index, "search", MatchMode::Any);
        let filters = SearchFilters::new(
            [FileCategory::Documentation],
            Some("DOCS".to_owned()),
            [".MD".to_owned()],
            Some(1),
        );

        let filtered = filter_results(&index, results, &filters);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].document_id, 0);
        assert_eq!(filters.path_contains.as_deref(), Some("docs"));
        assert_eq!(filters.extensions, ["md".to_owned()].into_iter().collect());
        assert!(filters.is_active());
        assert!(!SearchFilters::default().is_active());
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
