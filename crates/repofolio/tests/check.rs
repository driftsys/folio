//! Acceptance tests for `folio check` — M1 check-plan step 9
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! Runs the built `folio` binary end to end against small fixture
//! repositories, pinning the exit-status contract (0 clean/warnings-only,
//! 1 any error-severity finding, 2 usage failure) and both output
//! formats. The fixture-level finding-set tests already live in
//! `repofolio-core/tests/check_fixtures.rs`; these tests exist to pin the
//! command-line wiring on top of them, not to re-derive the rule engine.

use std::path::{Path, PathBuf};

use snapbox::cmd::Command;
use snapbox::file;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn folio() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin("folio"))
}

#[test]
fn clean_fixture_exits_0_with_an_empty_human_report() {
    folio()
        .arg("check")
        .arg(fixture("clean"))
        .assert()
        .code(0)
        .stdout_eq(file!["../snapshots/check/clean_human.stdout.expected"]);
}

/// Pins success criterion 1: the JSON output is exactly the `Report`
/// shape `repofolio-core` serializes, with no envelope added around it.
#[test]
fn clean_fixture_json_output_matches_the_report_shape() {
    folio()
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg(fixture("clean"))
        .assert()
        .code(0)
        .stdout_eq(file!["../snapshots/check/clean_json.stdout.expected"]);
}

/// Warnings alone must not fail the build (success criterion 2).
#[test]
fn warnings_only_fixture_exits_0() {
    folio()
        .arg("check")
        .arg(fixture("warnings"))
        .assert()
        .code(0)
        .stdout_eq(file!["../snapshots/check/warnings_human.stdout.expected"]);
}

/// A single error-severity finding fails the build (success criterion 2).
#[test]
fn error_fixture_exits_1() {
    folio()
        .arg("check")
        .arg(fixture("errors"))
        .assert()
        .code(1)
        .stdout_eq(file!["../snapshots/check/errors_human.stdout.expected"]);
}

#[test]
fn error_fixture_json_output_exits_1() {
    folio()
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg(fixture("errors"))
        .assert()
        .code(1)
        .stdout_eq(file!["../snapshots/check/errors_json.stdout.expected"]);
}

/// A path that is not a directory is a usage failure, not a finding —
/// exit 2, distinct from the 0/1 grading of the report itself.
#[test]
fn nonexistent_path_exits_2() {
    folio()
        .arg("check")
        .arg(fixture("clean").join("does-not-exist"))
        .assert()
        .code(2);
}

/// An unrecognized `--format` value is a usage failure clap itself
/// rejects before `run_check` ever sees it — still exit 2.
#[test]
fn invalid_format_value_exits_2() {
    folio()
        .arg("check")
        .arg("--format")
        .arg("xml")
        .arg(fixture("clean"))
        .assert()
        .code(2);
}
