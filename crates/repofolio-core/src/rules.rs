//! FOLIO- rule registry — M1 check-plan step 7
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! Four codes, and four codes only:
//!
//! | Code | Severity | Rule |
//! | --- | --- | --- |
//! | `FOLIO-001` | error | Manifest missing or unparseable |
//! | `FOLIO-002` | error (`info` when skipped) | Manifest fails the bundled schema |
//! | `FOLIO-101` | error | Required path missing, per active layer |
//! | `FOLIO-102` | warning | Recommended path missing, per active layer |
//!
//! Manifest presence belongs to `FOLIO-001` alone: a repository with no
//! manifest reports that fact once, never also as a missing path under
//! `FOLIO-101`. That is why `repofolio_ecosystem()`'s `markers.must` does
//! not list a manifest filename.
//!
//! `FOLIO-002` has nothing to validate once `FOLIO-001` has already
//! failed (there is no parsed value), so it is reported at severity
//! `Info` with a message saying it was skipped and why, rather than
//! silently omitted or reported as a second failure — see
//! [`skipped_schema_diagnostic`]. This is a recorded ruling, not a choice:
//! `Severity` stays exactly error/warning/info.

use std::path::Path;

use repofolio_manifest::{discover_manifest, parse_manifest, validate_manifest};

use crate::ecosystem::{Ecosystem, MarkerSpec};
use crate::report::{Diagnostic, Location, Severity};

const FOLIO_001: &str = "FOLIO-001";
const FOLIO_002: &str = "FOLIO-002";
const FOLIO_101: &str = "FOLIO-101";
const FOLIO_102: &str = "FOLIO-102";

/// The manifest filename reported when no manifest was found at all —
/// there is no real file to name, so the canonical form `folio init`
/// would scaffold stands in.
const CANONICAL_MANIFEST_NAME: &str = "project.toml";

/// Runs `FOLIO-001` (discover + parse) and `FOLIO-002` (schema
/// validation) against the manifest at `repo_root`. Implements the
/// discover -> parse -> validate pipeline stages (step 8): every failure
/// becomes a diagnostic rather than an early return, so the caller can
/// always continue to the later pipeline stages regardless of what
/// happened here.
pub fn manifest_rules(repo_root: &Path) -> Vec<Diagnostic> {
    let manifest_path = match discover_manifest(repo_root) {
        Ok(found) => found,
        Err(err) => {
            return vec![
                manifest_error(CANONICAL_MANIFEST_NAME, err.to_string()),
                skipped_schema_diagnostic(CANONICAL_MANIFEST_NAME),
            ];
        }
    };

    let file_name = manifest_path
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| CANONICAL_MANIFEST_NAME.to_string());

    let value = match parse_manifest(&manifest_path.path, manifest_path.format) {
        Ok(value) => value,
        Err(err) => {
            return vec![
                manifest_error(&file_name, err.to_string()),
                skipped_schema_diagnostic(&file_name),
            ];
        }
    };

    match validate_manifest(&value) {
        Ok(()) => Vec::new(),
        Err(err) => vec![Diagnostic {
            severity: Severity::Error,
            code: FOLIO_002.to_string(),
            message: err.to_string(),
            layer: None,
            location: Location::file(file_name),
        }],
    }
}

fn manifest_error(file_name: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code: FOLIO_001.to_string(),
        message,
        layer: None,
        location: Location::file(file_name.to_string()),
    }
}

/// `FOLIO-002` reported as skipped: severity `Info`, since `FOLIO-001`
/// has already failed and there is no parsed manifest left to validate.
fn skipped_schema_diagnostic(file_name: &str) -> Diagnostic {
    Diagnostic {
        severity: Severity::Info,
        code: FOLIO_002.to_string(),
        message: "skipped: FOLIO-001 already failed, nothing to validate".to_string(),
        layer: None,
        location: Location::file(file_name.to_string()),
    }
}

/// Runs `FOLIO-101` (required) and `FOLIO-102` (recommended)
/// path-presence checks for one active ecosystem's markers, naming
/// `ecosystem`'s name as the finding's layer.
///
/// Each marker entry is a [`MarkerSpec`]: a single required path, or an
/// alternative group satisfied when any one of its paths is present. A
/// missing alternative group produces exactly one finding naming every
/// alternative (via `MarkerSpec`'s `Display`), not one finding per
/// alternative — otherwise a repository satisfying the recommendation
/// through its second spelling would still be warned about the first.
pub fn path_rules(repo_root: &Path, ecosystem: &Ecosystem) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for spec in &ecosystem.markers.must {
        if !marker_spec_exists(repo_root, spec) {
            diagnostics.push(path_diagnostic(
                FOLIO_101,
                Severity::Error,
                "required",
                &ecosystem.ecosystem.name,
                spec,
            ));
        }
    }

    for spec in &ecosystem.markers.should {
        if !marker_spec_exists(repo_root, spec) {
            diagnostics.push(path_diagnostic(
                FOLIO_102,
                Severity::Warning,
                "recommended",
                &ecosystem.ecosystem.name,
                spec,
            ));
        }
    }

    diagnostics
}

/// A marker spec is satisfied when its single path exists at
/// `repo_root`, or — for an alternative group — when any one of its
/// paths exists there.
fn marker_spec_exists(repo_root: &Path, spec: &MarkerSpec) -> bool {
    match spec {
        MarkerSpec::One(path) => repo_root.join(path).exists(),
        MarkerSpec::AnyOf(paths) => paths.iter().any(|path| repo_root.join(path).exists()),
    }
}

