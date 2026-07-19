/* ============================================================================
 * Shun: related.rs
 * ============================================================================
 *
 * This file identifies files that are likely related to a selected repository
 * file. Scores combine text references to symbols defined by the source file,
 * file-name terms, and distinctive terms shared through the inverted index.
 *
 * ============================================================================
 */

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::documentation::Confidence;
use crate::index::{DocumentId, SearchIndex};
use crate::pathing::normalize_repository_path;
use crate::scanner::ScannedDocument;
use crate::symbol::SymbolVisibility;
use crate::tokenizer::tokenize;

const SYMBOL_WEIGHT: f64 = 5.0;
const FILE_TERM_WEIGHT: f64 = 2.0;
const MAX_SHARED_TERMS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RelatedResult {
    pub(crate) document_id: DocumentId,
    pub(crate) score: f64,
    pub(crate) confidence: Confidence,
    pub(crate) symbol_matches: Vec<String>,
    pub(crate) file_term_matches: Vec<String>,
    pub(crate) shared_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RelatedFiles {
    pub(crate) source_document_id: DocumentId,
    pub(crate) results: Vec<RelatedResult>,
}

pub(crate) fn find_related_files(
    documents: &[ScannedDocument],
    index: &SearchIndex,
    path: &Path,
) -> Option<RelatedFiles> {
    let requested_path = normalize_repository_path(path);
    let source_document_id = index
        .documents
        .iter()
        .find(|document| normalize_repository_path(&document.relative_path) == requested_path)
        .map(|document| document.id)?;
    let source_document = &documents[source_document_id];

    let symbol_terms = index
        .symbols
        .symbols
        .iter()
        .filter(|symbol| symbol.document_id == source_document_id)
        .filter(|symbol| symbol.visibility != SymbolVisibility::Private)
        .filter(|symbol| !is_generic_symbol(&symbol.name))
        .filter_map(|symbol| {
            tokenize(&symbol.name)
                .tokens
                .into_iter()
                .next()
                .map(|token| (token.term, symbol.name.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let file_terms = source_document
        .relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            tokenize(stem)
                .tokens
                .into_iter()
                .map(|token| token.term)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let source_terms = source_document
        .tokens
        .iter()
        .map(|token| token.term.as_str())
        .filter(|term| is_distinctive_term(term, index))
        .collect::<BTreeSet<_>>();

    let mut results = Vec::new();
    for (document_id, candidate) in documents.iter().enumerate() {
        if document_id == source_document_id {
            continue;
        }
        let candidate_terms = candidate
            .tokens
            .iter()
            .map(|token| token.term.as_str())
            .collect::<BTreeSet<_>>();
        let symbol_matches = symbol_terms
            .iter()
            .filter(|(term, _)| candidate_terms.contains(term.as_str()))
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>();
        let file_term_matches = file_terms
            .iter()
            .filter(|term| candidate_terms.contains(term.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let mut shared_terms = source_terms
            .intersection(&candidate_terms)
            .copied()
            .collect::<Vec<_>>();
        shared_terms.sort_by(|left, right| {
            term_weight(index, right)
                .total_cmp(&term_weight(index, left))
                .then_with(|| left.cmp(right))
        });
        shared_terms.truncate(MAX_SHARED_TERMS);

        if symbol_matches.is_empty() && file_term_matches.is_empty() && shared_terms.len() < 2 {
            continue;
        }

        let score = symbol_matches.len() as f64 * SYMBOL_WEIGHT
            + file_term_matches.len() as f64 * FILE_TERM_WEIGHT
            + shared_terms
                .iter()
                .map(|term| term_weight(index, term))
                .sum::<f64>()
            + category_weight(candidate.category);
        let confidence = if !symbol_matches.is_empty()
            && (!file_term_matches.is_empty() || shared_terms.len() >= 2)
        {
            Confidence::High
        } else if !symbol_matches.is_empty()
            || !file_term_matches.is_empty()
            || shared_terms.len() >= 3
        {
            Confidence::Medium
        } else {
            Confidence::Low
        };
        results.push(RelatedResult {
            document_id,
            score,
            confidence,
            symbol_matches,
            file_term_matches,
            shared_terms: shared_terms.into_iter().map(str::to_owned).collect(),
        });
    }

    results.sort_by(|left, right| compare_results(index, left, right));
    Some(RelatedFiles {
        source_document_id,
        results,
    })
}

fn is_distinctive_term(term: &str, index: &SearchIndex) -> bool {
    term.chars().count() >= 3
        && index
            .document_frequency
            .get(term)
            .is_some_and(|frequency| *frequency >= 2 && *frequency <= 6)
}

fn term_weight(index: &SearchIndex, term: &str) -> f64 {
    let document_count = index.documents.len() as f64;
    let document_frequency = index.document_frequency.get(term).copied().unwrap_or(1) as f64;
    (1.0 + document_count / document_frequency).ln()
}

fn category_weight(category: crate::classifier::FileCategory) -> f64 {
    use crate::classifier::FileCategory;

    match category {
        FileCategory::Test => 1.50,
        FileCategory::Documentation => 1.25,
        FileCategory::Configuration => 1.00,
        FileCategory::Example => 0.75,
        FileCategory::SourceCode => 0.50,
        FileCategory::ProjectMetadata => 0.25,
        FileCategory::Unknown => 0.0,
    }
}

fn compare_results(index: &SearchIndex, left: &RelatedResult, right: &RelatedResult) -> Ordering {
    right.score.total_cmp(&left.score).then_with(|| {
        index.documents[left.document_id]
            .relative_path
            .cmp(&index.documents[right.document_id].relative_path)
    })
}

fn is_generic_symbol(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "build" | "default" | "fmt" | "from" | "main" | "new" | "run" | "test" | "tests"
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::classifier::FileCategory;
    use crate::tokenizer::tokenize;

    use super::*;

    fn document(path: &str, category: FileCategory, content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);
        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            relative_path: PathBuf::from(path),
            category,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    #[test]
    fn ranks_tests_documentation_callers_and_configuration_by_evidence() {
        let documents = vec![
            document(
                "src/index.rs",
                FileCategory::SourceCode,
                "pub struct SearchIndex; impl SearchIndex { pub fn build_postings() {} }",
            ),
            document(
                "tests/index_tests.rs",
                FileCategory::Test,
                "SearchIndex::build_postings verifies postings",
            ),
            document(
                "docs/indexing.md",
                FileCategory::Documentation,
                "SearchIndex builds postings for repository search",
            ),
            document(
                "src/main.rs",
                FileCategory::SourceCode,
                "let index = SearchIndex; index builds postings",
            ),
            document(
                "config/index.toml",
                FileCategory::Configuration,
                "index_path = \"postings\"",
            ),
            document(
                "docs/unrelated.md",
                FileCategory::Documentation,
                "installation and terminal colors",
            ),
        ];
        let index = SearchIndex::build(&documents);

        let related = find_related_files(&documents, &index, Path::new("src/index.rs")).unwrap();

        assert_eq!(related.source_document_id, 0);
        assert_eq!(related.results.len(), 4);
        assert_eq!(related.results[0].document_id, 1);
        assert!(
            related.results[0]
                .symbol_matches
                .contains(&"SearchIndex".to_owned())
        );
        assert!(related.results.iter().any(|result| result.document_id == 2));
        assert!(related.results.iter().any(|result| result.document_id == 3));
        assert!(related.results.iter().any(|result| result.document_id == 4));
        assert!(related.results.iter().all(|result| result.document_id != 5));
    }
}
