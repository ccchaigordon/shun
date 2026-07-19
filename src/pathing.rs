/* ============================================================================
 * Shun: pathing.rs
 * ============================================================================
 *
 * This file provides lexical repository-path normalization shared by features
 * that compare user or documentation paths with scanner-relative paths.
 *
 * ============================================================================
 */

use std::path::{Component, Path};

pub(crate) fn normalize_repository_path(path: &Path) -> String {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                if let Some(value) = value.to_str() {
                    components.push(value);
                }
            }
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    let normalized = components.join("/");
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_relative_parent_components() {
        assert_eq!(
            normalize_repository_path(Path::new("docs/../src/index.rs")),
            "src/index.rs"
        );
    }

    #[test]
    #[cfg(windows)]
    fn compares_windows_repository_paths_without_case_sensitivity() {
        assert_eq!(
            normalize_repository_path(Path::new("Src/Index.rs")),
            "src/index.rs"
        );
    }
}
