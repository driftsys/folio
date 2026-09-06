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

/// Manifest parse failure. Maps to `FOLIO-003` (manifest unparseable) —
/// distinct from `crate::DiscoverError`'s `FOLIO-001` (manifest missing),
/// since discovery and parse failures are different codes.
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
    yaml_to_json(doc)
}

/// Converts one YAML node to its `serde_json::Value` equivalent.
///
/// This returns `Result` rather than always succeeding because
/// `yaml_rust2` itself flags two shapes as invalid rather than rejecting
/// them at load time: a `Real` scalar whose text does not parse as a
/// finite `f64` (for example YAML 1.1's `.nan`/`.inf` notation, which
/// `str::parse::<f64>` does not accept, or a literal large enough to
/// parse to an infinite float), and `BadValue`, which `yaml_rust2` uses
/// for an anchor alias it could not resolve (for instance a
/// self-referential anchor). Converting either to `Value::Null` would
/// turn a malformed manifest into one that merely looks empty at that
/// field — and since the manifest schema's `metadata` object allows any
/// properties (`additionalProperties: true`), a null there would pass
/// schema validation instead of being caught as a parse failure.
fn yaml_to_json(yaml: yaml_rust2::Yaml) -> Result<Value, String> {
    use yaml_rust2::Yaml;

    match yaml {
        Yaml::Real(s) => {
            let f = s
                .parse::<f64>()
                .map_err(|_| format!("invalid YAML floating-point literal: {s:?}"))?;
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .ok_or_else(|| format!("YAML floating-point literal is not finite: {s:?}"))
        }
        Yaml::Integer(i) => Ok(Value::Number(i.into())),
        Yaml::String(s) => Ok(Value::String(s)),
        Yaml::Boolean(b) => Ok(Value::Bool(b)),
        Yaml::Array(items) => items
            .into_iter()
            .map(yaml_to_json)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Yaml::Hash(hash) => hash
            .into_iter()
            .map(|(k, v)| Ok((yaml_key_to_string(k)?, yaml_to_json(v)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        Yaml::Null => Ok(Value::Null),
        // `Alias` is documented by yaml_rust2 as "not fully supported
        // yet" and is not reachable from `YamlLoader::load_from_str`
        // output (an alias is always resolved to its target value, or
        // to `BadValue` if that fails) — handled alongside `BadValue`
        // rather than silently nulled out, for the same reason.
        Yaml::Alias(_) => Err("YAML alias is not supported".to_string()),
        Yaml::BadValue => {
            Err("YAML contains an unresolved value (e.g. a self-referential anchor)".to_string())
        }
    }
}

/// Converts a YAML mapping key to a JSON object key. YAML permits
/// non-string mapping keys; a project manifest never uses them, so any
/// non-string key is rendered through its JSON form rather than rejected.
fn yaml_key_to_string(key: yaml_rust2::Yaml) -> Result<String, String> {
    match key {
        yaml_rust2::Yaml::String(s) => Ok(s),
        other => yaml_to_json(other).map(|v| v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Creates an isolated temporary directory and writes one file into
    /// it. Returns the directory guard alongside the file's path — the
    /// guard must be kept bound in the caller (even as `_dir`) because
    /// dropping it removes the directory.
    fn temp_file(filename: &str, contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join(filename);
        fs::write(&path, contents).expect("write temp manifest");
        (dir, path)
    }

    #[test]
    fn same_logical_manifest_parses_identically_across_formats() {
        let (_toml_dir, toml_path) = temp_file(
            "project.toml",
            "name = \"com.example.app\"\nversion = \"1.0.0\"\nkeywords = [\"a\", \"b\"]\n",
        );
        let (_yaml_dir, yaml_path) = temp_file(
            "project.yaml",
            "name: com.example.app\nversion: \"1.0.0\"\nkeywords:\n  - a\n  - b\n",
        );
        let (_json_dir, json_path) = temp_file(
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
        let (_dir, path) = temp_file("project.toml", "name = \n");

        let err = parse_manifest(&path, ManifestFormat::Toml).expect_err("invalid toml");

        assert!(matches!(err, ParseError::Toml { .. }));
    }

    #[test]
    fn invalid_yaml_is_a_parse_error() {
        let (_dir, path) = temp_file("project.yaml", "name: [unclosed\n");

        let err = parse_manifest(&path, ManifestFormat::Yaml).expect_err("invalid yaml");

        assert!(matches!(err, ParseError::Yaml { .. }));
    }

    #[test]
    fn invalid_json_is_a_parse_error() {
        let (_dir, path) = temp_file("project.json", "{ \"name\": }");

        let err = parse_manifest(&path, ManifestFormat::Json).expect_err("invalid json");

        assert!(matches!(err, ParseError::Json { .. }));
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("does-not-exist.toml");

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
        let (_dir, path) = temp_file("project.yaml", "just a string\n");

        let value = parse_manifest(&path, ManifestFormat::Yaml).expect("scalar yaml parses");

        assert_eq!(value, Value::String("just a string".to_string()));
    }

    #[test]
    fn yaml_sequence_document_parses_to_an_array_value() {
        let (_dir, path) = temp_file("project.yaml", "- a\n- b\n");

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
        let (_dir, path) = temp_file("project.toml", "int_field = 1\nfloat_field = 1.0\n");

        let value = parse_manifest(&path, ManifestFormat::Toml).expect("toml parses");

        assert!(value["int_field"].is_i64());
        assert_eq!(value["int_field"], serde_json::json!(1));
        assert!(value["float_field"].is_f64());
        assert_eq!(value["float_field"], serde_json::json!(1.0));
        assert_ne!(value["int_field"], value["float_field"]);
    }

    /// YAML 1.1's `.nan`/`.inf` notation is a real authoring mistake a
    /// manifest author could make expecting it to behave like a normal
    /// float: `yaml_rust2` tags it as a `Real` scalar, but
    /// `str::parse::<f64>` does not accept the leading dot, so the
    /// literal fails to parse as a float. Before this fix, that failure
    /// was swallowed to `Value::Null` instead of surfacing as a parse
    /// error.
    #[test]
    fn yaml_unparseable_real_literal_is_a_parse_error() {
        let (_dir, path) = temp_file("project.yaml", "value: .nan\n");

        let err = parse_manifest(&path, ManifestFormat::Yaml).expect_err("unparseable real");

        assert!(matches!(err, ParseError::Yaml { .. }));
    }

    /// A `Real` literal can parse successfully as `f64` yet not be
    /// representable as JSON: JSON numbers must be finite, and an
    /// oversized exponent like `1e400` parses to `f64::INFINITY` rather
    /// than failing. Before this fix, that non-finite value was
    /// swallowed to `Value::Null` instead of surfacing as a parse error.
    #[test]
    fn yaml_non_finite_real_is_a_parse_error() {
        let (_dir, path) = temp_file("project.yaml", "value: 1e400\n");

        let err = parse_manifest(&path, ManifestFormat::Yaml).expect_err("non-finite real");

        assert!(matches!(err, ParseError::Yaml { .. }));
    }

    /// A self-referential anchor (the alias is used before the anchored
    /// node it points to has finished being parsed) is exactly the case
    /// `yaml_rust2` represents as `Yaml::BadValue` rather than rejecting
    /// at load time. Nested under `metadata`, whose schema allows any
    /// properties, this is the concrete scenario a swallowed-to-null
    /// conversion would let through schema validation undetected.
    #[test]
    fn yaml_bad_value_from_self_referential_anchor_is_a_parse_error() {
        let (_dir, path) = temp_file(
            "project.yaml",
            "name: com.example.app\nversion: \"1.0.0\"\nmetadata:\n  self: &loop\n    ref: *loop\n",
        );

        let err = parse_manifest(&path, ManifestFormat::Yaml).expect_err("bad value from anchor");

        assert!(matches!(err, ParseError::Yaml { .. }));
    }
}
