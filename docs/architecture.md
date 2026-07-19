# Architecture

This document describes Shun's modules, data flow, and current search model.

## Processing Flow

```text
Repository
    |
    v
Scanner and exclusions
    |
    v
File classification
    |
    v
Tokenization
    |
    v
In-memory inverted index
    |
    v
Text ranking, symbol lookup, or project analysis
    |
    v
Terminal report
```

A search scans the selected repository and builds an index in memory. Persistent index storage is planned for a later milestone.

## Modules

| File                | Responsibility                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/main.rs`       | Parses arguments, dispatches commands, and writes rendered reports.                                   |
| `src/cli.rs`        | Defines the `index`, `search`, `symbol`, and `overview` commands with Clap.                           |
| `src/classifier.rs` | Assigns repository roles such as source, documentation, configuration, tests, examples, and metadata. |
| `src/scanner.rs`    | Walks the repository, filters files and directories, reads UTF-8 content, and invokes the tokenizer.  |
| `src/tokenizer.rs`  | Normalizes prose and technical identifiers into positioned terms.                                     |
| `src/index.rs`      | Assigns document IDs and owns term postings, corpus statistics, and the shared symbol index.          |
| `src/search.rs`     | Processes query terms, applies OR or AND matching, ranks documents, and selects snippet lines.        |
| `src/symbol.rs`     | Parses Rust syntax, extracts symbols, and finds exact definitions and likely text references.         |
| `src/overview.rs`   | Detects Rust project structure, repository roles, inline tests, and entry-point dependencies.         |
| `src/terminal.rs`   | Renders startup, help, index, search, symbol, and overview tables with optional ANSI colors.          |

## Data Model

`ScannedDocument` stores paths, category, file content, source token count, and normalized terms. Content is retained so snippets use the same text that was indexed.

`Token` stores a normalized term, zero-based source position, and one-based source line. Variants from one identifier share a position.

`IndexedDocument` stores the stable document ID, relative path, category, and source token count.

`Posting` connects one term to one document. It stores term frequency, positions, and source lines.

`SearchIndex` stores documents, term postings, document frequency, total source tokens, average document length, and a shared `SymbolIndex`. `BTreeMap` keeps term iteration ordered.

`SymbolIndex` stores Rust definitions and recoverable parse failures. Each symbol records its name, kind, document, source line, visibility, and optional parent.

`ProjectOverview` stores detected project kind, package name, entry points, core modules, configuration, tests, examples, likely workflow, and an optional Cargo manifest warning.

`SearchFilters` stores selected categories and extensions, optional path text, and an optional result limit. Category and extension values are normalized before filtering.

## Query Processing

Queries use the same tokenizer as repository files. Variants from one source query token form a group. A camel-case query such as `SearchIndex` can match the complete term or its `search` and `index` components.

OR mode accepts documents that match any source query group. `--match-all` accepts only documents that match every source query group.

Matched documents are scored with BM25 using `k1 = 1.2` and `b = 0.75`. The calculation uses corpus size, document frequency, term frequency, document length, and average document length.

The final score adds repository-aware factors to BM25. A query-group match in the file stem adds `2.0`, and a match in the parent path adds `1.0`. An exact case-sensitive symbol name adds `4.0` once per query token and document. Fixed category priors range from `0.30` for source code to `0.0` for unknown files. Metadata and symbol factors boost documents already retrieved through indexed content rather than creating candidates by themselves.

Every result retains its BM25, file-name, path, category, and exact-symbol factors. Terminal output reports those values and groups results as implementation, documentation, configuration, tests, examples, project metadata, and other. Group order is fixed, while each displayed rank retains the global score order. Results remain score-ordered within each group. Equal scores are ordered by repository path.

Filtering runs after ranking. Values within a category or extension list use OR matching, while category, extension, and path filters combine with AND matching. Path matching ignores letter case. The result limit is applied last, preserving global score order before terminal grouping.

## Snippets

Each result uses the earliest matched source line. The line is read from retained scanner content, which avoids a second file read after indexing.

## Rust Symbols

Rust files are parsed with `syn`. A visitor extracts functions, structs, enums, traits, implementations, modules, constants, statics, type aliases, macros, and methods. Definitions retain source lines from `proc-macro2` spans, visibility, and parent modules, traits, or implementations.

Lookup is exact and case-sensitive. Likely references come from the existing text token stream, exclude definition locations, and are deduplicated by document and line. They are intentionally not described as semantic compiler references. Parse failures are retained for reporting and do not stop indexing other files.

## Project Overview

Scanned `Cargo.toml` files are parsed as structured TOML. Default and manifest-declared library and binary targets determine entry points, including targets in workspace member packages. The root manifest determines the overall project type. Existing file classification supplies configuration, metadata, integration tests, and examples. Rust source containing top-level `#[cfg(test)]` modules or test attributes such as `#[test]` and `#[tokio::test]` is also reported as test-bearing.

The likely workflow begins with `src/main.rs`, or the first detected entry point, and records direct module roots imported through `crate::...`. This is deterministic structural inference rather than call-graph analysis.

## Planned Changes

Future milestones add related-file discovery, documentation checks, and index persistence. See [Roadmap](roadmap.md) for milestone status.
