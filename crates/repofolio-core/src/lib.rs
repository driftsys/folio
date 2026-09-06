//! repofolio-core — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! The diagnostic report shape (M1 check-plan step 5) and the ecosystem
//! registry (M1 check-plan step 6) are frozen at v1.0 — see
//! docs/archive/plans/2026-09-06-m1-check-plan.md. The `FOLIO-` rule registry
//! (step 7) and the check pipeline that runs them in order (step 8)
//! build on both without reshaping them.

mod ecosystem;
mod pipeline;
mod registry;
mod report;
mod rules;

pub use ecosystem::{
    detect_rust, repofolio_ecosystem, rust_ecosystem, Commands, Ecosystem, EcosystemMeta,
    MarkerSpec, Markers,
};
pub use pipeline::check;
pub use registry::{code_registry, ecosystem_registry, CodeEntry, Registry};
pub use report::{BySeverity, Diagnostic, Location, Report, Severity};
pub use rules::{manifest_rules, path_rules};
