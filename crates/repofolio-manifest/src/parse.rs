//! Manifest parsing to `serde_json::Value` — M1 check-plan step 2
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! Each of the three manifest formats is parsed to the same
//! `serde_json::Value` shape, so everything downstream (schema
//! validation, rule checks) operates on one representation regardless of
//! how the manifest was written on disk. Parse failure is distinct from
//! schema-validation failure (`crate::schema::SchemaError`) — a
//! syntactically broken file never reaches the schema validator.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::discover::ManifestFormat;

/// Manifest parse failure. Maps to `FOLIO-001` (manifest missing or
/// unparseable) once diagnostic codes are assigned in `repofolio-core` —
/// the same code as `crate::DiscoverError`, since both mean "there is no
/// usable manifest to validate."
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to read manifest at {path}: {source}", path = path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse TOML manifest at {path}: {source}", path = path.display())]
    Toml {
        path: PathBuf,
        #[source]
        source: toml_edit::de::Error,
    },

    #[error("failed to parse YAML manifest at {path}: {message}", path = path.display())]
    Yaml { path: PathBuf, message: String },

    #[error("failed to parse JSON manifest at {path}: {source}", path = path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// Parses the manifest at `path`, in the given `format`, to a
/// `serde_json::Value`.
pub fn parse_manifest(path: &Path, format: ManifestFormat) -> Result<Value, ParseError> {
    let text = fs::read_to_string(path).map_err(|source| ParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    match format {
        ManifestFormat::Toml => {
            toml_edit::de::from_str::<Value>(&text).map_err(|source| ParseError::Toml {
                path: path.to_path_buf(),
                source,
            })
        }
        ManifestFormat::Json => {
            serde_json::from_str::<Value>(&text).map_err(|source| ParseError::Json {
                path: path.to_path_buf(),
                source,
            })
        }
        ManifestFormat::Yaml => parse_yaml(&text).map_err(|message| ParseError::Yaml {
            path: path.to_path_buf(),
            message,
        }),
    }
}

fn parse_yaml(text: &str) -> Result<Value, String> {
    let mut docs =
        yaml_rust2::YamlLoader::load_from_str(text).map_err(|source| source.to_string())?;
    let doc = if docs.is_empty() {
        yaml_rust2::Yaml::Null
    } else {
        docs.remove(0)
    };
    Ok(yaml_to_json(doc))
}

fn yaml_to_json(yaml: yaml_rust2::Yaml) -> Value {
    use yaml_rust2::Yaml;

    match yaml {
        Yaml::Real(s) => s
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Yaml::Integer(i) => Value::Number(i.into()),
        Yaml::String(s) => Value::String(s),
        Yaml::Boolean(b) => Value::Bool(b),
        Yaml::Array(items) => Value::Array(items.into_iter().map(yaml_to_json).collect()),
        Yaml::Hash(hash) => Value::Object(
            hash.into_iter()
                .map(|(k, v)| (yaml_key_to_string(k), yaml_to_json(v)))
                .collect(),
        ),
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Value::Null,
    }
}

/// Converts a YAML mapping key to a JSON object key. YAML permits
/// non-string mapping keys; a project manifest never uses them, so any
/// non-string key is rendered through its JSON form rather than rejected.
fn yaml_key_to_string(key: yaml_rust2::Yaml) -> String {
    match key {
        yaml_rust2::Yaml::String(s) => s,
        other => yaml_to_json(other).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_file(case: &str, filename: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "repofolio-manifest-parse-{case}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock is after the epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(filename);
        fs::write(&path, contents).expect("write temp manifest");
        path
    }

    #[test]
    fn same_logical_manifest_parses_identically_across_formats() {
        let toml_path = temp_file(
            "identical-toml",
            "project.toml",
            "name = \"com.example.app\"\nversion = \"1.0.0\"\nkeywords = [\"a\", \"b\"]\n",
        );
        let yaml_path = temp_file(
            "identical-yaml",
            "project.yaml",
            "name: com.example.app\nversion: \"1.0.0\"\nkeywords:\n  - a\n  - b\n",
        );
        let json_path = temp_file(
            "identical-json",
            "project.json",
            r#"{"name": "com.example.app", "version": "1.0.0", "keywords": ["a", "b"]}"#,
        );

        let toml_value = parse_manifest(&toml_path, ManifestFormat::Toml).expect("toml parses");
        let yaml_value = parse_manifest(&yaml_path, ManifestFormat::Yaml).expect("yaml parses");
        let json_value = parse_manifest(&json_path, ManifestFormat::Json).expect("json parses");

        assert_eq!(toml_value, yaml_value);
        assert_eq!(yaml_value, json_value);
    }

    #[test]
    fn invalid_toml_is_a_parse_error() {
        let path = temp_file("invalid-toml", "project.toml", "name = \n");

        let err = parse_manifest(&path, ManifestFormat::Toml).expect_err("invalid toml");

        assert!(matches!(err, ParseError::Toml { .. }));
    }

    #[test]
    fn invalid_yaml_is_a_parse_error() {
        let path = temp_file("invalid-yaml", "project.yaml", "name: [unclosed\n");

        let err = parse_manifest(&path, ManifestFormat::Yaml).expect_err("invalid yaml");

        assert!(matches!(err, ParseError::Yaml { .. }));
    }

    #[test]
    fn invalid_json_is_a_parse_error() {
        let path = temp_file("invalid-json", "project.json", "{ \"name\": }");

        let err = parse_manifest(&path, ManifestFormat::Json).expect_err("invalid json");

        assert!(matches!(err, ParseError::Json { .. }));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let path = std::env::temp_dir().join("repofolio-manifest-parse-does-not-exist.toml");

        let err = parse_manifest(&path, ManifestFormat::Toml).expect_err("file absent");

        assert!(matches!(err, ParseError::Io { .. }));
    }

    /// YAML permits a top-level document that is a scalar or a sequence,
    /// not just a mapping. Parsing does not reject that shape — TOML
    /// cannot express it (a TOML document is always a table) and JSON
    /// defers entirely to `serde_json::Value`, so enforcing "must be an
    /// object" here would make YAML the odd one out. The resulting
    /// non-object `Value` is left for `crate::schema::validate_manifest`
    /// to reject, the same way an out-of-shape JSON or TOML document
    /// would be.
    #[test]
    fn yaml_scalar_document_parses_to_a_non_object_value() {
        let path = temp_file("yaml-scalar", "project.yaml", "just a string\n");

        let value = parse_manifest(&path, ManifestFormat::Yaml).expect("scalar yaml parses");

        assert_eq!(value, Value::String("just a string".to_string()));
    }

    #[test]
    fn yaml_sequence_document_parses_to_an_array_value() {
        let path = temp_file("yaml-sequence", "project.yaml", "- a\n- b\n");

        let value = parse_manifest(&path, ManifestFormat::Yaml).expect("sequence yaml parses");

        assert_eq!(
            value,
            Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string())
            ])
        );
    }

    /// TOML distinguishes an integer literal from a float literal; the
    /// resulting `serde_json::Value::Number` must preserve that
    /// distinction rather than collapsing both to the same JSON number
    /// representation, since a schema (or a future rule) may check
    /// `is_i64()`/`is_f64()`.
    #[test]
    fn toml_integer_and_float_are_distinct_json_numbers() {
        let path = temp_file(
            "int-float",
            "project.toml",
            "int_field = 1\nfloat_field = 1.0\n",
        );

        let value = parse_manifest(&path, ManifestFormat::Toml).expect("toml parses");

        assert!(value["int_field"].is_i64());
        assert_eq!(value["int_field"], serde_json::json!(1));
        assert!(value["float_field"].is_f64());
        assert_eq!(value["float_field"], serde_json::json!(1.0));
        assert_ne!(value["int_field"], value["float_field"]);
    }
}
