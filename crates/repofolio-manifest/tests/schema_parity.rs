//! Schema parity test — M1 check-plan step 4
//! (docs/archive/plans/2026-09-06-m1-check-plan.md), closing verification
//! follow-up 6 in `folio-plan.md`.
//!
//! Diffs the bundled schema copy against the canonical
//! `driftsys/schemas` checkout when that sibling checkout is present on
//! disk, so drift between the published schema and the copy embedded in
//! this crate is caught rather than silently accumulating. When the
//! sibling checkout is absent (e.g. a CI clone of only this repository),
//! the test skips rather than failing on missing context it cannot
//! control — unless `FOLIO_STRICT_PARITY` is set to any non-empty
//! value, in which case a missing checkout fails the test instead, so a
//! CI job that is supposed to have the sibling checked out catches a
//! misconfiguration rather than this test quietly asserting nothing.

use std::path::Path;

/// See the module doc comment for what this gates.
const FOLIO_STRICT_PARITY: &str = "FOLIO_STRICT_PARITY";

/// Pure decision logic for the constant above, factored out of
/// [`strict_parity_required`] so [`strict_parity_toggles_skip_vs_fail`]
/// can exercise both branches directly without mutating the process
/// environment (which would race
/// `bundled_schema_matches_canonical_source_when_present`'s own read of
/// the same variable under parallel test execution).
fn strict_parity_from(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.is_empty())
}

fn strict_parity_required() -> bool {
    strict_parity_from(std::env::var(FOLIO_STRICT_PARITY).ok().as_deref())
}

/// Exercises both branches of the gating decision
/// `bundled_schema_matches_canonical_source_when_present` uses when the
/// canonical checkout is absent: unset (or empty) skips, any non-empty
/// value fails. Testing the pure predicate this way pins both branches
/// without requiring the sibling checkout to actually be removed from
/// this machine to observe the fail branch.
#[test]
fn strict_parity_toggles_skip_vs_fail() {
    assert!(!strict_parity_from(None));
    assert!(!strict_parity_from(Some("")));
    assert!(strict_parity_from(Some("1")));
    assert!(strict_parity_from(Some("0")));
    assert!(strict_parity_from(Some("false")));
}

#[test]
fn bundled_schema_matches_canonical_source_when_present() {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../schemas/project/v1.json");

    if !canonical.is_file() {
        assert!(
            !strict_parity_required(),
            "{FOLIO_STRICT_PARITY} is set and the canonical driftsys/schemas checkout \
             was not found at {}",
            canonical.display()
        );
        eprintln!(
            "skipping schema parity test: canonical checkout not found at {}",
            canonical.display()
        );
        return;
    }

    let canonical_bytes = std::fs::read(&canonical)
        .unwrap_or_else(|e| panic!("read canonical schema {}: {e}", canonical.display()));
    let bundled_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/project-v1.json");
    let bundled_bytes = std::fs::read(&bundled_path)
        .unwrap_or_else(|e| panic!("read bundled schema {}: {e}", bundled_path.display()));

    assert_eq!(
        bundled_bytes,
        canonical_bytes,
        "crates/repofolio-manifest/schema/project-v1.json has drifted from {} — \
         copy it byte-identically, do not hand-edit it",
        canonical.display()
    );
}
