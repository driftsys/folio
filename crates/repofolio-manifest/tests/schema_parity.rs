//! Schema parity test — M1 check-plan step 4
//! (docs/wip/2026-09-06-m1-check-plan.md), closing verification
//! follow-up 6 in `folio-plan.md`.
//!
//! Diffs the bundled schema copy against the canonical
//! `driftsys/schemas` checkout when that sibling checkout is present on
//! disk, so drift between the published schema and the copy embedded in
//! this crate is caught rather than silently accumulating. When the
//! sibling checkout is absent (e.g. a CI clone of only this repository),
//! the test skips rather than failing on missing context it cannot
//! control.

use std::path::Path;

#[test]
fn bundled_schema_matches_canonical_source_when_present() {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../schemas/project/v1.json");

    if !canonical.is_file() {
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
