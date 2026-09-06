//! Check pipeline — M1 check-plan step 8
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! `discover -> parse -> validate -> core rules -> ecosystem detection
//! and rules -> report`. No stage aborts the run: `folio check` against
//! a repository with a missing or broken manifest still runs every later
//! stage, because the path-presence rules (`FOLIO-101`/`FOLIO-102`) do
//! not depend on the manifest having parsed or validated.

use std::path::Path;

use crate::ecosystem::{detect_rust, repofolio_ecosystem, rust_ecosystem};
use crate::report::Report;
use crate::rules::{manifest_rules, path_rules};

/// Runs the full check pipeline against the repository at `repo_root` and
/// returns its [`Report`].
pub fn check(repo_root: &Path) -> Report {
    let mut diagnostics = Vec::new();

    // discover -> parse -> validate (FOLIO-001, FOLIO-003, FOLIO-002).
    diagnostics.extend(manifest_rules(repo_root));

    // core rules: the repofolio layer is active in every repository.
    diagnostics.extend(path_rules(repo_root, &repofolio_ecosystem()));

    // ecosystem detection and rules: rust activates on a workspace
    // Cargo.toml.
    if detect_rust(repo_root) {
        diagnostics.extend(path_rules(repo_root, &rust_ecosystem()));
    }

    Report::new(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Severity;
    use std::fs;

    /// Pins the pipeline's no-abort property directly: a repository with
    /// no manifest at all — no `project.toml`/`.yaml`/`.json`, no other
    /// files — still yields `FOLIO-101`/`FOLIO-102` path findings, and
    /// `FOLIO-002` appears as an `Info`-severity "skipped" finding rather
    /// than as an error or as nothing at all. If any pipeline stage
    /// aborted the run on the missing manifest, this repository would
    /// produce only the `FOLIO-001` finding.
    #[test]
    fn a_repository_with_no_manifest_still_produces_path_findings_and_a_skipped_folio_002() {
        let temp = tempfile::tempdir().expect("create temp dir");

        let report = check(temp.path());

        let folio_001 = report
            .diagnostics
            .iter()
            .find(|d| d.code == "FOLIO-001")
            .expect("FOLIO-001 is present");
        assert_eq!(folio_001.severity, Severity::Error);

        let folio_002 = report
            .diagnostics
            .iter()
            .find(|d| d.code == "FOLIO-002")
            .expect("FOLIO-002 is present");
        assert_eq!(folio_002.severity, Severity::Info);
        assert!(folio_002.message.contains("skipped"));

        let path_findings: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code == "FOLIO-101" || d.code == "FOLIO-102")
            .collect();
        assert!(
            !path_findings.is_empty(),
            "expected FOLIO-101/FOLIO-102 findings even with no manifest"
        );
        // No Cargo.toml exists, so the rust ecosystem never activates:
        // every path finding here is the repofolio layer.
        for finding in &path_findings {
            assert_eq!(finding.layer.as_deref(), Some("repofolio"));
        }
        assert!(path_findings
            .iter()
            .any(|d| d.code == "FOLIO-101" && d.severity == Severity::Error));
        assert!(path_findings
            .iter()
            .any(|d| d.code == "FOLIO-102" && d.severity == Severity::Warning));
    }

    /// Fixture-level coverage for the `FOLIO-003` path through the full
    /// pipeline (unit-level coverage lives in `rules.rs`): a repository
    /// whose `project.toml` exists but fails to parse must report
    /// `FOLIO-003`, not `FOLIO-001`, and `FOLIO-002` must be skipped with
    /// a message naming `FOLIO-003` as the reason.
    #[test]
    fn a_repository_with_an_unparseable_manifest_produces_folio_003_and_a_skipped_folio_002() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("project.toml"), "name = \n").unwrap();

        let report = check(temp.path());

        let folio_003 = report
            .diagnostics
            .iter()
            .find(|d| d.code == "FOLIO-003")
            .expect("FOLIO-003 is present");
        assert_eq!(folio_003.severity, Severity::Error);

        let folio_002 = report
            .diagnostics
            .iter()
            .find(|d| d.code == "FOLIO-002")
            .expect("FOLIO-002 is present");
        assert_eq!(folio_002.severity, Severity::Info);
        assert!(folio_002.message.contains("skipped"));
        assert!(folio_002.message.contains("FOLIO-003"));

        assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-001"));
    }

    #[test]
    fn rust_layer_rules_run_only_when_a_workspace_cargo_toml_is_present() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        let report = check(temp.path());

        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.layer.as_deref() == Some("rust")
                && d.code == "FOLIO-101"
                && d.severity == Severity::Error));
    }

    #[test]
    fn rust_layer_rules_do_not_run_without_a_workspace_cargo_toml() {
        let temp = tempfile::tempdir().expect("create temp dir");

        let report = check(temp.path());

        assert!(!report
            .diagnostics
            .iter()
            .any(|d| d.layer.as_deref() == Some("rust")));
    }

    #[test]
    fn a_fully_conformant_repository_produces_an_empty_report() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let root = temp.path();
        fs::write(
            root.join("project.toml"),
            "name = \"com.example.clean\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        for file in [
            "README.md",
            "LICENSE",
            "bootstrap",
            "runw",
            ".gitignore",
            ".gitattributes",
            ".editorconfig",
            "Foliofile",
            "CHANGELOG.md",
            "CODEOWNERS",
            "CONTRIBUTING.md",
        ] {
            fs::write(root.join(file), "").unwrap();
        }
        for dir in ["docs", "scripts", ".githooks"] {
            fs::create_dir(root.join(dir)).unwrap();
        }

        let report = check(root);

        assert!(
            report.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            report.diagnostics
        );
        assert_eq!(report.count, 0);
    }
}
