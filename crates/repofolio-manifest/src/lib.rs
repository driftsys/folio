//! repofolio-manifest — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! Discovery, parsing and schema validation of the project manifest — M1
//! check-plan steps 1-4 (docs/wip/2026-09-06-m1-check-plan.md). Parse
//! failure (`ParseError`) and schema-validation failure (`SchemaError`)
//! are distinct types: they map to different diagnostic codes
//! (`FOLIO-001` and `FOLIO-002`) once `repofolio-core` assigns them.

mod discover;
mod parse;
mod schema;

pub use discover::{discover_manifest, DiscoverError, ManifestFormat, ManifestPath};
pub use parse::{parse_manifest, ParseError};
pub use schema::{validate_manifest, SchemaError};
