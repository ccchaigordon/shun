# Roadmap

This document tracks Shun's implementation milestones.

## Milestone 1: Repository Scanner

Status: completed for the initial formats and built-in exclusions.

- Scan directories recursively
- Support `.rs`, `.md`, `.txt`, `.toml`, and `.json`
- Ignore common generated directories
- Record absolute and relative paths
- Skip unreadable descendant files

## Milestone 2: File Classification

Status: completed.

- Classify source, documentation, configuration, tests, examples, and metadata
- Display category counts

## Milestone 3: Developer-Aware Tokenization

Status: completed.

- Preserve complete identifiers
- Split snake case, camel case, acronym boundaries, and kebab case
- Track source positions and lines

## Milestone 4: Inverted Index

Status: completed.

- Assign document IDs
- Store term and document frequency
- Store line-aware postings
- Support exact term lookup

## Milestone 5: Basic Search and Snippets

Status: completed.

- Add multi-keyword queries
- Support OR and AND matching
- Display paths and source-line snippets
- Rank by term frequency
- Format terminal reports and support `NO_COLOR`

## Milestone 6: BM25 and Grouped Results

Status: completed.

- Implement BM25
- Add file-name, path, and category boosts
- Group results by repository role
- Report score factors
- Filter results by category, extension, path, and result count
- Defer exact symbol boosts until symbols are indexed

## Milestone 7: Rust Symbols and Overview

Status: completed.

- Parse Rust source with `syn`
- Add symbol lookup
- Detect project type and entry points
- Summarize modules, configuration, tests, and workflow
- Report likely textual references separately from exact definitions
- Boost exact symbol-name search matches

## Milestone 8: Documentation Checks and Related Files

Status: completed.

- Extract references from Markdown
- Validate paths, symbols, commands, options, and configuration keys
- Report confidence levels
- Match source files with likely tests, documentation, callers, and configuration
- Ignore generated paths and explicitly non-current documentation examples
- Bound and group human-readable reports by repository role

Documentation checks will look for references to missing paths, symbols, commands, options, modules, and configuration keys. Reports will distinguish exact failures from uncertain text matches.

## Milestone 9: Persistence and CLI Controls

Status: Completed.

- Save and load versioned index snapshots
- Report saved-index statistics and clear persisted data
- Merge built-in, configuration-file, and command-line exclusions
- Emit command-specific JSON and support explicit color control

---

# Future Retrieval Milestones

The original repository-search and documentation-audit roadmap is complete. The following milestones are optional future enhancements focused on conceptual retrieval and evidence-based AI integration.

## Milestone 10: Chunk-Based Retrieval Foundation

Status: planned.

- Extract source-aware chunks from Rust and Markdown files
- Use Rust symbols and Markdown headings as natural boundaries
- Add fallback line-window chunking for unsupported structures
- Preserve document IDs and one-based line ranges
- Introduce a shared retrieval-candidate representation
- Keep current file-level BM25 behavior working
- Create a small repository retrieval evaluation dataset
- Record baseline BM25 retrieval quality

This milestone establishes retrieval data and evaluation boundaries. It does not add AI-generated output.

## Milestone 11: Experimental Semantic Search

Status: planned.

- Generate embeddings for repository chunks
- Prefer a local embedding model
- Store model name, model version, vector dimensions, and chunk metadata
- Persist or rebuild chunk vectors through an explicit lifecycle
- Add an experimental semantic retrieval mode
- Return file and line evidence for every result
- Compare semantic retrieval against the BM25 baseline
- Keep semantic search optional

No specific model or inference library is selected by this milestone.

## Milestone 12: Hybrid Retrieval

Status: planned.

- Retrieve candidates independently from BM25 and semantic search
- Merge candidates using reciprocal-rank fusion or another documented normalization strategy
- Preserve exact symbol, file-name, path, and category evidence
- Add planned `keyword`, `semantic`, and `hybrid` search modes
- Explain retrieval factors in human output
- Include retrieval-mode data in JSON output
- Benchmark all modes using the evaluation dataset

## Milestone 13: Evidence-Based Repository Answers

Status: optional future work.

- Retrieve supporting repository chunks before generating an answer
- Keep answer generation separate from retrieval
- Cite repository-relative paths and line ranges
- Avoid answering when retrieved evidence is insufficient
- Keep model usage optional and configurable
- Preserve fully local non-AI functionality

This milestone must not replace deterministic symbol lookup, documentation validation, or repository search.
