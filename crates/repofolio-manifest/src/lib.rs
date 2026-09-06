//! repofolio-manifest — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! Manifest discovery — M1 check-plan step 1
//! (docs/wip/2026-09-06-m1-check-plan.md).

mod discover;

pub use discover::{discover_manifest, DiscoverError, ManifestFormat, ManifestPath};
