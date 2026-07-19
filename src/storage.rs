/* ============================================================================
 * Shun: storage.rs
 * ============================================================================
 *
 * This file saves and loads versioned repository snapshots. A snapshot retains
 * scanned content and positioned tokens so every command can rebuild the same
 * derived SearchIndex without rescanning source files.
 *
 * ============================================================================
 */

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::scanner::ScannedDocument;

const SNAPSHOT_VERSION: u32 = 1;
const STORAGE_DIRECTORY: &str = ".shun";
const SNAPSHOT_FILE: &str = "index.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IndexSnapshot {
    pub(crate) version: u32,
    pub(crate) repository: PathBuf,
    pub(crate) indexed_at_unix_seconds: u64,
    pub(crate) exclusions: Vec<String>,
    pub(crate) documents: Vec<ScannedDocument>,
}

impl IndexSnapshot {
    pub(crate) fn new(
        repository: &Path,
        documents: Vec<ScannedDocument>,
        exclusions: Vec<String>,
    ) -> Result<Self> {
        Ok(Self {
            version: SNAPSHOT_VERSION,
            repository: repository.canonicalize().with_context(|| {
                format!("failed to resolve directory: {}", repository.display())
            })?,
            indexed_at_unix_seconds: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system time is before the Unix epoch")?
                .as_secs(),
            exclusions,
            documents,
        })
    }
}

pub(crate) fn save_snapshot(repository: &Path, snapshot: &IndexSnapshot) -> Result<PathBuf> {
    let directory = repository.join(STORAGE_DIRECTORY);
    fs::create_dir_all(&directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;
    let path = directory.join(SNAPSHOT_FILE);
    let temporary_path = directory.join("index.json.tmp");
    let bytes =
        serde_json::to_vec_pretty(snapshot).context("failed to serialize index snapshot")?;
    fs::write(&temporary_path, bytes)
        .with_context(|| format!("failed to write {}", temporary_path.display()))?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("failed to replace {}", path.display()))?;
    }
    fs::rename(&temporary_path, &path)
        .with_context(|| format!("failed to finalize {}", path.display()))?;
    Ok(path)
}

pub(crate) fn load_snapshot(repository: &Path) -> Result<Option<IndexSnapshot>> {
    let path = snapshot_path(repository);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let snapshot: IndexSnapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    if snapshot.version != SNAPSHOT_VERSION {
        bail!(
            "unsupported index version {} in {}; run `shun index` again",
            snapshot.version,
            path.display()
        );
    }
    let repository = repository
        .canonicalize()
        .with_context(|| format!("failed to resolve directory: {}", repository.display()))?;
    if snapshot.repository != repository {
        bail!("saved index belongs to another repository; run `shun index` again");
    }
    Ok(Some(snapshot))
}

pub(crate) fn clear_snapshot(repository: &Path) -> Result<bool> {
    let directory = repository.join(STORAGE_DIRECTORY);
    let path = directory.join(SNAPSHOT_FILE);
    if !path.is_file() {
        return Ok(false);
    }
    fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    if fs::read_dir(&directory)
        .with_context(|| format!("failed to inspect {}", directory.display()))?
        .next()
        .is_none()
    {
        fs::remove_dir(&directory)
            .with_context(|| format!("failed to remove empty directory {}", directory.display()))?;
    }
    Ok(true)
}

pub(crate) fn snapshot_path(repository: &Path) -> PathBuf {
    repository.join(STORAGE_DIRECTORY).join(SNAPSHOT_FILE)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::index::SearchIndex;
    use crate::scanner::{ScanOptions, scan_documents_with_options};

    use super::*;

    #[test]
    fn saves_loads_rebuilds_and_clears_a_snapshot() -> Result<()> {
        let directory = tempfile::tempdir().context("failed to create fixture")?;
        fs::create_dir(directory.path().join("src"))?;
        fs::create_dir(directory.path().join("private"))?;
        fs::write(
            directory.path().join("src/index.rs"),
            "pub struct SearchIndex;",
        )?;
        fs::write(directory.path().join("private/secret.rs"), "fn secret() {}")?;
        fs::write(
            directory.path().join(".shun.toml"),
            "exclude = [\"private\"]",
        )?;
        let options = ScanOptions::for_repository(directory.path(), Vec::new())?;
        let documents = scan_documents_with_options(directory.path(), &options)?;
        assert_eq!(documents.len(), 2);

        let snapshot = IndexSnapshot::new(
            directory.path(),
            documents,
            options.exclusions().map(str::to_owned).collect(),
        )?;
        let path = save_snapshot(directory.path(), &snapshot)?;
        assert!(path.is_file());
        fs::write(
            path.parent()
                .expect("snapshot has a parent")
                .join("keep.txt"),
            "keep",
        )?;

        let loaded = load_snapshot(directory.path())?.expect("snapshot should exist");
        assert_eq!(loaded, snapshot);
        let index = SearchIndex::build(&loaded.documents);
        assert_eq!(index.symbols.symbols[0].name, "SearchIndex");
        assert!(clear_snapshot(directory.path())?);
        assert!(load_snapshot(directory.path())?.is_none());
        assert!(directory.path().join(".shun/keep.txt").is_file());
        assert!(!clear_snapshot(directory.path())?);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_and_foreign_snapshots() -> Result<()> {
        let directory = tempfile::tempdir().context("failed to create fixture")?;
        let other_directory = tempfile::tempdir().context("failed to create other fixture")?;
        let mut snapshot = IndexSnapshot::new(directory.path(), Vec::new(), Vec::new())?;
        let path = save_snapshot(directory.path(), &snapshot)?;

        snapshot.version += 1;
        fs::write(&path, serde_json::to_vec_pretty(&snapshot)?)?;
        let version_error = load_snapshot(directory.path()).expect_err("version must be rejected");
        assert!(
            version_error
                .to_string()
                .contains("unsupported index version")
        );

        snapshot.version = SNAPSHOT_VERSION;
        snapshot.repository = other_directory.path().canonicalize()?;
        fs::write(&path, serde_json::to_vec_pretty(&snapshot)?)?;
        let repository_error =
            load_snapshot(directory.path()).expect_err("foreign snapshot must be rejected");
        assert!(
            repository_error
                .to_string()
                .contains("belongs to another repository")
        );
        Ok(())
    }
}
