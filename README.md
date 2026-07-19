# Shun

Shun is a Rust command-line tool for indexing and searching source code, documentation, configuration, tests, and project metadata.

Use Shun to answer questions such as:

- Where is a configuration value defined?
- Which tests cover a tokenizer or indexer?
- Where is a Rust symbol defined and likely referenced?
- Which implementation and documentation files relate to the same feature?
- Does the documentation mention paths, commands, options, or symbols that no longer exist?

Shun currently supports repository scanning, file classification, technical identifier tokenization, in-memory indexing, BM25 ranking, grouped keyword search, Rust symbol lookup, project overviews, related-file discovery, documentation audits, and score explanations.

## Current Status

Available now:

- Recursive scanning of `.rs`, `.md`, `.txt`, `.toml`, and `.json` files.
- File classification and technical identifier tokenization.
- An in-memory inverted index with source positions and line numbers.
- Multi-keyword search with OR matching and optional `--match-all` behavior.
- BM25 ranking with file-name, path, and category boosts.
- Results grouped by repository role with score factors and source-line snippets.
- Category, extension, path, and result-count filters.
- Rust symbol extraction with exact definition lookup and likely text references.
- Project type, entry point, module, configuration, test, example, and workflow summaries.
- Evidence-ranked related files grouped by repository role.
- Markdown reference audits for paths, symbols, commands, options, modules, and configuration keys.
- High, medium, and low confidence levels for potentially stale documentation.
- Bordered command, argument, and option help tables.
- Terminal colors with plain redirected output and `NO_COLOR` support.

Persistent index storage and machine-readable output are planned.

The `index` command scans repository files, builds an in-memory index, and reports corpus statistics. `search` returns ranked line-aware results, `symbol` finds exact Rust definitions and likely references, `overview` summarizes repository structure, `related` connects a file to likely companions, and `audit-docs` reports potentially stale Markdown references. The index is not persisted yet.

## Requirements

Shun uses Rust edition 2024 and should be built with a recent stable Rust toolchain.

