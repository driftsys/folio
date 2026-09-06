//! Fixture-level tests for the M1 check pipeline
//! (docs/archive/plans/2026-09-06-m1-check-plan.md steps 7-8).
//!
//! Each fixture under `tests/fixtures/` is a full repository tree, not
//! just a manifest — these pin the whole finding set `check()` produces
//! for a realistic repository, on top of the per-rule unit tests in
//! `src/rules.rs`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use repofolio_core::{check, BySeverity, Severity};

/// Copies fixture `name` into a fresh temp directory and returns it
/// together with the `TempDir` guard that must stay bound for as long as
/// the path is used (dropping it removes the directory).
///
/// The fixture is not used in place from `tests/fixtures/` because
/// `compliant` and `partial` each need a real `Cargo.toml` at their root
/// to activate the `rust` ecosystem layer under test — and a `Cargo.toml`
/// checked in at that path would make `cargo package` silently drop the
/// entire fixture directory as a nested package, since cargo treats any
/// directory containing a `Cargo.toml` as its own package and excludes
/// it (this is what `cargo package --list -p repofolio-core` confirms;
/// an `include` directive cannot override it, see commit c632564). The
/// checked-in fixture instead carries it as `Cargo.toml.fixture`; this
/// restores the real name in the copy so `check()` sees exactly the
/// tree a real repository would have.
fn materialize_fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let temp = tempfile::tempdir().expect("create temp dir");
    copy_dir_recursive(&source, temp.path());

    let disguised_manifest = temp.path().join("Cargo.toml.fixture");
    if disguised_manifest.is_file() {
        fs::rename(&disguised_manifest, temp.path().join("Cargo.toml"))
            .expect("restore Cargo.toml.fixture to Cargo.toml in the temp copy");
    }

    let root = temp.path().to_path_buf();
    (temp, root)
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {}: {e}", src.display())) {
        let entry = entry.expect("read dir entry");
        let file_type = entry.file_type().expect("read file type");
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path).expect("create dir in temp copy");
            copy_dir_recursive(&entry.path(), &dst_path);
        } else if file_type.is_file() {
            fs::copy(entry.path(), &dst_path).expect("copy file into temp copy");
        }
    }
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

/// A repository satisfying every FOLIO-001/003/002/101/102 rule for both
/// layers produces no diagnostics at all.
#[test]
fn compliant_fixture_produces_no_diagnostics() {
    let (_guard, root) = materialize_fixture("compliant");
    let report = check(&root);

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
/// that the missing `rustfmt.toml`/`.rustfmt.toml` alternative group's
/// `location.file` names only the first alternative (not a `Display`
/// dump of the whole spec, which is not a URI and cannot be opened).
#[test]
fn partial_fixture_pins_the_expected_finding_set() {
    let (_guard, root) = materialize_fixture("partial");
    let report = check(&root);

    // No manifest-level findings: project.toml is present and valid.
    assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-001"));
    assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-002"));
    assert!(!report.diagnostics.iter().any(|d| d.code == "FOLIO-003"));

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
        set(&["rust-toolchain.toml", "rustfmt.toml"])
    );

    assert_eq!(report.count, 9);
    assert_eq!(report.by_severity.error, 4);
    assert_eq!(report.by_severity.warning, 5);
    assert_eq!(report.by_severity.info, 0);
}

/// The no-abort property (step 8) at fixture scale: an entirely empty
/// repository — no manifest, no other files — still produces every
/// repofolio-layer FOLIO-101/FOLIO-102 finding, FOLIO-001 as an error,
/// and FOLIO-002 marked skipped at `Info` rather than omitted or
/// reported as a second error. The rust layer never activates, since
/// there is no Cargo.toml at all.
#[test]
fn empty_fixture_still_produces_path_findings_and_a_skipped_folio_002() {
    let (_guard, root) = materialize_fixture("empty");
    let report = check(&root);

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
    assert_eq!(report.by_severity.error, 10);
    assert_eq!(report.by_severity.warning, 5);
    assert_eq!(report.by_severity.info, 1);
}
