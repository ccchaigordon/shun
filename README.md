# Shun

Shun is a local developer knowledge search engine written in Rust. It is designed to index source code, documentation, configuration, tests, and project metadata within a software repository.

The goal is broader than finding files that contain a keyword. Shun is intended to help developers answer questions such as:

- Where is a configuration value defined?
- Which tests cover a tokenizer or indexer?
- Where is a Rust symbol defined and likely referenced?
- Which implementation and documentation files relate to the same feature?
- Does the documentation mention paths, commands, options, or symbols that no longer exist?

Shun is developed incrementally. Repository scanning, file classification, developer-aware tokenization, in-memory indexing, and basic ranked search exist today. BM25 ranking, symbol extraction, relationship discovery, and documentation auditing remain planned work.

## Project Positioning

> Search the repository, expose stale documentation, and make the codebase prove what it claims.

Shun is not intended to replace AI coding assistants such as GitHub Copilot, Cursor, Claude Code, or Codex. It is a local, deterministic repository-intelligence and documentation-audit tool that provides fast, explainable retrieval across source code, documentation, configuration, and tests.

Its core product principles are:

- **Local and private:** Core indexing, retrieval, and auditing run without uploading repository content, requiring an API key, or depending on an internet connection.
- **Deterministic:** The same repository state and query should produce the same ranked evidence, making behavior reproducible and testable.
- **Explainable:** Search results should identify their matching terms, structural boosts, category, and score contributors.
- **Exhaustive:** Repository audits should systematically check every supported file instead of relying on opportunistic context selection.
- **Fast for repeated use:** A persistent local index should avoid rescanning and model inference for routine repository questions.
- **Scriptable and CI-friendly:** Commands should support structured output and meaningful exit behavior for pull-request, pre-commit, nightly, and release checks.
- **Complementary to AI:** Shun should retrieve and validate focused repository evidence that an AI assistant can explain, summarize, or use when proposing changes.

AI tools remain better suited to code generation, broad explanations, refactoring, debugging, and intent inference. Shun owns retrieval, ranking, validation, evidence, and repository structure. This division keeps its purpose narrow, trustworthy, and useful with or without an AI model.

## Current Status

Implemented:

- Recursive repository scanning.
- Support for `.rs`, `.md`, `.txt`, `.toml`, and `.json` files.
- Case-insensitive extension matching.
- Exclusion of common generated, dependency, and editor directories.
- UTF-8 text reading.
- Lowercase Unicode-aware tokenization for prose and technical identifiers.
- Preservation and splitting of snake-case, camel-case, acronym, and kebab-case identifiers.
- Zero-based source positions and one-based line numbers for every searchable term.
- Absolute and repository-relative path metadata.
- Classification of source, documentation, configuration, tests, examples, and project metadata.
- Per-file token counts.
- Per-category file totals.
- Stable document IDs assigned from deterministic path order.
- In-memory inverted index with term and document frequency.
- Position-aware and line-aware posting lists.
- Exact normalized-term lookup.
- Total and average document-length statistics.
- Multi-keyword repository search with OR matching by default.
- Optional AND matching across source query tokens.
- Deterministic term-frequency ranking.
- Path, category, score, line number, and source-line snippets for results.
- Deterministic path-sorted output.
- Graceful handling of unreadable descendants.
- Unit tests for classification, scanning, tokenization, indexing, queries, ranking, and snippets.

Not implemented yet:

- BM25 ranking.
- Result grouping and explanations.
- Persistent index storage.
- Rust symbol extraction.
- Symbol-reference estimation.
- Project overview generation.
- Related-file discovery.
- Documentation consistency auditing.

The current `index` command scans repository files, builds an in-memory index, and reports corpus statistics. The `search` command builds the same index for a query and returns ranked line-aware results. The index is not persisted yet.

## Requirements

Shun uses Rust edition 2024 and should be built with a recent stable Rust toolchain.

