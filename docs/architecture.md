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
Query matching and ranking
    |
    v
Terminal report
```

A search scans the selected repository and builds an index in memory. Persistent index storage is planned for a later milestone.

## Modules

| File                | Responsibility                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| `src/main.rs`       | Parses arguments, dispatches commands, and writes rendered reports.                                   |
| `src/cli.rs`        | Defines the `index` and `search` commands with Clap.                                                  |
| `src/classifier.rs` | Assigns repository roles such as source, documentation, configuration, tests, examples, and metadata. |
| `src/scanner.rs`    | Walks the repository, filters files and directories, reads UTF-8 content, and invokes the tokenizer.  |
| `src/tokenizer.rs`  | Normalizes prose and technical identifiers into positioned terms.                                     |
| `src/index.rs`      | Assigns document IDs and builds term postings and corpus statistics.                                  |
| `src/search.rs`     | Processes query terms, applies OR or AND matching, ranks documents, and selects snippet lines.        |
| `src/terminal.rs`   | Renders startup, index, and `term-table` search output with optional ANSI colors.                     |

## Data Model

`ScannedDocument` stores paths, category, file content, source token count, and normalized terms. Content is retained so snippets use the same text that was indexed.

`Token` stores a normalized term, zero-based source position, and one-based source line. Variants from one identifier share a position.

`IndexedDocument` stores the stable document ID, relative path, category, and source token count.

`Posting` connects one term to one document. It stores term frequency, positions, and source lines.

`SearchIndex` stores documents, term postings, document frequency, total source tokens, and average document length. `BTreeMap` keeps term iteration ordered.

## Query Processing

Queries use the same tokenizer as repository files. Variants from one source query token form a group. A camel-case query such as `SearchIndex` can match the complete term or its `search` and `index` components.

OR mode accepts documents that match any source query group. `--match-all` accepts only documents that match every source query group.

The current score is the sum of unique matching-term frequencies in each document. Equal scores are ordered by repository path. BM25, path boosts, category boosts, grouped results, and score explanations are planned for Milestone 6.

## Snippets

Each result uses the earliest matched source line. The line is read from retained scanner content, which avoids a second file read after indexing.

## Planned Changes

Future milestones add BM25 ranking, grouped results, Rust symbols, related-file discovery, documentation checks, and index persistence. See [Roadmap](roadmap.md) for milestone status.
