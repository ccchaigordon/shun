# Contributing to Shun

Shun is a maintainer-led project with a deliberately focused command surface. Bug reports, design proposals, and documentation corrections are welcome when they preserve the project's local, deterministic, explainable, and CI-friendly behavior. Public implementation contributions are accepted only after the maintainer agrees to the proposed scope.

## Before You Start

Search the existing issues before opening a new one. Small documentation corrections, such as typo fixes or broken-link repairs, may be submitted directly.

For code changes, new features, dependency changes, persistence-format changes, retrieval-model changes, or broader documentation restructuring, open an issue and wait for maintainer agreement before beginning work. Unsolicited implementation pull requests may be closed without review.

Do not report security vulnerabilities in a public issue. Follow the [Security Policy](SECURITY.md) instead.

## Development Setup

Shun uses Rust edition 2024 and a recent stable Rust toolchain. From the crate root, verify a clean checkout with:

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

Create a short-lived branch with a descriptive name such as:

```text
feat/chunk-extraction
fix/snapshot-validation
docs/search-lifecycle
```

## Change Expectations

Keep each change focused on one problem. Prefer existing modules, dependencies, and patterns over new abstractions. Fix root causes, preserve public behavior unless the proposal explicitly changes it, and avoid unrelated formatting or refactoring.

Contributions should:

- Add or update focused tests for changed behavior.
- Update user and architecture documentation when contracts change.
- Preserve deterministic keyword search, symbol lookup, and documentation validation.
- Keep optional retrieval experiments separate from the trusted lexical and structural core.
- Report repository-relative paths and source evidence for retrieval features.
- Justify new dependencies, persisted fields, and command-line options.
- Avoid committing generated output, `.shun` snapshots, or build artifacts.

## Commits and Pull Requests

Use Conventional Commit prefixes such as `feat:`, `fix:`, `test:`, `docs:`, or `chore:`. Keep commits reviewable and avoid mixing unrelated changes.

A pull request should:

- Link the agreed issue or explain the narrowly scoped defect.
- Describe user-visible behavior and important tradeoffs.
- List the validation commands that were run.
- Include documentation and tests required by the change.
- Call out snapshot compatibility, output-schema changes, or new dependencies.
- Pass the repository's Rust validation workflow.

Review may request changes for correctness, maintainability, scope, retrieval evidence, or compatibility. Approval is not guaranteed solely because validation passes.

## Licensing

By submitting a contribution, you confirm that you have the right to provide it and agree that it will be licensed under the repository's [MIT License](LICENSE).

## Conduct

Be respectful and technical. Discuss ideas and code without personal attacks, harassment, or discriminatory language. The maintainer may moderate or close interactions that do not support constructive project work.

Repeated disruptive behavior, spam, or refusal to follow maintainer guidance may result in issues or pull requests being locked or closed.
