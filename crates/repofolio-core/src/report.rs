//! Diagnostic report type — M1 check-plan step 5
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! Mirrors the house diagnostics shape at
//! `schemas/markspec/diagnostics/v1.json` (`count`, `bySeverity`,
//! `diagnostics[]`, `severity`/`code`/`message`/`location.file`) so that
//! every driftsys tool emits the same wire shape and SARIF can later be
//! added as a serialiser rather than as a breaking change to this
//! contract. This type is frozen at v1.0: `folio-cli` (step 9) and every
//! rule added in step 7 build on it without reshaping it.
//!
//! Two departures from the markspec schema, both deliberate:
//!
//! - `location.line` and `location.column` are optional here, where
//!   markspec requires `line`. A path-presence finding (`FOLIO-101`,
//!   `FOLIO-102`) names a file, not a line; a future content-match rule
//!   (Part 6 of the task-model design) can still populate them without a
//!   shape change.
//! - `Diagnostic` carries an additional `layer` field with no equivalent
//!   in the markspec schema, because `FOLIO-101`/`FOLIO-102` are emitted
//!   once per active layer (`repofolio`, `rust`, ...) and a finding must
//!   say which one produced it. It is optional and omitted from the JSON
//!   when absent (manifest-level codes such as `FOLIO-001`/`FOLIO-002`
//!   are not layer-specific), which keeps it an additive field a SARIF
//!   serialiser can carry in its `properties` bag.

use serde::{Deserialize, Serialize};

/// A diagnostic report: the wire shape of `folio check --format json`.
///
/// `count` and `by_severity` are derived from `diagnostics` — construct
/// a `Report` through [`Report::new`] rather than assembling the fields
/// by hand, so the two can never drift out of sync with the list they
/// summarize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub count: usize,
    pub by_severity: BySeverity,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// Builds a `Report` from its findings, computing `count` and
    /// `by_severity` from the list rather than accepting them as
    /// separate inputs that could disagree with it.
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        let mut by_severity = BySeverity::default();
        for diagnostic in &diagnostics {
            match diagnostic.severity {
                Severity::Error => by_severity.error += 1,
                Severity::Warning => by_severity.warning += 1,
                Severity::Info => by_severity.info += 1,
            }
        }

        Report {
            count: diagnostics.len(),
            by_severity,
            diagnostics,
        }
    }
}

/// Finding counts broken down by severity. Field names match the
/// mirrored schema exactly (`error`, `warning`, `info`) with no renaming
/// needed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BySeverity {
    pub error: usize,
    pub warning: usize,
    pub info: usize,
}

/// One finding. `code` stays a plain string rather than a closed enum:
/// ecosystem-specific codes (`RUST-001`, `UNITY-001`, Part 6 of the
/// task-model design) flow through the same type as the core `FOLIO-xxx`
/// codes, and plugin ecosystems can never mint a fixed Rust variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    /// The active layer (ecosystem name, e.g. `"repofolio"` or
    /// `"rust"`) that produced this finding, when the finding is
    /// per-layer. `None` for manifest-level codes (`FOLIO-001`,
    /// `FOLIO-002`), which apply once to the whole repository rather
    /// than once per active ecosystem.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub layer: Option<String>,
    pub location: Location,
}

/// Severity is exactly these three variants — no fourth. A skipped
/// check (`FOLIO-002` when `FOLIO-001` already failed) is reported at
/// `Info`, not a separate "skipped" variant: this keeps exit-code
/// grading (error present -> 1, warnings/info alone -> 0) a matter of
/// counting three buckets, and maps cleanly to a future SARIF `level`
/// of `note` with `kind: notApplicable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Where a finding applies. `file` is the only field every finding can
/// supply; `line`/`column` are absent for path-presence findings and
/// reserved for a future rule kind (e.g. `content-match`) that can
/// point at a specific position without a shape change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub column: Option<u32>,
}

impl Location {
    /// A location naming only a file, the common case for the
    /// path-presence rules (`FOLIO-101`, `FOLIO-102`) and the
    /// manifest rules (`FOLIO-001`, `FOLIO-002`).
    pub fn file(file: impl Into<String>) -> Self {
        Location {
            file: file.into(),
            line: None,
            column: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_diagnostics() -> Vec<Diagnostic> {
        vec![
            Diagnostic {
                severity: Severity::Error,
                code: "FOLIO-101".to_string(),
                message: "required path missing: README.md".to_string(),
                layer: Some("repofolio".to_string()),
                location: Location::file("README.md"),
            },
            Diagnostic {
                severity: Severity::Warning,
                code: "FOLIO-102".to_string(),
                message: "recommended path missing: CHANGELOG.md".to_string(),
                layer: Some("repofolio".to_string()),
                location: Location::file("CHANGELOG.md"),
            },
            Diagnostic {
                severity: Severity::Info,
                code: "FOLIO-002".to_string(),
                message: "skipped: FOLIO-001 already failed, nothing to validate".to_string(),
                layer: None,
                location: Location::file("project.toml"),
            },
        ]
    }

    #[test]
    fn count_and_by_severity_are_derived_from_diagnostics() {
        let report = Report::new(sample_diagnostics());

        assert_eq!(report.count, 3);
        assert_eq!(
            report.by_severity,
            BySeverity {
                error: 1,
                warning: 1,
                info: 1,
            }
        );
    }

    #[test]
    fn empty_report_has_zero_counts() {
        let report = Report::new(vec![]);

        assert_eq!(report.count, 0);
        assert_eq!(report.by_severity, BySeverity::default());
    }

    /// Pins the serialised wire shape: field names, camelCase
    /// `bySeverity`, lowercase severities, and that `layer` is omitted
    /// from the JSON when absent rather than serialised as `null`. Any
    /// unintended change to this shape — the actual purpose of this
    /// batch — fails this test rather than merely a `Debug` comparison.
    #[test]
    fn report_serializes_to_the_pinned_json_shape() {
        let report = Report::new(sample_diagnostics());

        let actual = serde_json::to_string_pretty(&report).expect("report serializes");

        let expected = r#"{
  "count": 3,
  "bySeverity": {
    "error": 1,
    "warning": 1,
    "info": 1
  },
  "diagnostics": [
    {
      "severity": "error",
      "code": "FOLIO-101",
      "message": "required path missing: README.md",
      "layer": "repofolio",
      "location": {
        "file": "README.md"
      }
    },
    {
      "severity": "warning",
      "code": "FOLIO-102",
      "message": "recommended path missing: CHANGELOG.md",
      "layer": "repofolio",
      "location": {
        "file": "CHANGELOG.md"
      }
    },
    {
      "severity": "info",
      "code": "FOLIO-002",
      "message": "skipped: FOLIO-001 already failed, nothing to validate",
      "location": {
        "file": "project.toml"
      }
    }
  ]
}"#;

        assert_eq!(actual, expected);
    }

    #[test]
    fn location_with_line_and_column_round_trips() {
        let location = Location {
            file: "src/lib.rs".to_string(),
            line: Some(12),
            column: Some(5),
        };

        let value = serde_json::to_value(&location).expect("location serializes");

        assert_eq!(
            value,
            serde_json::json!({"file": "src/lib.rs", "line": 12, "column": 5})
        );
    }
}
