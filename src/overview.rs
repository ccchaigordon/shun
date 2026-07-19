/* ============================================================================
 * Shun: overview.rs
 * ============================================================================
 *
 * This file summarizes repository structure from scanned documents, Cargo
 * metadata, file roles, and direct crate dependencies in the primary entry
 * point.
 *
 * ============================================================================
 */

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::classifier::FileCategory;
use crate::scanner::ScannedDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectKind {
    RustWorkspace,
    RustLibraryAndCli,
    RustCli,
    RustLibrary,
    RustPackage,
    RustRepository,
    Unknown,
}

impl fmt::Display for ProjectKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RustWorkspace => "Rust workspace",
            Self::RustLibraryAndCli => "Rust library and CLI application",
            Self::RustCli => "Rust CLI application",
            Self::RustLibrary => "Rust library",
            Self::RustPackage => "Rust package",
            Self::RustRepository => "Rust repository",
            Self::Unknown => "Unknown repository",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectOverview {
    pub(crate) project_kind: ProjectKind,
    pub(crate) package_name: Option<String>,
    pub(crate) entry_points: Vec<PathBuf>,
    pub(crate) core_modules: Vec<PathBuf>,
    pub(crate) configuration: Vec<PathBuf>,
    pub(crate) tests: Vec<PathBuf>,
    pub(crate) examples: Vec<PathBuf>,
    pub(crate) workflow: Vec<String>,
    pub(crate) manifest_error: Option<String>,
}

impl ProjectOverview {
    pub(crate) fn build(documents: &[ScannedDocument]) -> Self {
        let manifest_document = documents
            .iter()
            .find(|document| document.relative_path == Path::new("Cargo.toml"));
        let (manifest, manifest_error) = manifest_document.map_or((None, None), |document| {
            match toml::from_str::<toml::Value>(&document.content) {
                Ok(manifest) => (Some(manifest), None),
                Err(error) => (None, Some(error.to_string())),
            }
        });
        let package_name = manifest
            .as_ref()
            .and_then(|manifest| manifest.get("package"))
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)
            .map(str::to_owned);

        let mut entry_points = BTreeSet::new();
        for manifest_document in documents.iter().filter(|document| {
            document
                .relative_path
                .file_name()
                .and_then(|name| name.to_str())
                == Some("Cargo.toml")
        }) {
            let package_root = manifest_document
                .relative_path
                .parent()
                .unwrap_or_else(|| Path::new(""));
            collect_default_targets(package_root, documents, &mut entry_points);
            if let Ok(package_manifest) = toml::from_str::<toml::Value>(&manifest_document.content)
            {
                collect_manifest_targets(
                    &package_manifest,
                    package_root,
                    documents,
                    &mut entry_points,
                );
            }
        }
        let entry_points = entry_points.into_iter().collect::<Vec<_>>();

        let has_manifest_binary = manifest
            .as_ref()
            .and_then(|manifest| manifest.get("bin"))
            .and_then(toml::Value::as_array)
            .is_some_and(|targets| !targets.is_empty());
        let has_manifest_library = manifest
            .as_ref()
            .is_some_and(|manifest| manifest.get("lib").is_some());
        let has_main = has_manifest_binary
            || entry_points.iter().any(|path| {
                path == Path::new("src/main.rs") || path.starts_with(Path::new("src/bin"))
            });
        let has_library = has_manifest_library
            || entry_points
                .iter()
                .any(|path| path == Path::new("src/lib.rs"));
        let has_workspace = manifest
            .as_ref()
            .is_some_and(|manifest| manifest.get("workspace").is_some());
        let has_rust = documents
            .iter()
            .any(|document| has_rust_extension(&document.relative_path));
        let project_kind = if has_workspace {
            ProjectKind::RustWorkspace
        } else if has_main && has_library {
            ProjectKind::RustLibraryAndCli
        } else if has_main {
            ProjectKind::RustCli
        } else if has_library {
            ProjectKind::RustLibrary
        } else if manifest.is_some() {
            ProjectKind::RustPackage
        } else if has_rust {
            ProjectKind::RustRepository
        } else {
            ProjectKind::Unknown
        };