Install Rust through [rustup](https://rustup.rs/), then verify the tools:

```powershell
rustc --version
cargo --version
```

No database, network service, or external search engine is required.

## Quick Start

From the crate directory, run Shun without a command to see its startup screen and available commands:

```powershell
cargo run
```

Build an in-memory index report for the current repository:

```powershell
cargo run -- index .
```

Search the current repository. Quote queries that contain spaces:

```powershell
cargo run -- search "search index"
```

Require every query token instead of the default match-any behavior:

```powershell
cargo run -- search "search index" --match-all
```

Find an exact Rust symbol and summarize the current repository:

```powershell
cargo run -- symbol SearchIndex
cargo run -- overview
```

Find files related to a source module and audit Markdown references:

```powershell
cargo run -- related src/index.rs
cargo run -- audit-docs --confidence high
```

`search` scans and indexes the selected repository for each invocation. You do not need to run `index` first. Neither command writes an index file yet.

## Project Documentation

- [Architecture](docs/architecture.md) describes the modules, data flow, index, and query model.
- [Maintainer Guide](docs/maintainer-guide.md) records the maintenance and validation workflow.
- [Roadmap](docs/roadmap.md) tracks completed and planned milestones.

## Building

From the crate directory:

```powershell
cd C:\example-folder
cargo build
```

Build an optimized executable with:

```powershell
cargo build --release
```

Executable locations on Windows:

```text
target\debug\shun.exe
target\release\shun.exe
```

## Running

Running Shun without a subcommand displays its identity and table-based command help:

```text
 __ _
/ _\ |__  _   _ _ __
\ \| '_ \| | | | '_ \
_\ \ | | | |_| | | | |
\__/_| |_|\__,_|_| |_|

Search code and documentation.
------------------------------------------------------------
Search repository files and check documentation references

Usage: shun [COMMAND]

+----------------------------------------------------+
|                      COMMANDS                      |
+-----------+----------------------------------------+
| index     | Build an in-memory index and report    |
|           | repository statistics                  |
+-----------+----------------------------------------+
| search    | Search repository content with         |
|           | normalized developer-aware terms       |
+-----------+----------------------------------------+
| symbol    | Find exact Rust symbol definitions and |
|           | likely text references                 |
+-----------+----------------------------------------+
| overview  | Summarize project type, entry points,  |
|           | modules, and repository roles          |
+-----------+----------------------------------------+
| related   | Find tests, documentation, callers,    |
|           | and configuration related to a file    |
+-----------+----------------------------------------+
| audit-docs| Report potentially stale references in |
|           | Markdown documentation                 |
+-----------+----------------------------------------+
| help      | Print this message or the help of the  |
|           | given subcommand(s)                    |
+-----------+----------------------------------------+
```

Display help:

```powershell
cargo run
cargo run -- --help
cargo run -- index --help
cargo run -- search --help
cargo run -- symbol --help
cargo run -- overview --help
cargo run -- related --help
cargo run -- audit-docs --help
```

Scan the current repository:

```powershell
cargo run -- index .
```

Scan another repository:

```powershell
cargo run -- index "C:\another-folder"
```

Search the current repository with default OR matching:

```powershell
cargo run -- search "inverted postings"
```

Require every source query token and search another repository:

```powershell
cargo run -- search "database timeout" --match-all --directory "C:\another-folder"
```

Run the built executable directly:

```powershell
.\target\debug\shun.exe index .
```

Install Shun into Cargo's binary directory:

```powershell
cargo install --path .
```

After installation:

```powershell
shun --help
shun index .
shun search "search index"
shun symbol SearchIndex
shun overview
shun related src/index.rs
shun audit-docs --confidence high
```

If `shun` is not recognized, ensure `%USERPROFILE%\.cargo\bin` is included in the user `PATH`.

## Current Index Command

```text
shun index <DIRECTORY>
```

The command performs these steps:

1. Validate and resolve the repository directory.
2. Scan supported files while skipping configured directories.
3. Classify and tokenize readable UTF-8 files.
4. Build the in-memory index.
5. Print file, category, and corpus statistics.

Example:

```powershell
shun index .
```

Possible output:

```text
INDEX
Repository  C:\example-folder
------------------------------------------------------------
    ID  PATH               CATEGORY          TOKENS
------------------------------------------------------------
     0  Cargo.toml         Project metadata      22
     1  README.md          Documentation      3,129
     2  src\classifier.rs  Source code          473
------------------------------------------------------------
SUMMARY
    Files              10
    Unique terms       1,472
    Posting entries    2,900
    Source tokens      8,505
    Average length     850.50 tokens

BY CATEGORY
    Source code        8
    Documentation      1
    Project metadata   1
```

Token counts change as source and documentation evolve.

## Current Search Command

```text
shun search [OPTIONS] <QUERY>
```

The command scans and indexes the selected repository in memory for each invocation. The directory defaults to the current directory. When the selected path is `.`, reports display its absolute path. Query text uses the same developer-aware normalization as indexed content, including complete technical identifiers and their snake-case, camel-case, acronym, and kebab-case components.

Default OR mode returns a document when any source query token matches. `--match-all` requires every source query token, while normalized variants from one identifier remain alternatives within that token. Results use BM25 with file-name, parent-path, category, and exact-symbol boosts. They are grouped by repository role and ordered by score within each group, with repository path used to break ties.

Shun uses BM25 parameters `k1 = 1.2` and `b = 0.75`. A normalized query-group match in the file stem adds `2.0`, while a match in the parent path adds `1.0`. An exact case-sensitive Rust symbol name adds `4.0` once per query token and document. Category priors add `0.30` for source code, `0.25` for documentation, `0.20` for configuration, `0.15` for tests, `0.10` for examples, `0.05` for project metadata, and `0.0` for unknown files. Metadata and symbol boosts only affect documents already retrieved from indexed content.

Search options:

| Option                    | Behavior                                                                   |
| ------------------------- | -------------------------------------------------------------------------- |
| `-d, --directory <PATH>`  | Search another repository. The default is the current directory.           |
| `--match-all`             | Require every source query token.                                          |
| `--category <CATEGORY>`   | Keep selected repository roles. Repeat it or use comma-separated values.   |
| `--extension <EXTENSION>` | Keep selected file extensions. A leading dot and letter case are optional. |
| `--path <TEXT>`           | Keep repository-relative paths containing text without case sensitivity.   |
| `--limit <COUNT>`         | Keep the first positive number of globally ranked results.                 |

Values within `--category` or `--extension` use OR matching. Different filter types combine with AND matching. Filters run after BM25 ranking, and `--limit` runs last. File-name and path boosts therefore use the complete repository index while the displayed results satisfy every active filter.

Canonical category values are `source-code`, `documentation`, `configuration`, `tests`, `examples`, `project-metadata`, and `unknown`. The aliases `source`, `docs`, `test`, `example`, and `metadata` are also accepted.

Examples:

```powershell
shun search "ranking" --category source-code,tests
shun search "timeout" --extension toml,json --path config
shun search "documentation" --category docs --limit 5
```

Always quote a multi-word query so the shell passes it as one argument. A query containing only punctuation, such as `"???"`, has no searchable terms. Shun reports this instead of performing a broad match.

Example:

```powershell
shun search "inverted postings"
```

Possible output:

```text
SEARCH
Query       "inverted postings"
Match       ANY term (OR)
Repository  C:\example-folder
------------------------------------------------------------

+---------------------------------------------------------------------------------+
|                            SEARCH RESULTS (2 matches)                           |
+---------------------------------------------------------------------------------+
|                               IMPLEMENTATION (1)                                |
+---------------------------------------------------------------------------------+
|                                 #1  src\index.rs                                |
+----------------------------------------+----------------------------------------+
| Line: 5                                |    Category: Source code  Score: 3.140 |
| Matches  inverted, postings                                                     |
| Factors: BM25 2.840 + file name 0.000 + path 0.000 + category 0.300             |
| Snippet: * This file builds and owns Shun's in-memory inverted index.            |
+----------------------------------------+----------------------------------------+
|                                DOCUMENTATION (1)                                |
+---------------------------------------------------------------------------------+
|                                  #2  README.md                                  |
+----------------------------------------+----------------------------------------+
| Line: 50                               |  Category: Documentation  Score: 2.370 |
| Matches  inverted, postings                                                     |
| Factors: BM25 2.120 + file name 0.000 + path 0.000 + category 0.250             |
| Snippet: - In-memory inverted index with term and document frequency.           |
+---------------------------------------------------------------------------------+
```

Search result fields:

| Field    | Meaning                                                                   |
| -------- | ------------------------------------------------------------------------- |
| `#`      | Global score rank shown within the result's repository-role group.        |
| Path     | Repository-relative file path.                                            |
| Line     | One-based source line for the first match.                                |
| Category | Repository role such as source code, documentation, or tests.             |
| Score    | BM25 plus file-name, parent-path, category, and exact-symbol boosts.      |
| Factors  | Individual values contributing to the displayed score.                    |
| Matches  | Normalized complete terms or identifier components that produced the hit. |
| Snippet  | Trimmed source line at the reported location.                             |

`No matches found.` is printed when searchable query terms do not occur in the index. `No matches satisfy the active filters.` distinguishes a filtered empty result. With `--match-all`, try fewer terms or omit the flag. With default OR matching, check spelling or use a broader technical term.

## Rust Symbol Lookup

```text
shun symbol [OPTIONS] <NAME>
```

`symbol` parses supported Rust files with `syn` and finds definitions whose names exactly match the case-sensitive query. It reports symbol kind, visibility, parent symbol, source line, and a source snippet. It also reports likely references found through normalized text tokens. These references are navigation hints, not compiler-resolved usages.

```powershell
shun symbol SearchIndex
shun symbol ProjectOverview --directory "C:\another-folder"
```

A malformed Rust file is skipped during symbol parsing and reported without preventing valid files from being searched.

## Project Overview

```text
shun overview [OPTIONS]
```

`overview` reads Cargo metadata and scanned repository roles to identify the Rust project type, package name, entry points, core modules, configuration, tests, and examples. Rust files with inline `#[cfg(test)]` modules or `#[test]` functions are included in the test section. The likely workflow starts at the primary entry point and lists its direct `crate::...` imports. It is a structural summary, not a runtime call graph.

```powershell
shun overview
shun overview --directory "C:\another-folder"
```

## Related Files

```text
shun related [OPTIONS] <PATH>
```

`related` scores other repository files using references to non-private Rust symbols defined by the selected file, normalized file-name terms, and distinctive shared index terms. Results are grouped as implementation, documentation, configuration, tests, examples, project metadata, and other. Confidence expresses evidence strength rather than compiler-resolved relationships.

```powershell
shun related src/index.rs
shun related src/search.rs --limit 10 --directory "C:\another-folder"
```

The path must be relative to the scanned repository. `--limit` defaults to 20 and applies before terminal grouping.

## Documentation Audit

```text
shun audit-docs [OPTIONS]
```

`audit-docs` parses Markdown links, inline code, and fenced code with `pulldown-cmark`. It compares code-like references with scanned repository paths, exact Rust symbols, Clap commands and options, and keys parsed from TOML and JSON. Findings mean potentially stale documentation and require developer review.

High confidence covers unresolved paths, Shun commands, and Shun options. Medium confidence covers unresolved Rust symbols and modules. Low confidence covers conservative snake-style configuration assignments. Generated paths and lines explicitly marked planned, historical, or deferred are ignored to reduce false positives.

```powershell
shun audit-docs
shun audit-docs --confidence high
shun audit-docs --directory "C:\another-folder" --confidence medium
```

The confidence option is a minimum threshold. `high` reports only high-confidence findings, `medium` includes high and medium, and the default `low` includes all findings.

## Terminal Display

Startup, command help, and search results use bordered ASCII tables with word-aware wrapping. Colors distinguish headings, paths, categories, scores, and matched terms in an interactive terminal. They are disabled when standard output is redirected, which keeps files and pipelines free of ANSI escape sequences.

Set the conventional `NO_COLOR` environment variable to disable colors explicitly:

```powershell
$env:NO_COLOR = "1"
shun search "search index"
Remove-Item Env:NO_COLOR
```

Spacing, separators, labels, and result ordering remain the same with or without color.

## Supported Files

| Extension                   | Role                       | Current behavior                     |
| --------------------------- | -------------------------- | ------------------------------------ |
| `.rs`                       | Rust source and tests      | Tokenized and parsed for symbols.    |
| `.md`                       | Documentation              | Tokenized and parsed for references. |
| `.txt`                      | Documentation or notes     | Read as UTF-8 and tokenized as text. |
| `.toml`                     | Configuration and metadata | Read as UTF-8 and tokenized as text. |
| `.json`                     | Configuration and metadata | Read as UTF-8 and tokenized as text. |
| `.yaml`, `.yml`             | Configuration              | Planned.                             |
| `.py`, `.js`, `.ts`, `.tsx` | Source code                | Optional later support.              |
| `.pdf`, `.docx`             | Binary documents           | Outside the initial scope.           |

Extensions are matched case-insensitively.

## Ignored Directories

The scanner currently prunes these directory names before descending into them:

```text
.git
target
node_modules
dist
build
coverage
.idea
.vscode
```

This prevents generated output, dependency caches, version-control internals, coverage reports, and editor settings from polluting search results.

Planned improvements include `.gitignore`-aware traversal, configurable exclusions, and a `.shunignore` file.

## Tokenization

The developer-aware tokenizer:

- Keeps Unicode letters and numbers.
- Keeps underscores and internal hyphens in complete technical identifiers.
- Treats other punctuation and whitespace as separators.
- Removes empty terms.
- Converts complete identifiers and components to lowercase.
- Preserves complete snake-case and kebab-case identifiers.
- Splits snake case, camel case, acronym boundaries, and kebab case.
- Deduplicates variants generated from the same source token.
- Records a zero-based source position and one-based line for every variant.

Example input:

```text
Rust, Search-engine_v2!
```

Current output:

```text
rust
search-engine_v2
search
engine
v2
```

For example:

```text
build_search_index
```

will produce:

```text
build_search_index
build
search
index
```

Similarly, `SearchIndexBuilder` will produce the complete normalized identifier plus `search`, `index`, and `builder`.

All variants from one source identifier share a position. This allows `build_search_index` to remain one source token for document-length calculations while exposing four searchable terms. These source positions will support phrase matching, and line numbers will support code-aware snippets.

## Error Handling

Root-path errors stop the command:

- The root does not exist.
- The root is not a directory.
- The root cannot be resolved to an absolute path.

Descendant errors are recoverable:

- A directory entry cannot be accessed.
- A supported file cannot be opened.
- A supported file is not valid UTF-8.

Recoverable errors are printed to standard error and scanning continues. One problematic file should not prevent the rest of a repository from being processed.

## Command Status

| Command                         | Purpose                                                       | Status       |
| ------------------------------- | ------------------------------------------------------------- | ------------ |
| `shun index <path>`             | Scan a repository and report index statistics.                | Implemented. |
| `shun search [options] <query>` | Search and filter repository files.                           | Implemented. |
| `shun symbol <name>`            | Find a Rust symbol definition and likely references.          | Implemented. |
| `shun overview`                 | Summarize project structure and likely workflow.              | Implemented. |
| `shun related <path>`           | Find likely tests, documentation, callers, and configuration. | Implemented. |
| `shun audit-docs`               | Report potentially stale documentation references.            | Implemented. |
| `shun stats`                    | Display index and category statistics.                        | Planned.     |
| `shun clear`                    | Remove persisted index data.                                  | Planned.     |

## Testing

Run all tests:

```powershell
cargo test
```

Check compilation:

```powershell
cargo check
```

Check formatting:

```powershell
cargo fmt -- --check
```

Tests cover CLI parsing, help routing, filtering, classification, tokenization, source locations, scanning, indexing, BM25 ranking, boosts, grouping, snippets, Rust symbols, likely references, project overview detection, documentation audits, related-file ranking, terminal reports, count formatting, and color checks.

A normal local verification sequence is:

```powershell
cargo fmt -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

## License

No license file is currently present. Will add one before distributing Shun.
