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
- Defer exact symbol boosts until symbols are indexed

## Milestone 7: Rust Symbols and Overview

Status: next.

- Parse Rust source with `syn`
- Add symbol lookup
- Detect project type and entry points
- Summarize modules, configuration, tests, and workflow

## Milestone 8: Documentation Checks and Related Files

- Extract references from Markdown
- Validate paths, symbols, commands, options, and configuration keys
- Report confidence levels
- Match source files with likely tests, documentation, callers, and configuration

Documentation checks will look for references to missing paths, symbols, commands, options, modules, and configuration keys. Reports will distinguish exact failures from uncertain text matches.

## Milestone 9: Persistence and CLI Controls

- Save and load index data
- Add statistics and clear commands
- Add configurable exclusions
- Add JSON output and controls for non-interactive use