        let entry_set = entry_points.iter().collect::<BTreeSet<_>>();
        let core_modules = documents
            .iter()
            .filter(|document| document.category == FileCategory::SourceCode)
            .filter(|document| !entry_set.contains(&document.relative_path))
            .map(|document| document.relative_path.clone())
            .collect();
        let configuration = documents
            .iter()
            .filter(|document| {
                matches!(
                    document.category,
                    FileCategory::Configuration | FileCategory::ProjectMetadata
                )
            })
            .map(|document| document.relative_path.clone())
            .collect();
        let tests = documents
            .iter()
            .filter(|document| {
                document.category == FileCategory::Test || contains_rust_tests(document)
            })
            .map(|document| document.relative_path.clone())
            .collect();
        let examples = paths_for_category(documents, FileCategory::Example);
        let workflow = infer_workflow(documents, &entry_points);

        Self {
            project_kind,
            package_name,
            entry_points,
            core_modules,
            configuration,
            tests,
            examples,
            workflow,
            manifest_error,
        }
    }
}

fn collect_manifest_targets(
    manifest: &toml::Value,
    package_root: &Path,
    documents: &[ScannedDocument],
    entry_points: &mut BTreeSet<PathBuf>,
) {
    if let Some(path) = manifest
        .get("lib")
        .and_then(|target| target.get("path"))
        .and_then(toml::Value::as_str)
    {
        insert_existing_target(path, package_root, documents, entry_points);
    }
    if let Some(targets) = manifest.get("bin").and_then(toml::Value::as_array) {
        for path in targets
            .iter()
            .filter_map(|target| target.get("path").and_then(toml::Value::as_str))
        {
            insert_existing_target(path, package_root, documents, entry_points);
        }
    }
}

fn collect_default_targets(
    package_root: &Path,
    documents: &[ScannedDocument],
    entry_points: &mut BTreeSet<PathBuf>,
) {
    for relative_target in ["src/main.rs", "src/lib.rs"] {
        let path = package_root.join(relative_target);
        if contains_path(documents, &path) {
            entry_points.insert(path);
        }
    }
    let binary_root = package_root.join("src/bin");
    entry_points.extend(
        documents
            .iter()
            .filter(|document| {
                document.relative_path.starts_with(&binary_root)
                    && has_rust_extension(&document.relative_path)
            })
            .map(|document| document.relative_path.clone()),
    );
}

fn insert_existing_target(
    path: &str,
    package_root: &Path,
    documents: &[ScannedDocument],
    entry_points: &mut BTreeSet<PathBuf>,
) {
    let path = package_root.join(path);
    if contains_path(documents, &path) {
        entry_points.insert(path);
    }
}

fn contains_path(documents: &[ScannedDocument], path: &Path) -> bool {
    documents
        .iter()
        .any(|document| document.relative_path == path)
}

fn has_rust_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
}

fn paths_for_category(documents: &[ScannedDocument], category: FileCategory) -> Vec<PathBuf> {
    documents
        .iter()
        .filter(|document| document.category == category)
        .map(|document| document.relative_path.clone())
        .collect()
}

fn contains_rust_tests(document: &ScannedDocument) -> bool {
    if !has_rust_extension(&document.relative_path) {
        return false;
    }
    let Ok(file) = syn::parse_file(&document.content) else {
        return false;
    };
    file.items.iter().any(|item| match item {
        syn::Item::Fn(function) => has_test_attribute(&function.attrs),
        syn::Item::Mod(module) => has_cfg_test_attribute(&module.attrs),
        _ => false,
    })
}

fn has_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test")
    })
}

fn has_cfg_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
    })
}

fn infer_workflow(documents: &[ScannedDocument], entry_points: &[PathBuf]) -> Vec<String> {
    let Some(entry_point) = entry_points
        .iter()
        .find(|path| path.ends_with(Path::new("src/main.rs")))
        .or_else(|| entry_points.first())
    else {
        return Vec::new();
    };
    let Some(document) = documents
        .iter()
        .find(|document| document.relative_path == *entry_point)
    else {
        return Vec::new();
    };
    let mut workflow = vec![module_name(entry_point)];
    let Ok(file) = syn::parse_file(&document.content) else {
        return workflow;
    };

    for item in file.items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        collect_crate_use_roots(&item_use.tree, &mut workflow);
    }
    workflow
}

fn collect_crate_use_roots(tree: &syn::UseTree, workflow: &mut Vec<String>) {
    let syn::UseTree::Path(path) = tree else {
        return;
    };
    if path.ident != "crate" {
        return;
    }
    collect_first_use_segments(&path.tree, workflow);
}

