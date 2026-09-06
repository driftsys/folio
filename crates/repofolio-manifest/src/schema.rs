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
/// `repofolio-core` — distinct from `crate::ParseError` (`FOLIO-001`),
/// since a manifest that parses can still fail validation.
#[derive(Debug, thiserror::Error)]
#[error("manifest failed schema validation: {errors:?}")]
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
        .map(|error| error.to_string())
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
}
