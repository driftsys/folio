//! Bundled schema and validation — M1 check-plan step 3
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! The schema is the Repofolio project manifest schema, copied
//! byte-identically from `driftsys/schemas/project/v1.json` into
//! `schema/project-v1.json` and embedded at compile time so `folio check`
//! never fetches it at runtime. `tests/schema_parity.rs` detects drift
//! between the two copies.

use serde_json::Value;

const SCHEMA_JSON: &str = include_str!("../schema/project-v1.json");

/// Schema-validation failure. Maps to `FOLIO-002` (manifest fails the
/// bundled schema) once diagnostic codes are assigned in
/// `repofolio-core` — distinct from `crate::ParseError` (`FOLIO-003`),
/// since a manifest that parses can still fail validation.
///
/// `errors` holds one entry per schema violation, not one entry for the
/// whole failure, so a caller (`repofolio_core::rules::manifest_rules`)
/// can emit one diagnostic per violation rather than collapsing every
/// violation into a single finding. Each entry is prefixed with its
/// JSON-pointer `instance_path` (e.g. `/authors/0: ...`) when that path
/// is non-empty, the same convention `jsonschema`'s own
/// `ValidationErrors` `Display` impl uses — `ValidationError`'s `Display`
/// does not include the path on its own.
#[derive(Debug, thiserror::Error)]
#[error("manifest failed schema validation: {}", errors.join("; "))]
pub struct SchemaError {
    pub errors: Vec<String>,
}

/// Validates a parsed manifest against the bundled project manifest
/// schema (JSON Schema draft-07).
pub fn validate_manifest(value: &Value) -> Result<(), SchemaError> {
    let schema: Value = serde_json::from_str(SCHEMA_JSON).expect("bundled schema is valid JSON");
    let validator =
        jsonschema::validator_for(&schema).expect("bundled schema compiles as draft-07");

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| {
            let instance_path = error.instance_path().to_string();
            if instance_path.is_empty() {
                error.to_string()
            } else {
                format!("{instance_path}: {error}")
            }
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SchemaError { errors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture(name: &str) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/schema")
            .join(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
        serde_json::from_str(&text).expect("fixture is valid JSON")
    }

    #[test]
    fn minimal_manifest_is_valid() {
        assert!(validate_manifest(&fixture("minimal.json")).is_ok());
    }

    #[test]
    fn full_manifest_is_valid() {
        assert!(validate_manifest(&fixture("full.json")).is_ok());
    }

    #[test]
    fn invalid_name_is_rejected() {
        assert!(validate_manifest(&fixture("invalid-name.json")).is_err());
    }

    #[test]
    fn missing_required_is_rejected() {
        assert!(validate_manifest(&fixture("missing-required.json")).is_err());
    }

    /// Pins two properties of `SchemaError.errors` that a single combined
    /// error string cannot give a caller: one entry per violation (so a
    /// caller can emit one diagnostic per violation rather than one for
    /// the whole failure), and each entry naming its own instance path
    /// when that path is non-empty. The `additionalProperties` violation
    /// has no instance path of its own (it applies to the object as a
    /// whole), so it carries no path prefix.
    #[test]
    fn errors_carry_one_entry_per_violation_with_the_instance_path_prefixed() {
        let value = serde_json::json!({
            "name": "com.example.x",
            "version": "1.0.0",
            "authors": [{"name": "Sebastien"}],
            "versioning": "semver"
        });

        let err = validate_manifest(&value).expect_err("two violations expected");

        assert_eq!(err.errors.len(), 2);
        assert!(
            err.errors.iter().any(|e| e.starts_with("/authors/0: ")),
            "expected an /authors/0-prefixed entry, got {:?}",
            err.errors
        );
        assert!(
            err.errors
                .iter()
                .any(|e| e.contains("versioning") && !e.starts_with('/')),
            "expected an unprefixed additionalProperties entry, got {:?}",
            err.errors
        );
    }
}