fn collect_first_use_segments(tree: &syn::UseTree, workflow: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => push_unique(workflow, path.ident.to_string()),
        syn::UseTree::Name(name) => push_unique(workflow, name.ident.to_string()),
        syn::UseTree::Rename(rename) => push_unique(workflow, rename.ident.to_string()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_first_use_segments(item, workflow);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn module_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("entry")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::classifier::classify_file;
    use crate::tokenizer::tokenize;

    use super::*;

    fn document(path: &str, content: &str) -> ScannedDocument {
        let relative_path = PathBuf::from(path);
        let tokenized = tokenize(content);
        ScannedDocument {
            absolute_path: Path::new("/").join(path),
            category: classify_file(&relative_path),
            relative_path,
            content: content.to_owned(),
            token_count: tokenized.source_token_count,
            tokens: tokenized.tokens,
        }
    }

    #[test]
    fn summarizes_a_rust_cli_repository() {
        let documents = vec![
            document(
                "Cargo.toml",
                "[package]\nname = \"sample\"\nversion = \"0.1.0\"",
            ),
            document(
                "src/main.rs",
                "use crate::cli::run;\nuse crate::{scanner, search};\nfn main() {}",
            ),
            document("src/cli.rs", "pub fn run() {}"),
            document("src/scanner.rs", "pub fn scan() {}"),
            document("src/search.rs", "pub fn search() {}"),
            document("src/tokenizer.rs", "#[cfg(test)] mod tests {}"),
            document("src/runtime.rs", "#[tokio::test] async fn works() {}"),
            document("config/app.toml", "enabled = true"),
            document("tests/cli_tests.rs", "#[test] fn works() {}"),
            document("examples/basic.rs", "fn main() {}"),
        ];

        let overview = ProjectOverview::build(&documents);

        assert_eq!(overview.project_kind, ProjectKind::RustCli);
        assert_eq!(overview.package_name.as_deref(), Some("sample"));
        assert_eq!(overview.entry_points, [PathBuf::from("src/main.rs")]);
        assert_eq!(
            overview.core_modules,
            [
                PathBuf::from("src/cli.rs"),
                PathBuf::from("src/scanner.rs"),
                PathBuf::from("src/search.rs"),
                PathBuf::from("src/tokenizer.rs"),
                PathBuf::from("src/runtime.rs")
            ]
        );
        assert_eq!(overview.configuration.len(), 2);
        assert_eq!(
            overview.tests,
            [
                PathBuf::from("src/tokenizer.rs"),
                PathBuf::from("src/runtime.rs"),
                PathBuf::from("tests/cli_tests.rs")
            ]
        );
        assert_eq!(overview.examples, [PathBuf::from("examples/basic.rs")]);
        assert_eq!(overview.workflow, ["main", "cli", "scanner", "search"]);
        assert!(overview.manifest_error.is_none());
    }

    #[test]
    fn detects_manifest_declared_library_and_binary_targets() {
        let documents = vec![
            document(
                "Cargo.toml",
                "[package]\nname = \"custom\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"source/library.rs\"\n\n[[bin]]\nname = \"custom\"\npath = \"commands/run.rs\"",
            ),
            document("source/library.rs", "pub fn library() {}"),
            document("commands/run.rs", "fn main() {}"),
        ];

        let overview = ProjectOverview::build(&documents);

        assert_eq!(overview.project_kind, ProjectKind::RustLibraryAndCli);
        assert_eq!(
            overview.entry_points,
            [
                PathBuf::from("commands/run.rs"),
                PathBuf::from("source/library.rs")
            ]
        );
    }

    #[test]
    fn finds_entry_points_in_virtual_workspace_members() {
        let documents = vec![
            document(
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/app\", \"crates/core\"]",
            ),
            document(
                "crates/app/Cargo.toml",
                "[package]\nname = \"app\"\nversion = \"0.1.0\"",
            ),
            document("crates/app/src/main.rs", "fn main() {}"),
            document(
                "crates/core/Cargo.toml",
                "[package]\nname = \"core\"\nversion = \"0.1.0\"\n[lib]\npath = \"source/core.rs\"",
            ),
            document("crates/core/source/core.rs", "pub fn run() {}"),
        ];

        let overview = ProjectOverview::build(&documents);

        assert_eq!(overview.project_kind, ProjectKind::RustWorkspace);
        assert_eq!(
            overview.entry_points,
            [
                PathBuf::from("crates/app/src/main.rs"),
                PathBuf::from("crates/core/source/core.rs")
            ]
        );
        assert_eq!(overview.workflow, ["main"]);
    }
}
