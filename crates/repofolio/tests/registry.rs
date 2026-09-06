//! Acceptance tests for `folio registry` — CLAUDE.md's non-negotiable
//! architecture decision 6 (tool-contract clause 4: "a `<tool> registry
//! --format json` dump"), added after M1's original check-plan.
//!
//! Runs the built `folio` binary end to end, pinning the wire shape at
//! the CLI boundary the same way `check.rs` pins `check`'s. The registry
//! content itself (which codes, which ecosystems) is unit-tested in
//! `repofolio-core`'s own `registry` module — these tests exist to pin
//! the command-line wiring on top of it, not to re-derive the registry.

use snapbox::cmd::Command;
use snapbox::file;

fn folio() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin("folio"))
}

#[test]
fn registry_json_output_matches_the_pinned_shape() {
    folio()
        .arg("registry")
        .arg("--format")
        .arg("json")
        .assert()
        .code(0)
        .stdout_eq(file!["../snapshots/registry/registry_json.stdout.expected"]);
}

/// `--format json` is the default, so bare `folio registry` must
/// produce byte-identical output to the explicit form.
#[test]
fn registry_defaults_to_json_output() {
    folio()
        .arg("registry")
        .assert()
        .code(0)
        .stdout_eq(file!["../snapshots/registry/registry_json.stdout.expected"]);
}

#[test]
fn registry_rejects_an_unrecognized_format_value() {
    folio()
        .arg("registry")
        .arg("--format")
        .arg("human")
        .assert()
        .code(2);
}
