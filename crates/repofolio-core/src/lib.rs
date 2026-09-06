//! repofolio-core — see docs/planning/folio-plan.md for this crate scope
//! within the folio workspace (Integration architecture, M0 item 1).
//!
//! The diagnostic report shape — M1 check-plan step 5, see
//! docs/wip/2026-09-06-m1-check-plan.md. Frozen at v1.0: later milestones
//! build on this shape rather than reshaping it.

mod report;

pub use report::{BySeverity, Diagnostic, Location, Report, Severity};
