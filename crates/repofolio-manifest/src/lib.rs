//! repofolio-manifest — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! Manifest discovery and parsing to `serde_json::Value` — M1 check-plan
//! steps 1-2 (docs/wip/2026-09-06-m1-check-plan.md).

mod discover;
mod parse;

pub use discover::{discover_manifest, DiscoverError, ManifestFormat, ManifestPath};
pub use parse::{parse_manifest, ParseError};
