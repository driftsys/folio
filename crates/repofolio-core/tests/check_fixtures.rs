//! Fixture-level tests for the M1 check pipeline
//! (docs/wip/2026-09-06-m1-check-plan.md steps 7-8).
//!
//! Each fixture under `tests/fixtures/` is a full repository tree, not
//! just a manifest — these pin the whole finding set `check()` produces
//! for a realistic repository, on top of the per-rule unit tests in
//! `src/rules.rs`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use repofolio_core::{check, BySeverity, Severity};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Collects the `location.file` of every diagnostic matching both `code`
/// and `layer` (the same code fires for more than one layer, e.g.
/// `FOLIO-101` for both `repofolio` and `rust`), asserting each one has
/// `severity` — a fixture-level test that only checked *some* field
/// would not really pin the finding.
fn paths_for(
    report: &repofolio_core::Report,
    code: &str,
    severity: Severity,
    layer: Option<&str>,
) -> BTreeSet<String> {
    report
        .diagnostics
        .iter()
        .filter(|d| d.code == code && d.layer.as_deref() == layer)
        .map(|d| {
            assert_eq!(
                d.severity, severity,
                "{code} finding {:?} has unexpected severity",
                d
            );
            d.location.file.clone()
        })
        .collect()
}

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|s| s.to_string()).collect()
}

/// A repository satisfying every FOLIO-001/002/101/102 rule for both
/// layers produces no diagnostics at all.
#[test]
fn compliant_fixture_produces_no_diagnostics() {
    let report = check(&fixture("compliant"));

    assert_eq!(
        report.diagnostics,
        Vec::new(),
        "compliant fixture must be clean"
    );
    assert_eq!(report.count, 0);
    assert_eq!(report.by_severity, BySeverity::default());
}

/// A repository with a valid manifest but several missing required and
/// recommended paths, across both the always-active `repofolio` layer
/// and a detected `rust` layer, pins the exact finding set — including
/// that the `rust-toolchain.toml` alternative-group's message names the
/// spec verbatim rather than one of its two alternatives.
#[test]
fn partial_fixture_pins_the_expected_finding_set() {
    let report = check(&fixture("partial"));

    // No manifest-level findings: project.toml is present and valid.
    assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-001"));
    assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-002"));

    assert_eq!(
        paths_for(&report, "FOLIO-101", Severity::Error, Some("repofolio")),
        set(&["bootstrap", "runw", "scripts/"])
    );
    assert_eq!(
        paths_for(&report, "FOLIO-102", Severity::Warning, Some("repofolio")),
        set(&["Foliofile", "CODEOWNERS", ".githooks/"])
    );
    assert_eq!(
        paths_for(&report, "FOLIO-101", Severity::Error, Some("rust")),
        set(&["Cargo.lock"])
    );
    assert_eq!(
        paths_for(&report, "FOLIO-102", Severity::Warning, Some("rust")),
        set(&["rust-toolchain.toml", "rustfmt.toml or .rustfmt.toml"])
    );

    assert_eq!(report.count, 9);
    assert_eq!(
        report.by_severity,
        BySeverity {
            error: 4,
            warning: 5,
            info: 0,
        }
    );
}

/// The no-abort property (step 8) at fixture scale: an entirely empty
/// repository — no manifest, no other files — still produces every
/// repofolio-layer FOLIO-101/FOLIO-102 finding, FOLIO-001 as an error,
/// and FOLIO-002 marked skipped at `Info` rather than omitted or
/// reported as a second error. The rust layer never activates, since
/// there is no Cargo.toml at all.
#[test]
fn empty_fixture_still_produces_path_findings_and_a_skipped_folio_002() {
    let report = check(&fixture("empty"));

    let folio_001: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "FOLIO-001")
        .collect();
    assert_eq!(folio_001.len(), 1);
    assert_eq!(folio_001[0].severity, Severity::Error);
    assert_eq!(folio_001[0].layer, None);

    let folio_002: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "FOLIO-002")
        .collect();
    assert_eq!(folio_002.len(), 1);
    assert_eq!(folio_002[0].severity, Severity::Info);
    assert_eq!(folio_002[0].layer, None);
    assert!(folio_002[0].message.contains("skipped"));

    assert_eq!(
        paths_for(&report, "FOLIO-101", Severity::Error, Some("repofolio")),
        set(&[
            "README.md",
            "LICENSE",
            "bootstrap",
            "runw",
            ".gitignore",
            ".gitattributes",
            ".editorconfig",
            "docs/",
            "scripts/",
        ])
    );
    assert_eq!(
        paths_for(&report, "FOLIO-102", Severity::Warning, Some("repofolio")),
        set(&[
            "Foliofile",
            "CHANGELOG.md",
            "CODEOWNERS",
            "CONTRIBUTING.md",
            ".githooks/",
        ])
    );

    // No Cargo.toml anywhere in this fixture: the rust layer never
    // activates, so it contributes no findings at all.
    assert!(!report
        .diagnostics
        .iter()
        .any(|d| d.layer.as_deref() == Some("rust")));

    assert_eq!(report.count, 16);
    assert_eq!(
        report.by_severity,
        BySeverity {
            error: 10,
            warning: 5,
            info: 1,
        }
    );
}
