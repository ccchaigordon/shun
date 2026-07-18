/* ============================================================================
 * Shun: index.rs
 * ============================================================================
 *
 * This file builds and owns Shun's in-memory inverted index. It assigns stable
 * document IDs, converts positioned tokenizer output into postings, records
 * term and document frequency, and calculates corpus-length statistics needed
 * by later query and BM25 milestones.
 *
 * ============================================================================
 */

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::classifier::FileCategory;
use crate::scanner::ScannedDocument;

pub(crate) type DocumentId = usize;

/// This stores searchable metadata for one indexed repository file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedDocument {
    /// The stable zero-based identifier assigned from deterministic scanner order.
    pub(crate) id: DocumentId,
    /// The repository-relative path displayed in search and audit evidence.
    pub(crate) relative_path: PathBuf,
    /// The developer-facing role used for grouping and future ranking boosts.
    pub(crate) category: FileCategory,
    /// The unexpanded source-token count used for corpus-length calculations.
    pub(crate) token_count: usize,
}

/// This records one term's occurrences within one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Posting {
    /// The indexed document containing this term.
    pub(crate) document_id: DocumentId,
    /// The number of source-token positions containing this term in the document.
    pub(crate) term_frequency: usize,
    /// Zero-based source-token positions for every occurrence.
    pub(crate) positions: Vec<usize>,
    /// One-based source lines corresponding to every position.
    pub(crate) lines: Vec<usize>,
}

/// This owns indexed documents, term postings, and corpus statistics.
#[derive(Debug, PartialEq)]
pub(crate) struct SearchIndex {
    /// Indexed file metadata in document-ID order.
    pub(crate) documents: Vec<IndexedDocument>,
    /// Deterministic normalized-term mappings to document postings.
    pub(crate) postings: BTreeMap<String, Vec<Posting>>,
    /// The number of indexed documents containing each normalized term.
    pub(crate) document_frequency: BTreeMap<String, usize>,
    /// The sum of unexpanded source-token counts across all documents.
    pub(crate) total_token_count: usize,
    /// The mean unexpanded source-token count, or zero for an empty corpus.
    pub(crate) average_document_length: f64,
}

#[derive(Debug, Default)]
struct PostingBuilder {
    /// Occurrence positions collected for one term in the current document.
    positions: Vec<usize>,
    /// Source lines collected in the same order as occurrence positions.
    lines: Vec<usize>,
}

impl SearchIndex {
    /// This builds a deterministic in-memory index from path-sorted scanned documents.
    /// Parameters: scanned_documents contains repository metadata and positioned terms.
    /// Returns: An index with stable document IDs, postings, document frequencies, and
    /// corpus-length statistics.
    pub(crate) fn build(scanned_documents: &[ScannedDocument]) -> Self {
        let mut documents = Vec::with_capacity(scanned_documents.len());
        let mut postings: BTreeMap<String, Vec<Posting>> = BTreeMap::new();
        let mut total_token_count = 0;

        for (document_id, scanned_document) in scanned_documents.iter().enumerate() {
            documents.push(IndexedDocument {
                id: document_id,
                relative_path: scanned_document.relative_path.clone(),
                category: scanned_document.category,
                token_count: scanned_document.token_count,
            });
            total_token_count += scanned_document.token_count;

            let mut document_terms: BTreeMap<&str, PostingBuilder> = BTreeMap::new();
            for token in &scanned_document.tokens {
                let builder = document_terms.entry(&token.term).or_default();
                builder.positions.push(token.position);
                builder.lines.push(token.line);
            }

            for (term, builder) in document_terms {
                postings.entry(term.to_owned()).or_default().push(Posting {
                    document_id,
                    term_frequency: builder.positions.len(),
                    positions: builder.positions,
                    lines: builder.lines,
                });
            }
        }

        let document_frequency = postings
            .iter()
            .map(|(term, term_postings)| (term.clone(), term_postings.len()))
            .collect();
        let average_document_length = if documents.is_empty() {
            0.0
        } else {
            total_token_count as f64 / documents.len() as f64
        };

        Self {
            documents,
            postings,
            document_frequency,
            total_token_count,
            average_document_length,
        }
    }

    /// This retrieves the posting list for one exact normalized term.
    /// Parameters: term is the normalized complete identifier or component to find.
    /// Returns: The term's postings in document-ID order, or an empty slice when absent.
    pub(crate) fn postings_for(&self, term: &str) -> &[Posting] {
        self.postings
            .get(term)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::tokenizer::tokenize;

    use super::*;

    /// This creates scanner output for index tests without accessing the file system.
    /// Parameters: path identifies the fixture document and content is tokenized text.
    /// Returns: A source-code ScannedDocument with positioned searchable terms.
    fn scanned_document(path: &str, content: &str) -> ScannedDocument {
        let tokenized = tokenize(content);

        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            relative_path: PathBuf::from(path),
            category: FileCategory::SourceCode,
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    /// This verifies document IDs, postings, frequency, locations, exact lookup, and statistics.
    /// Parameters: none. The test builds an index from two in-memory scanner fixtures.
    /// Returns: Nothing. The test panics if any index structure or statistic differs.
    #[test]
    fn builds_postings_and_corpus_statistics() {
        let documents = vec![
            scanned_document("src/a.rs", "SearchIndex search\nsearch"),
            scanned_document("src/b.rs", "search engine"),
        ];

        let index = SearchIndex::build(&documents);

        assert_eq!(index.documents.len(), 2);
        assert_eq!(index.documents[0].id, 0);
        assert_eq!(index.documents[0].relative_path, Path::new("src/a.rs"));
        assert_eq!(index.documents[1].id, 1);
        assert_eq!(index.total_token_count, 5);
        assert_eq!(index.average_document_length, 2.5);
        assert_eq!(index.document_frequency["search"], 2);
        assert_eq!(
            index.postings_for("search"),
            [
                Posting {
                    document_id: 0,
                    term_frequency: 3,
                    positions: vec![0, 1, 2],
                    lines: vec![1, 1, 2],
                },
                Posting {
                    document_id: 1,
                    term_frequency: 1,
                    positions: vec![0],
                    lines: vec![1],
                },
            ]
        );
        assert_eq!(index.postings_for("searchindex")[0].positions, [0]);
        assert!(index.postings_for("missing").is_empty());
    }

    /// This verifies that indexing an empty corpus produces empty structures and zero statistics.
    /// Parameters: none. The test passes an empty document slice.
    /// Returns: Nothing. The test panics if the empty-index invariants differ.
    #[test]
    fn builds_an_empty_index() {
        let index = SearchIndex::build(&[]);

        assert!(index.documents.is_empty());
        assert!(index.postings.is_empty());
        assert!(index.document_frequency.is_empty());
        assert_eq!(index.total_token_count, 0);
        assert_eq!(index.average_document_length, 0.0);
    }
}
