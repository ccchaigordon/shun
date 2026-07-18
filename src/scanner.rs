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

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

use crate::tokenizer::tokenize;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ScannedDocument {
    pub(crate) absolute_path: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) token_count: usize,
}

/// This recursively discovers and reads supported documents below a root directory.
/// Parameters: root is the directory at which recursive traversal begins.
/// Returns: A relative-path-sorted vector containing one ScannedDocument for every
/// supported file that was read successfully, or an error when root does not exist,
/// cannot be resolved, or is not a directory.

pub(crate) fn scan_documents(root: &Path) -> Result<Vec<ScannedDocument>> {
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
        .filter_entry(|entry| !is_ignored_directory(entry))
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

        documents.push(ScannedDocument {
            absolute_path: entry.path().to_path_buf(),
            relative_path,
            token_count: tokenize(&content).len(),
        });
    }

    // Order the paths to keep command output and tests deterministic.

    documents.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(documents)
}

/// This determines whether a path has an extension supported by repository indexing.
/// Parameters: path is the file-system path whose extension will be inspected.
/// Returns: true for Rust source, documentation, and initial configuration formats;
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

fn is_ignored_directory(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && matches!(
            entry.file_name().to_str(),
            Some(
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | "coverage"
                    | ".idea"
                    | ".vscode"
            )
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    #[test]
    fn scans_supported_documents_recursively() -> Result<()> {
        let directory = tempfile::tempdir().context("failed to create temporary directory")?;
        let source = directory.path().join("src");
        let generated = directory.path().join("target");
        fs::create_dir(&source)?;
        fs::create_dir(&generated)?;
        fs::write(directory.path().join("README.md"), "Rust ownership")?;
        fs::write(directory.path().join("Cargo.toml"), "package name shun")?;
        fs::write(source.join("main.rs"), "fn main search engine")?;
        fs::write(source.join("data.JSON"), "configuration true")?;
        fs::write(source.join("image.png"), "not indexed")?;
        fs::write(generated.join("generated.rs"), "must be ignored")?;

        let documents = scan_documents(directory.path())?;
        let relative_paths: Vec<_> = documents
            .iter()
            .map(|document| document.relative_path.as_path())
            .collect();

        assert_eq!(documents.len(), 4);
        assert_eq!(
            relative_paths,
            vec![
                Path::new("Cargo.toml"),
                Path::new("README.md"),
                Path::new("src/data.JSON"),
                Path::new("src/main.rs"),
            ]
        );
        assert!(
            documents
                .iter()
                .all(|document| document.absolute_path.is_absolute())
        );
        assert_eq!(documents[3].token_count, 4);
        Ok(())
    }
}
