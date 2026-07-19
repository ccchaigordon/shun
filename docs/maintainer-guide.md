# Maintainer Guide

This repository is not accepting public contributions at this time. This document records the workflow used by the project maintainer.

## Branch Workflow

1. Update `main` from `origin/main`.
2. Create a branch for one feature or fix.
3. Implement and test one defined change.
4. Use a conventional commit prefix such as `feat:`, `fix:`, `test:`, `docs:`, or `chore:`.
5. Push the branch and open a pull request into `main`.
6. Merge after review and a passing `Rust validation` check.

Example branch names:

```text
feat/file-classification
feat/identifier-tokenization
fix/scanner-path-handling
docs/update-user-guide
```

## Validation

Run the same commands used by CI:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

For command changes, also run the affected command against this repository and inspect its plain output.

## Continuous Integration

Pull requests into `main` run the `Rust validation` workflow. Branch protection should require that check before merge.

The workflow verifies formatting, Clippy warnings, and the full test suite with the lockfile.

## Dependencies

| Crate            | Purpose                                    |
| ---------------- | ------------------------------------------ |
| `anyhow`         | Application errors and context.            |
| `clap`           | CLI parsing and generated help.            |
| `proc-macro2`    | Rust syntax span line locations.           |
| `pulldown-cmark` | Structured Markdown event parsing.         |
| `serde_json`     | Structured JSON configuration parsing.     |
| `syn`            | Rust syntax parsing and symbol extraction. |
| `term-table`     | Bordered terminal report tables.           |
| `textwrap`       | Word-aware snippet wrapping.               |
| `toml`           | Structured Cargo manifest parsing.         |
| `walkdir`        | Repository traversal.                      |
| `tempfile`       | Temporary repository fixtures in tests.    |

Add a dependency only when the milestone that needs it begins.

## Documentation

Keep `README.md` focused on setup and command use. Put implementation notes in `docs/architecture.md` and the maintainer process here.

Update terminal examples when command labels or report fields change.