fn path_diagnostic(
    code: &str,
    severity: Severity,
    kind: &str,
    layer: &str,
    spec: &MarkerSpec,
) -> Diagnostic {
    Diagnostic {
        severity,
        code: code.to_string(),
        message: format!("{kind} path missing: {spec}"),
        layer: Some(layer.to_string()),
        location: Location::file(spec.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::ecosystem::{Commands, EcosystemMeta, Markers};

    fn ecosystem_with(name: &str, must: Vec<MarkerSpec>, should: Vec<MarkerSpec>) -> Ecosystem {
        Ecosystem {
            schema: None,
            ecosystem: EcosystemMeta {
                name: name.to_string(),
                version: 1,
            },
            always: false,
            markers: Markers { must, should },
            commands: Commands::default(),
        }
    }

    fn one(path: &str) -> MarkerSpec {
        MarkerSpec::One(path.to_string())
    }

    fn any_of(paths: &[&str]) -> MarkerSpec {
        MarkerSpec::AnyOf(paths.iter().map(|s| s.to_string()).collect())
    }

    // FOLIO-001 / FOLIO-002 -------------------------------------------

    #[test]
    fn folio_001_fires_when_the_manifest_is_missing_and_folio_002_is_skipped() {
        let temp = tempfile::tempdir().expect("create temp dir");

        let diagnostics = manifest_rules(temp.path());

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code, "FOLIO-001");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].layer, None);
        assert_eq!(diagnostics[0].location.file, "project.toml");
        assert_eq!(diagnostics[1].code, "FOLIO-002");
        assert_eq!(diagnostics[1].severity, Severity::Info);
        assert!(diagnostics[1].message.contains("skipped"));
    }

    #[test]
    fn folio_001_fires_when_the_manifest_is_unparseable_and_folio_002_is_skipped() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("project.toml"), "name = \n").unwrap();

        let diagnostics = manifest_rules(temp.path());

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].code, "FOLIO-001");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].location.file, "project.toml");
        assert_eq!(diagnostics[1].code, "FOLIO-002");
        assert_eq!(diagnostics[1].severity, Severity::Info);
    }

    #[test]
    fn folio_002_fires_at_error_severity_when_the_manifest_fails_schema() {
        let temp = tempfile::tempdir().expect("create temp dir");
        // "version" is required by the bundled schema and is absent here.
        fs::write(
            temp.path().join("project.toml"),
            "name = \"com.example.x\"\n",
        )
        .unwrap();

        let diagnostics = manifest_rules(temp.path());

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "FOLIO-002");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].layer, None);
        assert_eq!(diagnostics[0].location.file, "project.toml");
    }

    #[test]
    fn manifest_rules_is_silent_when_the_manifest_is_valid() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp.path().join("project.toml"),
            "name = \"com.example.x\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        let diagnostics = manifest_rules(temp.path());

        assert!(diagnostics.is_empty());
    }

    // FOLIO-101 ---------------------------------------------------------

    #[test]
    fn folio_101_fires_for_a_missing_required_path() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let ecosystem = ecosystem_with("repofolio", vec![one("README.md")], vec![]);

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "FOLIO-101");
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(diagnostics[0].layer.as_deref(), Some("repofolio"));
        assert_eq!(diagnostics[0].location.file, "README.md");
    }

    #[test]
    fn folio_101_is_silent_when_the_required_path_is_present() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("README.md"), "hello\n").unwrap();
        let ecosystem = ecosystem_with("repofolio", vec![one("README.md")], vec![]);

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn folio_101_names_a_directory_spec_satisfied_by_the_directorys_presence() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::create_dir(temp.path().join("docs")).unwrap();
        let ecosystem = ecosystem_with("repofolio", vec![one("docs/")], vec![]);

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert!(diagnostics.is_empty());
    }

    // FOLIO-102 ---------------------------------------------------------

    #[test]
    fn folio_102_fires_for_a_missing_recommended_path() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let ecosystem = ecosystem_with("repofolio", vec![], vec![one("CHANGELOG.md")]);

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "FOLIO-102");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(diagnostics[0].layer.as_deref(), Some("repofolio"));
        assert_eq!(diagnostics[0].location.file, "CHANGELOG.md");
    }

    #[test]
    fn folio_102_is_silent_when_the_recommended_path_is_present() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("CHANGELOG.md"), "# changelog\n").unwrap();
        let ecosystem = ecosystem_with("repofolio", vec![], vec![one("CHANGELOG.md")]);

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn folio_102_alternate_group_is_silent_when_either_alternative_is_present() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join(".rustfmt.toml"), "").unwrap();
        let ecosystem = ecosystem_with(
            "rust",
            vec![],
            vec![any_of(&["rustfmt.toml", ".rustfmt.toml"])],
        );

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn folio_102_alternate_group_produces_exactly_one_finding_naming_the_full_spec() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let ecosystem = ecosystem_with(
            "rust",
            vec![],
            vec![any_of(&["rustfmt.toml", ".rustfmt.toml"])],
        );

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "FOLIO-102");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0].location.file,
            "rustfmt.toml or .rustfmt.toml"
        );
    }

    #[test]
    fn path_rules_reports_both_required_and_recommended_findings_together() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let ecosystem = ecosystem_with(
            "repofolio",
            vec![one("README.md")],
            vec![one("CHANGELOG.md")],
        );

        let diagnostics = path_rules(temp.path(), &ecosystem);

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|d| d.code == "FOLIO-101"
            && d.severity == Severity::Error
            && d.layer.as_deref() == Some("repofolio")));
        assert!(diagnostics.iter().any(|d| d.code == "FOLIO-102"
            && d.severity == Severity::Warning
            && d.layer.as_deref() == Some("repofolio")));
    }
}
