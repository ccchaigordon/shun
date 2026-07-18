/* ============================================================================
 * Shun: classifier.rs
 * ============================================================================
 *
 * This file classifies supported repository files by their developer-facing
 * role. Classification uses repository-relative paths, directory conventions,
 * well-known file names, and file extensions.
 *
 * Note: These categories will later support grouped search results and
 * category-specific ranking boosts.
 *
 * ============================================================================
 */

use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum FileCategory {
    SourceCode,
    Documentation,
    Configuration,
    Test,
    Example,
    ProjectMetadata,
    Unknown,
}

impl FileCategory {
    pub(crate) const ALL: [Self; 7] = [
        Self::SourceCode,
        Self::Documentation,
        Self::Configuration,
        Self::Test,
        Self::Example,
        Self::ProjectMetadata,
        Self::Unknown,
    ];
}

impl fmt::Display for FileCategory {
    /// This formats a category as a readable terminal-output label.
    /// Parameters: formatter is the output formatter supplied by Rust's display system.
    /// Returns: A formatting result after writing the category label.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::SourceCode => "Source code",
            Self::Documentation => "Documentation",
            Self::Configuration => "Configuration",
            Self::Test => "Tests",
            Self::Example => "Examples",
            Self::ProjectMetadata => "Project metadata",
            Self::Unknown => "Unknown",
        };

        formatter.write_str(label)
    }
}

/// This assigns a developer-facing role to a repository-relative file path.
/// Parameters: path is the file path relative to the scanned repository root.
/// Returns: The category selected from path conventions, well-known file names,
/// and the file extension.
pub(crate) fn classify_file(path: &Path) -> FileCategory {
    if has_component(path, "tests") || has_test_file_name(path) {
        return FileCategory::Test;
    }
    if has_component(path, "examples") {
        return FileCategory::Example;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    if matches!(
        file_name.to_ascii_lowercase().as_str(),
        "cargo.toml" | "package.json" | "rust-toolchain.toml"
    ) {
        return FileCategory::ProjectMetadata;
    }

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("rs") => FileCategory::SourceCode,
        Some("md" | "txt") => FileCategory::Documentation,
        Some("toml" | "json" | "yaml" | "yml") => FileCategory::Configuration,
        _ => FileCategory::Unknown,
    }
}

/// This checks whether a repository path contains a named directory component.
/// Parameters: path is the repository-relative path and expected is the directory
/// name to match without case sensitivity.
/// Returns: true when any parent component matches the expected directory name.
fn has_component(path: &Path, expected: &str) -> bool {
    path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(expected))
        })
    })
}

/// This detects common test-file naming conventions outside a tests directory.
/// Parameters: path is the repository-relative path whose file stem is inspected.
/// Returns: true for stems ending in _test, _tests, .test, or .spec.
fn has_test_file_name(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|stem| {
            stem.ends_with("_test")
                || stem.ends_with("_tests")
                || stem.ends_with(".test")
                || stem.ends_with(".spec")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_repository_files_by_role() {
        let cases = [
            ("src/index.rs", FileCategory::SourceCode),
            ("tests/index_tests.rs", FileCategory::Test),
            ("src/parser_test.rs", FileCategory::Test),
            ("examples/basic.rs", FileCategory::Example),
            ("README.md", FileCategory::Documentation),
            ("docs/search.txt", FileCategory::Documentation),
            ("config/shun.toml", FileCategory::Configuration),
            ("config/settings.JSON", FileCategory::Configuration),
            ("Cargo.toml", FileCategory::ProjectMetadata),
            ("package.json", FileCategory::ProjectMetadata),
            ("assets/image.png", FileCategory::Unknown),
        ];

        for (path, expected) in cases {
            assert_eq!(classify_file(Path::new(path)), expected, "path: {path}");
        }
    }
}
