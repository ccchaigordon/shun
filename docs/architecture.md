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
Versioned snapshot and in-memory inverted index
    |
    v
Text ranking, symbol lookup, project analysis, audit, or relation scoring
    |
    v
Human terminal report or structured JSON
```

`index` scans the selected repository and writes a versioned snapshot. Repository commands load saved documents and rebuild derived index structures in memory, or scan directly when no snapshot exists.

## Modules

| File                   | Responsibility                                                                                        |
| ---------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/main.rs`          | Parses arguments, loads or scans documents, dispatches commands, and selects an output format.        |
| `src/cli.rs`           | Defines repository, lifecycle, exclusion, and global output controls with Clap.                       |
| `src/classifier.rs`    | Assigns repository roles such as source, documentation, configuration, tests, examples, and metadata. |
| `src/scanner.rs`       | Walks the repository, filters files and directories, reads UTF-8 content, and invokes the tokenizer.  |
| `src/tokenizer.rs`     | Normalizes prose and technical identifiers into positioned terms.                                     |
| `src/index.rs`         | Assigns document IDs and owns term postings, corpus statistics, and the shared symbol index.          |
| `src/search.rs`        | Processes query terms, applies OR or AND matching, ranks documents, and selects snippet lines.        |
| `src/symbol.rs`        | Parses Rust syntax, extracts symbols, and finds exact definitions and likely text references.         |
| `src/overview.rs`      | Detects Rust project structure, repository roles, inline tests, and entry-point dependencies.         |
| `src/pathing.rs`       | Normalizes repository-relative paths for audit and related-file comparisons.                          |
| `src/documentation.rs` | Extracts Markdown references and validates them against repository evidence.                          |
| `src/related.rs`       | Scores likely tests, documentation, callers, and configuration for a selected file.                   |
| `src/terminal.rs`      | Renders all command reports as bounded tables with optional ANSI colors.                              |
| `src/json_output.rs`   | Builds command-specific structured JSON values for non-interactive consumers.                         |
| `src/storage.rs`       | Saves, validates, loads, and removes versioned repository snapshots under `.shun`.                    |

## Data Model

`ScannedDocument` stores paths, category, file content, source token count, and normalized terms. Content is retained so snippets use the same text that was indexed.

`Token` stores a normalized term, zero-based source position, and one-based source line. Variants from one identifier share a position.

`IndexedDocument` stores the stable document ID, relative path, category, and source token count.

`Posting` connects one term to one document. It stores term frequency, positions, and source lines.

`SearchIndex` stores documents, term postings, document frequency, total source tokens, average document length, and a shared `SymbolIndex`. `BTreeMap` keeps term iteration ordered.

`SymbolIndex` stores Rust definitions and recoverable parse failures. Each symbol records its name, kind, document, source line, visibility, and optional parent.

`ProjectOverview` stores detected project kind, package name, entry points, core modules, configuration, tests, examples, likely workflow, and an optional Cargo manifest warning.

`DocumentationReference` records a Markdown document, source line, typed reference kind, and value. `AuditFinding` adds a confidence level and unresolved-evidence reason.

`RelatedResult` records a candidate document, score, confidence, matched source symbols, file-name terms, and distinctive shared terms.

`SearchFilters` stores selected categories and extensions, optional path text, and an optional result limit. Category and extension values are normalized before filtering.

`IndexSnapshot` stores a format version, canonical repository path, Unix timestamp, merged directory exclusions, and scanned documents. Derived postings and symbols are rebuilt rather than serialized twice.

## Persistence and Exclusions

Snapshots use pretty JSON at the selected repository's **.shun/index.json** location. Saving first creates `.shun` and then replaces the snapshot. Loading rejects unsupported format versions and repository-path mismatches. Clearing removes the file and its now-empty storage directory.

The scanner merges built-in directory names, the optional root `.shun.toml` `exclude` array, and repeatable or comma-delimited `index --exclude` values. Directory names match at any depth. `.shun` is always excluded so an index cannot ingest itself.

Read-oriented commands prefer a saved snapshot for deterministic and fast repeated work. Snapshots intentionally do not track file-system mutations; explicit `index` and `clear` commands own that lifecycle.

## Output Boundary

Command computation is shared by both output modes. `terminal.rs` owns bounded human-readable reports and ANSI color policy. `json_output.rs` owns structured values and includes a command discriminator, repository context, summary data, and command-specific results. `main.rs` pretty-serializes one JSON document with a trailing newline. JSON mode always disables ANSI rendering.

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

## Documentation Audit

`pulldown-cmark` extracts Markdown link destinations, inline code, and fenced code with source offsets. References are classified as repository paths, Rust symbols or modules, Shun commands or options, and snake-style configuration assignments. Paths resolve from either the repository root or the containing document directory. Symbols use the shared `SymbolIndex`, command metadata comes from Clap, and TOML and JSON keys are parsed structurally.

Unresolved paths, commands, and options have high confidence. Missing exact symbols and modules have medium confidence. Configuration keys have low confidence because prose cannot prove their intended configuration scope. Generated paths and explicitly planned, historical, deprecated-example, or deferred lines are excluded.

## Related Files

Related-file scoring uses non-private, non-generic symbols defined in the selected file, normalized file-name terms, and distinctive shared terms from the inverted index. Symbol matches contribute the strongest factor, followed by file-name terms and inverse-document-frequency-weighted shared terms. Repository-role priors help tests, documentation, and configuration surface without creating matches by themselves.

Results are deterministic, grouped by repository role, and described as likely relationships. They are not compiler-resolved callers or guaranteed test coverage.

## Roadmap Status

All planned milestones are implemented. See [Roadmap](roadmap.md) for the delivered sequence.