Install Rust through [rustup](https://rustup.rs/), then verify the tools:

```powershell
rustc --version
cargo --version
```

No database, network service, or external search engine is required.

## Project Layout

```text
main/
|-- Cargo.toml
|-- Cargo.lock
|-- README.md
`-- src/
    |-- classifier.rs
    |-- cli.rs
    |-- index.rs
    |-- main.rs
    |-- scanner.rs
    |-- search.rs
    `-- tokenizer.rs
```

### `src/main.rs`

The application entry point. It parses command-line arguments, dispatches the selected command, and formats terminal output. Feature-specific work is delegated to modules.

### `src/cli.rs`

Defines the `index` and `search` command-line interfaces with `clap`. Future commands will include `symbol`, `overview`, `related`, `audit-docs`, `stats`, and `clear`.

### `src/classifier.rs`

Classifies repository-relative paths as source code, documentation, configuration, tests, examples, project metadata, or unknown. Directory roles take precedence over extensions so files under `tests/` and `examples/` are categorized correctly.

### `src/index.rs`

Assigns stable document IDs, builds deterministic term-to-posting mappings, stores term and document frequency, retains token positions and source lines, and calculates corpus-length statistics.

### `src/scanner.rs`

Validates the repository root, prunes ignored directories, walks supported files, reads and retains UTF-8 content, invokes the tokenizer, and records absolute and relative paths.

### `src/search.rs`

Normalizes raw queries, groups identifier variants by source query token, merges posting lists with OR or AND semantics, ranks documents by summed term frequency, resolves deterministic ties by path, and selects source lines for snippets.

### `src/tokenizer.rs`

Normalizes text into lowercase alphanumeric terms. Indexed content and search queries use the same tokenizer so matching rules remain consistent.

## Building

From the crate directory:

```powershell
cd D:\Career\rust-cli-document-search-engine\main
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

Display help:

```powershell
cargo run
cargo run -- --help
cargo run -- index --help
cargo run -- search --help
```

Scan the current repository:

```powershell
cargo run -- index .
```

Scan another repository:

```powershell
cargo run -- index "D:\Career\another-project"
```

Search the current repository with default OR matching:

```powershell
cargo run -- search "inverted postings"
```

Require every source query token and search another repository:

```powershell
cargo run -- search "database timeout" --match-all --directory "D:\Career\another-project"
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
```

If `shun` is not recognized, ensure `%USERPROFILE%\.cargo\bin` is included in the user `PATH`.

## Current Index Command

```text
shun index <DIRECTORY>
```

The command currently performs these steps:

1. Verify that the supplied path exists.
2. Verify that the path is a directory.
3. Resolve the root to an absolute path.
4. Recursively walk repository descendants.
5. Prune configured generated and editor directories.
6. Ignore unsupported file formats.
7. Read supported files as UTF-8 text.
8. Skip unreadable or invalid UTF-8 descendants.
9. Tokenize readable content.
10. Classify each file by its repository role.
11. Store absolute paths, relative paths, categories, and token counts.
12. Sort results by repository-relative path.
13. Assign stable document IDs from the sorted file order.
14. Build term postings and document-frequency statistics.
15. Calculate total and average source-document length.
16. Print discovered files, category totals, and index statistics.

Example:

```powershell
shun index .
```

Possible output:

```text
     0  Cargo.toml [Project metadata]: 22 tokens
     1  README.md [Documentation]: 2580 tokens
     2  src\classifier.rs [Source code]: 473 tokens
     3  src\cli.rs [Source code]: 100 tokens
     4  src\index.rs [Source code]: 616 tokens

Indexed 8 files.
    Source code: 6
    Documentation: 1
    Project metadata: 1
Unique terms: 1218
Posting entries: 2090
Total source tokens: 5569
Average document length: 696.12 tokens
```

Token counts change as source and documentation evolve.

## Current Search Command

```text
shun search <QUERY> [--directory <DIRECTORY>] [--match-all]
```

The command scans and indexes the selected repository in memory for each invocation. The directory defaults to the current directory. Query text uses the same developer-aware normalization as indexed content, including complete technical identifiers and their snake-case, camel-case, acronym, and kebab-case components.

Default OR mode returns a document when any source query token matches. `--match-all` requires every source query token, while normalized variants from one identifier remain alternatives within that token. Results are ordered by summed term frequency, with repository path as the deterministic tie-breaker.

Example:

```powershell
shun search "inverted postings"
```

Possible output:

```text
1. src\index.rs:5 [Source code] (score: 20)
    * This file builds and owns Shun's in-memory inverted index. It assigns stable
2. README.md:50 [Documentation] (score: 16)
    - In-memory inverted index with term and document frequency.
```

Each result identifies the repository-relative path, one-based source line, file category, current term-frequency score, and trimmed matching line. `No matches found.` is printed when no normalized query term occurs in the index.

## Supported Files

| Extension                   | Role                       | Current behavior                     |
| --------------------------- | -------------------------- | ------------------------------------ |
| `.rs`                       | Rust source and tests      | Read as UTF-8 and tokenized as text. |
| `.md`                       | Documentation              | Read as UTF-8 and tokenized as text. |
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

## Current Data Model

The scanner currently records:

```rust
struct ScannedDocument {
    absolute_path: PathBuf,
    relative_path: PathBuf,
    category: FileCategory,
    content: String,
    token_count: usize,
    tokens: Vec<Token>,
}

struct Token {
    term: String,
    position: usize,
    line: usize,
}

struct IndexedDocument {
    id: DocumentId,
    relative_path: PathBuf,
    category: FileCategory,
    token_count: usize,
}

struct Posting {
    document_id: DocumentId,
    term_frequency: usize,
    positions: Vec<usize>,
    lines: Vec<usize>,
}

struct SearchIndex {
    documents: Vec<IndexedDocument>,
    postings: BTreeMap<String, Vec<Posting>>,
    document_frequency: BTreeMap<String, usize>,
    total_token_count: usize,
    average_document_length: f64,
}
```

`content` retains the exact UTF-8 text used for tokenization so snippets do not require a second file read. `token_count` counts original source tokens. `tokens` contains the complete normalized terms and any generated identifier components, so its length may be larger.

Each posting represents one term in one document. `term_frequency` counts source-token occurrences, while `positions` and `lines` preserve every occurrence location. `BTreeMap` keeps term iteration deterministic. A term's document frequency equals the number of postings in its posting list and is also stored explicitly for later ranking.

Planned document metadata includes:

- File name and extension.
- File size and line count.
- Modification time or content hash.
- Extracted Rust symbols.

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

## Planned Architecture

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
Text and identifier tokenization
    |
    v
Document metadata and Rust symbols
    |
    v
Inverted index and persistent storage
    |
    v
Query processing and BM25 ranking
    |
    v
Grouped snippets and result explanations
    |
    +--> Project overview
    +--> Related-file discovery
    `--> Documentation consistency audit
```

### File Classification

Each file will be assigned a category such as:

```rust
enum FileCategory {
    SourceCode,
    Documentation,
    Configuration,
    Test,
    Example,
    ProjectMetadata,
    Unknown,
}
```

Classification will use extension, file name, directory name, and simple repository conventions. For example, `src/index.rs` is source code, `tests/index_tests.rs` is a test, and `Cargo.toml` is project metadata.

### Inverted Index

The implemented in-memory index maps normalized terms to postings:

```rust
type DocumentId = usize;

type InvertedIndex = BTreeMap<String, Vec<Posting>>;

struct Posting {
    document_id: DocumentId,
    term_frequency: usize,
    positions: Vec<usize>,
    lines: Vec<usize>,
}
```

Positions support future phrase matching. Line numbers support code-aware snippets and terminal navigation.
Exact normalized terms can be retrieved directly. Persistent serialization is deferred to the persistence milestone.

### Query Processing

Queries use the same normalization rules as repository content. The implemented modes include:

- OR matching by default.
- Optional AND matching with `--match-all`.
- Term-frequency score display.
- Deterministic path tie-breaking.
- One-line snippets from the earliest matching source line.

Result limits and category filters remain planned CLI improvements.

### Ranking

The current ranking baseline sums matching-term frequency within each document. Milestone 6 will replace this with BM25 and developer-specific boosts for:

- Exact symbol-name matches.
- File-name matches.
- Path matches.
- Documentation-heading matches.
- Relevant file categories.

Results should explain important score contributors instead of presenting an unexplained number.

### Rust Symbol Extraction

Rust source files will eventually be parsed with `syn` to extract functions, structs, enums, traits, implementations, modules, constants, static values, type aliases, and macros.

Text occurrences of known names may be reported as likely references. They will not be described as compiler-accurate references.

### Grouped Results

Search output will group results by developer role:

```text
Definitions
Implementation
Documentation
Configuration
Tests
Examples
Other
```

Each result will include a path, line-aware snippet, category, score when requested, and an explanation of why it matched.

## Standout Feature: Documentation Consistency Audit

The planned `audit-docs` command will compare code-like references in documentation against indexed repository facts.

It will look for likely stale references such as:

- Missing file paths.
- Missing Rust symbols.
- Missing CLI commands.
- Missing CLI options.
- Renamed modules.
- Outdated configuration keys.

The audit will report potential issues rather than claiming that documentation is definitely wrong. Results will use confidence levels:

- High: an exact path or command reference no longer exists.
- Medium: an exact symbol is missing but a similar name exists.
- Low: a conceptual phrase may refer to code but cannot be verified reliably.

False-positive reduction will prioritize inline code, explicit paths, names ending in `()`, and options beginning with `--`. Ignore rules will be supported later.

## Planned Commands

| Command               | Purpose                                                       | Status                                    |
| --------------------- | ------------------------------------------------------------- | ----------------------------------------- |
| `shun index <path>`   | Scan and eventually persist a repository index.               | Scanner implemented; persistence planned. |
| `shun search <query>` | Search indexed repository knowledge.                          | Planned.                                  |
| `shun symbol <name>`  | Find a Rust symbol definition and likely references.          | Planned.                                  |
| `shun overview`       | Summarize project structure and likely workflow.              | Planned.                                  |
| `shun related <path>` | Find likely tests, documentation, callers, and configuration. | Planned.                                  |
| `shun audit-docs`     | Report potentially stale documentation references.            | Planned.                                  |
| `shun stats`          | Display index and category statistics.                        | Planned.                                  |
| `shun clear`          | Remove persisted index data.                                  | Planned.                                  |

## Development Roadmap

### Milestone 1: Repository Scanner

Status: completed for the initial formats and built-in exclusions.

- Scan recursively.
- Support `.rs`, `.md`, `.txt`, `.toml`, and `.json`.
- Ignore common generated directories.
- Record absolute and relative paths.
- Report skipped files without crashing.

### Milestone 2: File Classification

Status: completed.

- Add `FileCategory`.
- Classify source, documentation, configuration, tests, examples, and metadata.
- Display category counts.

### Milestone 3: Developer-Aware Tokenization

Status: completed.

- Preserve complete identifiers.
- Split snake case, camel case, and kebab case.
- Track term positions and source lines.

### Milestone 4: Inverted Index

Status: completed.

- Assign document IDs.
- Store term and document frequency.
- Store line-aware postings.
- Support exact term lookup.

### Milestone 5: Basic Search and Snippets

Status: completed.

- Add multi-keyword queries.
- Support OR and AND matching.
- Display paths and line-aware snippets.
- Rank initially by term frequency.

### Milestone 6: BM25 and Grouped Results

Status: next.

- Implement BM25.
- Add file-name, path, category, and later symbol boosts.
- Group results by developer role.
- Explain important match factors.

### Milestone 7: Rust Symbols and Overview

- Parse Rust source with `syn`.
- Add symbol lookup.
- Detect project type and entry points.
- Summarize modules, configuration, tests, and likely workflow.

### Milestone 8: Documentation Audit and Related Files

- Extract references from Markdown.
- Validate paths, symbols, commands, options, and configuration keys.
- Report confidence levels.
- Match source files with likely tests, documentation, callers, and configuration.

### Milestone 9: Persistence and CLI Polish

- Save and load index data.
- Add statistics and clear commands.
- Add configurable exclusions.
- Add JSON output and improved terminal formatting.

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

Current tests cover file classification, identifier-aware tokenization, source positions, line tracking, Unicode identifiers, repository scanning and retained content, exact term lookup, posting construction, document frequency, corpus statistics, OR and AND queries, identifier query groups, deterministic term-frequency ranking, and line snippets. Future tests will cover BM25, symbol extraction, reference extraction, and documentation-audit confidence.

A normal local verification sequence is:

```powershell
cargo fmt -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

## Contribution Workflow

Major features and unrelated changes should be developed on dedicated branches rather than committed directly to `main`.

Recommended branch names use a short change type and topic:

```text
feat/file-classification
feat/identifier-tokenization
fix/scanner-path-handling
docs/update-architecture
```

The expected workflow is:

1. Update local `main` from `origin/main`.
2. Create a focused feature or fix branch.
3. Implement and validate one coherent milestone or change.
4. Commit with a conventional prefix such as `feat:`, `fix:`, `test:`, `docs:`, or `chore:`.
5. Push the branch and open a pull request into `main`.
6. Merge only after review and the `Rust validation` check passes.

Pull requests targeting `main` automatically run:

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `cargo test --locked --all-features`

Configure the repository's branch protection rule to require the `Rust validation` status check before merging.

## Dependencies

| Crate      | Purpose                                 |
| ---------- | --------------------------------------- |
| `anyhow`   | Application-level errors and context.   |
| `clap`     | Typed CLI parsing and help generation.  |
| `walkdir`  | Recursive repository traversal.         |
| `tempfile` | Temporary repository fixtures in tests. |

Likely future dependencies include `serde`, `serde_json`, `bincode`, `ignore`, `syn`, `regex`, and `strsim`. Dependencies will be added only when the corresponding milestone needs them.

## Scope and Non-Goals

Shun is not intended to become an IDE, compiler, or enterprise search service in its first major version. The following are outside the initial scope:

- Language Server Protocol implementation.
- Compiler-accurate reference resolution.
- Multi-language AST support.
- Distributed or cloud indexing.
- User accounts and authentication.
- Web crawling and internet search.
- Full static or runtime analysis.
- Automated code modification or bug fixing.
- Graphical desktop or web interfaces.
- Large-scale retrieval-augmented generation infrastructure.

Semantic and hybrid search may be explored only after deterministic lexical search, ranking, and documentation auditing are reliable.

## Design Reference: Toshi

[Toshi](https://github.com/toshi-search/Toshi) is a larger Rust full-text search engine project with service-oriented indexing and query capabilities. Shun uses it only as a high-level reference for documenting build requirements, architecture, usage, queries, and testing.

Shun is not a fork of Toshi and does not depend on Toshi. Shun remains a local CLI focused on understanding developer repositories, while Toshi is designed as a broader full-text search service.

## License

No license file is currently present. Add a license before distributing Shun or accepting external contributions.
