/* ============================================================================
 * Shun: scanner.rs
 * ============================================================================
 *
 * This file discovers supported repository documents and gathers metadata
 * needed for indexing. The step involves:
 *
 * validating a root directory --> walking its descendants --> filtering files
 * by extension --> reading text content --> using the shared tokenizer to
 * count normalized terms.
 *
 * Unsupported or unreadable entries are skipped. Generated and editor-owned
 * directories are pruned before traversal to avoid indexing irrelevant files.
 *
 * ============================================================================
 */

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::classifier::{FileCategory, classify_file};
use crate::tokenizer::{Token, tokenize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ScannedDocument {
    pub(crate) absolute_path: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) category: FileCategory,
    pub(crate) content: String,
    pub(crate) token_count: usize,
    pub(crate) tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScanOptions {
    exclusions: BTreeSet<String>,
}

impl ScanOptions {
    pub(crate) fn for_repository(
        root: &Path,
        exclusions: impl IntoIterator<Item = String>,
    ) -> Result<Self> {
        let mut configured = BUILT_IN_EXCLUSIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<BTreeSet<_>>();
        configured.extend(exclusions.into_iter().filter_map(normalize_exclusion));

        let config_path = root.join(".shun.toml");
        if config_path.is_file() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("failed to read {}", config_path.display()))?;
            let config = toml::from_str::<ScannerConfig>(&content)
                .with_context(|| format!("failed to parse {}", config_path.display()))?;
            configured.extend(config.exclude.into_iter().filter_map(normalize_exclusion));
        }

        Ok(Self {
            exclusions: configured,
        })
    }

    pub(crate) fn exclusions(&self) -> impl Iterator<Item = &str> {
        self.exclusions.iter().map(String::as_str)
    }
}

fn normalize_exclusion(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            exclusions: BUILT_IN_EXCLUSIONS
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ScannerConfig {
    #[serde(default)]
    exclude: Vec<String>,
}

const BUILT_IN_EXCLUSIONS: &[&str] = &[
    ".git",
    ".shun",
    "target",
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".idea",
    ".vscode",
];

/// This recursively discovers and reads supported documents below a root directory.
/// Parameters: root is the directory at which recursive traversal begins.
/// Returns: A relative-path-sorted vector containing one ScannedDocument for every
/// supported file that was read successfully, or an error when root does not exist,
/// cannot be resolved, or is not a directory.
pub(crate) fn scan_documents_with_options(
    root: &Path,
    options: &ScanOptions,
) -> Result<Vec<ScannedDocument>> {
    if !root.exists() {
        bail!("directory does not exist: {}", root.display());
    }
    if !root.is_dir() {
        bail!("path is not a directory: {}", root.display());
    }

    let absolute_root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve directory: {}", root.display()))?;
    let mut documents = Vec::new();

    for entry in WalkDir::new(&absolute_root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_directory(entry, options))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("Skipping inaccessible path: {error}");
                continue;
            }
        };

        if !entry.file_type().is_file() || !is_supported(entry.path()) {
            continue;
        }

        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("Skipping {}: {error}", entry.path().display());
                continue;
            }
        };

        let relative_path = entry
            .path()
            .strip_prefix(&absolute_root)
            .expect("walked entries must remain below the scan root")
            .to_path_buf();
        let tokenized = tokenize(&content);

        documents.push(ScannedDocument {
            absolute_path: entry.path().to_path_buf(),
            category: classify_file(&relative_path),
            relative_path,
            content,
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        });
    }

    // Order the paths to keep command output and tests deterministic.

    documents.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(documents)
}

/// This determines whether a path has an extension supported by repository indexing.
/// Parameters: path is the file-system path whose extension will be inspected.
/// Returns: true for Rust source, documentation, and initial configuration formats,
/// false when the extension is missing, unsupported, or is not valid UTF-8.
fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "rs" | "md" | "txt" | "toml" | "json"
            )
        })
}

/// This prevents generated output, dependency caches, and editor settings from being traversed.
/// Parameters: entry is the candidate directory entry provided by WalkDir.
/// Returns: true only when the entry is a directory with a configured ignored name.
fn is_ignored_directory(entry: &walkdir::DirEntry, options: &ScanOptions) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| options.exclusions.contains(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    #[test]
    fn scans_supported_documents_recursively() -> Result<()> {
        let directory = tempfile::tempdir().context("failed to create temporary directory")?;
        let source = directory.path().join("src");
        let tests = directory.path().join("tests");
        let examples = directory.path().join("examples");
        let generated = directory.path().join("target");
        fs::create_dir(&source)?;
        fs::create_dir(&tests)?;
        fs::create_dir(&examples)?;
        fs::create_dir(&generated)?;
        fs::write(directory.path().join("README.md"), "Rust ownership")?;
        fs::write(directory.path().join("Cargo.toml"), "package name shun")?;
        fs::write(source.join("main.rs"), "fn build_search_index")?;
        fs::write(source.join("data.JSON"), "configuration true")?;
        fs::write(tests.join("scanner_tests.rs"), "test scanner")?;
        fs::write(examples.join("basic.rs"), "example scanner")?;
        fs::write(source.join("image.png"), "not indexed")?;
        fs::write(generated.join("generated.rs"), "must be ignored")?;

        let documents = scan_documents_with_options(directory.path(), &ScanOptions::default())?;
        let relative_paths: Vec<_> = documents
            .iter()
            .map(|document| document.relative_path.as_path())
            .collect();

        assert_eq!(documents.len(), 6);
        assert_eq!(
            relative_paths,
            vec![
                Path::new("Cargo.toml"),
                Path::new("README.md"),
                Path::new("examples/basic.rs"),
                Path::new("src/data.JSON"),
                Path::new("src/main.rs"),
                Path::new("tests/scanner_tests.rs"),
            ]
        );
        assert!(
            documents
                .iter()
                .all(|document| document.absolute_path.is_absolute())
        );
        assert_eq!(documents[0].category, FileCategory::ProjectMetadata);
        assert_eq!(documents[1].category, FileCategory::Documentation);
        assert_eq!(documents[2].category, FileCategory::Example);
        assert_eq!(documents[3].category, FileCategory::Configuration);
        assert_eq!(documents[4].category, FileCategory::SourceCode);
        assert_eq!(documents[5].category, FileCategory::Test);
        assert_eq!(documents[4].content, "fn build_search_index");
        assert_eq!(documents[4].token_count, 2);
        assert_eq!(documents[4].tokens.len(), 5);
        assert_eq!(documents[4].tokens[0].term, "fn");
        assert_eq!(documents[4].tokens[1].term, "build_search_index");
        assert_eq!(documents[4].tokens[4].term, "index");
        Ok(())
    }

    #[test]
    fn merges_and_normalizes_configured_exclusions() -> Result<()> {
        let directory = tempfile::tempdir().context("failed to create temporary directory")?;
        fs::write(
            directory.path().join(".shun.toml"),
            "exclude = [\" vendor \", \"\", \"target\"]",
        )?;

        let options = ScanOptions::for_repository(
            directory.path(),
            vec![" generated ".to_owned(), " ".to_owned()],
        )?;
        let exclusions = options.exclusions().collect::<Vec<_>>();

        assert!(exclusions.contains(&"vendor"));
        assert!(exclusions.contains(&"generated"));
        assert_eq!(
            exclusions
                .iter()
                .filter(|value| **value == "target")
                .count(),
            1
        );
        assert!(!exclusions.contains(&""));
        Ok(())
    }
}
