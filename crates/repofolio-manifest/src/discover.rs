//! Manifest discovery — M1 check-plan step 1
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! Finds the project manifest file at a repository root. The manifest may
//! be expressed in any one of three serialization formats; when more than
//! one is present, `project.toml` wins over `project.yaml`, which wins
//! over `project.json`.

use std::path::{Path, PathBuf};

/// On-disk serialization format of a project manifest, in discovery
/// priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestFormat {
    Toml,
    Yaml,
    Json,
}

impl ManifestFormat {
    /// Formats searched at the repository root, highest priority first.
    const PRIORITY: [ManifestFormat; 3] = [
        ManifestFormat::Toml,
        ManifestFormat::Yaml,
        ManifestFormat::Json,
    ];

    fn filename(self) -> &'static str {
        match self {
            ManifestFormat::Toml => "project.toml",
            ManifestFormat::Yaml => "project.yaml",
            ManifestFormat::Json => "project.json",
        }
    }
}

/// A manifest file located at a repository root, together with its
/// detected format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPath {
    pub path: PathBuf,
    pub format: ManifestFormat,
}

/// Manifest discovery failure. Maps to `FOLIO-001` (manifest missing or
/// unparseable) once diagnostic codes are assigned in `repofolio-core`.
#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error(
        "no project manifest (project.toml, project.yaml, or project.json) found in {root}",
        root = root.display()
    )]
    NotFound { root: PathBuf },
}

/// Finds the project manifest at `root`, trying `project.toml`,
/// `project.yaml`, then `project.json` in that order and returning the
/// first one present.
pub fn discover_manifest(root: &Path) -> Result<ManifestPath, DiscoverError> {
    for format in ManifestFormat::PRIORITY {
        let path = root.join(format.filename());
        if path.is_file() {
            return Ok(ManifestPath { path, format });
        }
    }
    Err(DiscoverError::NotFound {
        root: root.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Creates an isolated, empty temporary directory for one test. Named
    /// per-call so parallel tests never collide.
    fn temp_root(case: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "repofolio-manifest-discover-{case}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is after the epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    #[test]
    fn finds_toml_alone() {
        let root = temp_root("toml-alone");
        fs::write(root.join("project.toml"), "name = \"x\"\n").unwrap();

        let found = discover_manifest(&root).expect("manifest found");

        assert_eq!(found.path, root.join("project.toml"));
        assert_eq!(found.format, ManifestFormat::Toml);
    }

    #[test]
    fn finds_yaml_alone() {
        let root = temp_root("yaml-alone");
        fs::write(root.join("project.yaml"), "name: x\n").unwrap();

        let found = discover_manifest(&root).expect("manifest found");

        assert_eq!(found.path, root.join("project.yaml"));
        assert_eq!(found.format, ManifestFormat::Yaml);
    }

    #[test]
    fn finds_json_alone() {
        let root = temp_root("json-alone");
        fs::write(root.join("project.json"), "{\"name\": \"x\"}\n").unwrap();

        let found = discover_manifest(&root).expect("manifest found");

        assert_eq!(found.path, root.join("project.json"));
        assert_eq!(found.format, ManifestFormat::Json);
    }

    #[test]
    fn errors_when_none_present() {
        let root = temp_root("none-present");

        let err = discover_manifest(&root).expect_err("no manifest present");

        match err {
            DiscoverError::NotFound { root: reported } => assert_eq!(reported, root),
        }
    }

    #[test]
    fn prefers_toml_over_yaml_and_json_when_multiple_present() {
        let root = temp_root("multiple-present");
        fs::write(root.join("project.toml"), "name = \"x\"\n").unwrap();
        fs::write(root.join("project.yaml"), "name: x\n").unwrap();
        fs::write(root.join("project.json"), "{\"name\": \"x\"}\n").unwrap();

        let found = discover_manifest(&root).expect("manifest found");

        assert_eq!(found.format, ManifestFormat::Toml);
    }

    #[test]
    fn prefers_yaml_over_json_when_toml_absent() {
        let root = temp_root("yaml-over-json");
        fs::write(root.join("project.yaml"), "name: x\n").unwrap();
        fs::write(root.join("project.json"), "{\"name\": \"x\"}\n").unwrap();

        let found = discover_manifest(&root).expect("manifest found");

        assert_eq!(found.format, ManifestFormat::Yaml);
    }
}
