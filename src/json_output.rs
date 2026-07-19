/* ============================================================================
 * Shun: json_output.rs
 * ============================================================================
 *
 * This file converts command results into stable structured JSON values for
 * scripts and other non-interactive consumers.
 *
 * ============================================================================
 */

use std::path::Path;

use clap::Command as ClapCommand;
use serde_json::{Value, json};

use crate::documentation::{Confidence, DocumentationAudit};
use crate::index::SearchIndex;
use crate::overview::ProjectOverview;
use crate::related::RelatedFiles;
use crate::scanner::ScannedDocument;
use crate::search::{MatchMode, SearchFilters, SearchResult, snippet_for};
use crate::storage::IndexSnapshot;
use crate::symbol::SymbolLookup;

pub(crate) fn root(command: &ClapCommand) -> Value {
    json!({
        "command": "shun",
        "version": command.get_version(),
        "commands": command.get_subcommands().map(|subcommand| json!({
            "name": subcommand.get_name(),
            "about": subcommand.get_about().map(ToString::to_string),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn index(
    directory: &Path,
    index: &SearchIndex,
    saved_path: &Path,
    exclusions: &[String],
) -> Value {
    json!({
        "command": "index",
        "repository": directory,
        "saved_path": saved_path,
        "exclusions": exclusions,
        "summary": index_summary(index),
        "documents": index.documents.iter().map(|document| json!({
            "id": document.id,
            "path": document.relative_path,
            "category": document.category.to_string(),
            "tokens": document.token_count,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn search(
    directory: &Path,
    query: &str,
    mode: MatchMode,
    index: &SearchIndex,
    documents: &[ScannedDocument],
    results: &[SearchResult],
    filters: &SearchFilters,
) -> Value {
    json!({
        "command": "search",
        "repository": directory,
        "query": query,
        "match": match mode { MatchMode::Any => "any", MatchMode::All => "all" },
        "filters": {
            "categories": filters.categories.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "path": filters.path_contains,
            "extensions": filters.extensions,
            "limit": filters.limit,
        },
        "results": results.iter().enumerate().map(|(rank, result)| {
            let document = &index.documents[result.document_id];
            json!({
                "rank": rank + 1,
                "path": document.relative_path,
                "category": document.category.to_string(),
                "line": result.line,
                "score": result.score,
                "score_factors": {
                    "bm25": result.score_factors.bm25,
                    "file_name": result.score_factors.file_name_boost,
                    "path": result.score_factors.path_boost,
                    "exact_symbol": result.score_factors.exact_symbol_boost,
                    "category": result.score_factors.category_boost,
                },
                "matched_terms": result.matched_terms,
                "snippet": snippet_for(&documents[result.document_id].content, result.line),
            })
        }).collect::<Vec<_>>(),
    })
}

pub(crate) fn symbol(
    directory: &Path,
    lookup: &SymbolLookup,
    index: &SearchIndex,
    documents: &[ScannedDocument],
) -> Value {
    json!({
        "command": "symbol",
        "repository": directory,
        "query": lookup.query,
        "parse_failures": index.symbols.parse_failures.iter().map(|failure| json!({
            "path": failure.path,
            "message": failure.message,
        })).collect::<Vec<_>>(),
        "definitions": lookup.definitions.iter().map(|definition| json!({
            "path": index.documents[definition.document_id].relative_path,
            "line": definition.line,
            "kind": definition.kind.to_string(),
            "visibility": definition.visibility.to_string(),
            "parent": definition.parent,
            "snippet": snippet_for(&documents[definition.document_id].content, definition.line),
        })).collect::<Vec<_>>(),
        "likely_references": lookup.references.iter().map(|reference| json!({
            "path": index.documents[reference.document_id].relative_path,
            "line": reference.line,
            "snippet": snippet_for(&documents[reference.document_id].content, reference.line),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn overview(directory: &Path, overview: &ProjectOverview, index: &SearchIndex) -> Value {
    json!({
        "command": "overview",
        "repository": directory,
        "project_type": overview.project_kind.to_string(),
        "package": overview.package_name,
        "files": index.documents.len(),
        "symbols": index.symbols.symbols.len(),
        "entry_points": overview.entry_points,
        "core_modules": overview.core_modules,
        "configuration": overview.configuration,
        "tests": overview.tests,
        "examples": overview.examples,
        "likely_workflow": overview.workflow,
        "manifest_error": overview.manifest_error,
    })
}

pub(crate) fn related(directory: &Path, related: &RelatedFiles, index: &SearchIndex) -> Value {
    json!({
        "command": "related",
        "repository": directory,
        "source": index.documents[related.source_document_id].relative_path,
        "results": related.results.iter().map(|result| {
            let document = &index.documents[result.document_id];
            json!({
                "path": document.relative_path,
                "category": document.category.to_string(),
                "score": result.score,
                "confidence": result.confidence.to_string(),
                "evidence": {
                    "symbols": result.symbol_matches,
                    "file_terms": result.file_term_matches,
                    "shared_terms": result.shared_terms,
                },
            })
        }).collect::<Vec<_>>(),
    })
}

pub(crate) fn audit(
    directory: &Path,
    audit: &DocumentationAudit,
    index: &SearchIndex,
    documents: &[ScannedDocument],
    minimum_confidence: Confidence,
) -> Value {
    json!({
        "command": "audit-docs",
        "repository": directory,
        "minimum_confidence": minimum_confidence.to_string(),
        "documents_checked": audit.documents_checked,
        "references_checked": audit.references_checked,
        "findings": audit.findings.iter().map(|finding| json!({
            "path": index.documents[finding.reference.document_id].relative_path,
            "line": finding.reference.line,
            "kind": finding.reference.kind.to_string(),
            "reference": finding.reference.value,
            "confidence": finding.confidence.to_string(),
            "reason": finding.reason,
            "snippet": snippet_for(
                &documents[finding.reference.document_id].content,
                finding.reference.line,
            ),
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn stats(directory: &Path, snapshot: &IndexSnapshot, index: &SearchIndex) -> Value {
    json!({
        "command": "stats",
        "repository": directory,
        "indexed_at_unix_seconds": snapshot.indexed_at_unix_seconds,
        "exclusions": snapshot.exclusions,
        "summary": index_summary(index),
    })
}

pub(crate) fn clear(directory: &Path, removed: bool) -> Value {
    json!({
        "command": "clear",
        "repository": directory,
        "removed": removed,
    })
}

fn index_summary(index: &SearchIndex) -> Value {
    json!({
        "files": index.documents.len(),
        "symbols": index.symbols.symbols.len(),
        "unique_terms": index.postings.len(),
        "posting_entries": index.postings.values().map(Vec::len).sum::<usize>(),
        "source_tokens": index.total_token_count,
        "average_document_length": index.average_document_length,
        "categories": crate::classifier::FileCategory::ALL.iter().map(|category| {
            (category.to_string(), index.documents.iter().filter(|document| document.category == *category).count())
        }).collect::<std::collections::BTreeMap<_, _>>(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::classifier::FileCategory;
    use crate::scanner::ScannedDocument;
    use crate::tokenizer::tokenize;

    use super::*;

    #[test]
    fn renders_structured_index_json() {
        let tokenized = tokenize("search index");
        let documents = vec![ScannedDocument {
            absolute_path: PathBuf::from("/src/index.rs"),
            relative_path: PathBuf::from("src/index.rs"),
            category: FileCategory::SourceCode,
            content: "search index".to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }];
        let index = SearchIndex::build(&documents);

        let value = super::index(
            Path::new("."),
            &index,
            Path::new(".shun/index.json"),
            &["target".to_owned()],
        );

        assert_eq!(value["command"], "index");
        assert_eq!(value["summary"]["files"], 1);
        assert_eq!(value["documents"][0]["path"], "src/index.rs");
        assert_eq!(value["exclusions"][0], "target");
    }

    #[test]
    fn renders_structured_stats_and_clear_json() {
        let snapshot = IndexSnapshot {
            version: 1,
            repository: PathBuf::from("."),
            indexed_at_unix_seconds: 1_700_000_000,
            exclusions: vec!["target".to_owned()],
            documents: Vec::new(),
        };
        let index = SearchIndex::build(&snapshot.documents);

        let stats = super::stats(Path::new("."), &snapshot, &index);
        let removed = super::clear(Path::new("."), true);
        let absent = super::clear(Path::new("."), false);

        assert_eq!(stats["command"], "stats");
        assert_eq!(stats["indexed_at_unix_seconds"], 1_700_000_000_u64);
        assert_eq!(stats["exclusions"][0], "target");
        assert_eq!(stats["summary"]["files"], 0);
        assert_eq!(removed["removed"], true);
        assert_eq!(absent["removed"], false);
    }
}
