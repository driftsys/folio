//! The `FOLIO-` diagnostic code registry and the ecosystem registry, as
//! data — the tool-contract clause CLAUDE.md's non-negotiable
//! architecture decision 6 requires of every orchestrated tool, folio
//! included, from v0.1 onward: "a `<tool> registry --format json` dump".
//! Both registries already existed as data before this module (the code
//! table in `rules.rs`'s module doc, the two `Ecosystem` values in
//! `ecosystem.rs`); this only assembles them into one serializable type
//! for `folio registry --format json` to print. Per the drift rule this
//! decision states elsewhere: registries hold facts, MCP serves facts,
//! skills hold procedure — this type is the fact.

use serde::{Deserialize, Serialize};

use crate::ecosystem::{repofolio_ecosystem, rust_ecosystem, Ecosystem};
use crate::report::Severity;

/// One entry in the `FOLIO-` code registry: the code's nominal severity
/// and a human-readable description of the condition it reports.
/// `FOLIO-002`'s info-severity "skipped" case is a message-level nuance
/// of one particular finding, not a fact about the code itself, so its
/// nominal severity here is `Error` — the severity it reports at when it
/// actually fires against a parsed manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeEntry {
    pub code: String,
    pub severity: Severity,
    pub description: String,
}

/// Every `FOLIO-` code this milestone defines, in numeric order. Kept in
/// one place so `folio registry` and `rules.rs`'s module doc describe
/// the same five codes; if a code is ever added or its meaning changes,
/// both need updating together.
pub fn code_registry() -> Vec<CodeEntry> {
    vec![
        CodeEntry {
            code: "FOLIO-001".to_string(),
            severity: Severity::Error,
            description: "Manifest missing: none of project.toml, project.yaml, \
                or project.json exists at the repository root."
                .to_string(),
        },
        CodeEntry {
            code: "FOLIO-002".to_string(),
            severity: Severity::Error,
            description: "Manifest parses but fails the bundled schema. Reported \
                at info severity, with a message naming the reason, when \
                FOLIO-001 or FOLIO-003 has already failed and there is nothing \
                to validate."
                .to_string(),
        },
        CodeEntry {
            code: "FOLIO-003".to_string(),
            severity: Severity::Error,
            description: "A manifest file was found but is unreadable or fails \
                to parse in its own format (malformed TOML, unparseable YAML, \
                malformed JSON)."
                .to_string(),
        },
        CodeEntry {
            code: "FOLIO-101".to_string(),
            severity: Severity::Error,
            description: "A required path for an active layer is absent from \
                the repository."
                .to_string(),
        },
        CodeEntry {
            code: "FOLIO-102".to_string(),
            severity: Severity::Warning,
            description: "A recommended path for an active layer is absent \
                from the repository."
                .to_string(),
        },
    ]
}

/// Every ecosystem this milestone knows about, active in the current
/// repository or not — the same two instances `pipeline::check` uses,
/// so the registry dump and the actual check pipeline can never
/// describe two different sets of ecosystems.
pub fn ecosystem_registry() -> Vec<Ecosystem> {
    vec![repofolio_ecosystem(), rust_ecosystem()]
}

/// The full registry dump: `folio registry --format json`. Frozen in
/// spirit like [`crate::Report`] — a consumer parses this once and
/// should not need reshaping across a minor version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub codes: Vec<CodeEntry>,
    pub ecosystems: Vec<Ecosystem>,
}

impl Registry {
    /// Builds the registry from [`code_registry`] and
    /// [`ecosystem_registry`] — the only constructor, so the two can
    /// never drift from what those functions actually return.
    pub fn new() -> Self {
        Registry {
            codes: code_registry(),
            ecosystems: ecosystem_registry(),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_registry_has_exactly_the_five_milestone_codes_in_order() {
        let entries = code_registry();
        let codes: Vec<&str> = entries.iter().map(|e| e.code.as_str()).collect();
        assert_eq!(
            codes,
            vec![
                "FOLIO-001",
                "FOLIO-002",
                "FOLIO-003",
                "FOLIO-101",
                "FOLIO-102"
            ]
        );
    }

    /// Pins each code's severity individually — `code_registry_has_
    /// exactly_the_five_milestone_codes_in_order` only checks the code
    /// strings, and the pinned-JSON-shape test below only spot-checks
    /// `FOLIO-001`, so neither would catch e.g. `FOLIO-102` (the one
    /// warning-severity code) silently becoming `Error`.
    #[test]
    fn every_code_has_its_documented_severity() {
        let entries = code_registry();
        let severities: Vec<(&str, Severity)> = entries
            .iter()
            .map(|e| (e.code.as_str(), e.severity))
            .collect();
        assert_eq!(
            severities,
            vec![
                ("FOLIO-001", Severity::Error),
                ("FOLIO-002", Severity::Error),
                ("FOLIO-003", Severity::Error),
                ("FOLIO-101", Severity::Error),
                ("FOLIO-102", Severity::Warning),
            ]
        );
    }

    #[test]
    fn ecosystem_registry_has_exactly_repofolio_and_rust() {
        let names: Vec<String> = ecosystem_registry()
            .iter()
            .map(|e| e.ecosystem.name.clone())
            .collect();
        assert_eq!(names, vec!["repofolio", "rust"]);
    }

    #[test]
    fn registry_new_matches_the_two_registries_it_assembles() {
        let registry = Registry::new();
        assert_eq!(registry.codes, code_registry());
        assert_eq!(registry.ecosystems, ecosystem_registry());
    }

    /// Pins the serialised wire shape, the same discipline
    /// `report.rs`'s `report_serializes_to_the_pinned_json_shape` test
    /// applies to `Report`: an unintended shape change fails this test
    /// rather than only a `Debug` comparison.
    #[test]
    fn registry_serializes_to_the_pinned_json_shape() {
        let registry = Registry::new();

        let actual = serde_json::to_string_pretty(&registry).expect("registry serializes");
        let value: serde_json::Value = serde_json::from_str(&actual).expect("valid json");

        let codes = value["codes"].as_array().expect("codes is an array");
        assert_eq!(codes.len(), 5);
        assert_eq!(codes[0]["code"], "FOLIO-001");
        assert_eq!(codes[0]["severity"], "error");
        assert!(codes[0]["description"].is_string());

        let ecosystems = value["ecosystems"].as_array().expect("ecosystems array");
        assert_eq!(ecosystems.len(), 2);
        assert_eq!(ecosystems[0]["ecosystem"]["name"], "repofolio");
        assert_eq!(ecosystems[1]["ecosystem"]["name"], "rust");
    }
}
