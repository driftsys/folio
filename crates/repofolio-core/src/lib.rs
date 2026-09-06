//! repofolio-core — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! The diagnostic report shape (M1 check-plan step 5) and the ecosystem
//! registry (M1 check-plan step 6) — see
//! docs/wip/2026-09-06-m1-check-plan.md. Both are frozen at v1.0: later
//! milestones build on their shape rather than reshaping them.

mod ecosystem;
mod report;

pub use ecosystem::{
    detect_rust, repofolio_ecosystem, rust_ecosystem, Commands, Ecosystem, EcosystemMeta, Markers,
};
pub use report::{BySeverity, Diagnostic, Location, Report, Severity};
