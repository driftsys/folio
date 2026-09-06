//! Ecosystem registry — M1 check-plan step 6
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! `Ecosystem` mirrors the on-disk `folio.ecosystem.toml` envelope
//! described in Part 6 of `docs/wip/2026-09-06-folio-task-model-design.md`
//! field for field, so the v0.4 ecosystem loader deserializes a file
//! straight into this type with no translation layer:
//!
//! ```toml
//! "$schema" = "https://driftsys.github.io/schemas/folio-ecosystem/v1.json"
//!
//! [ecosystem]
//! name = "rust"
//! version = 1
//!
//! [markers]
//! must = ["Cargo.toml"]
//!
//! [commands.build]
//! debug = "cargo build"
//! ```
//!
//! `name` and `version` are nested under `ecosystem: EcosystemMeta`
//! because the file nests them under `[ecosystem]`; a flat `name` field
//! on `Ecosystem` itself could not deserialize that table without a
//! second, parallel implementation doing the translation — exactly what
//! this mirror exists to avoid. `version` is the ecosystem *file format*
//! version (an integer the design doc's own example sets to `1`), not a
//! semantic version of the ecosystem's tooling.
//!
//! `always` is deliberately **not** part of this mirror: it is
//! `#[serde(skip)]`, so no `folio.ecosystem.toml` — including one fetched
//! from a third-party remote source (Part 6's pinned, checksummed remote
//! ecosystem source) — can set it. Granting an ecosystem unconditional
//! activation is a privilege only the built-in `repofolio` registration
//! exercises, in code, never through a file a plugin author controls.
//!
//! `schema` mirrors the file's top-level `"$schema"` key. It is carried
//! (as an optional field, so the two M1 instances below need not set it)
//! rather than dropped, for two reasons: dropping it would make a real
//! `folio.ecosystem.toml` fail to round-trip through this type, and its
//! value is how a v0.4 loader will eventually tell which schema version a
//! given file was authored against.
//!
//! M1 populates two instances by hand — `repofolio` (always active) and
//! `rust` (activated by a root `Cargo.toml` declaring `[workspace]`) —
//! with empty `markers` and `commands`. Step 7 fills in the required and
//! recommended paths; this step only fixes the shape.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One ecosystem: the core `repofolio` layer, or an ecosystem such as
/// `rust` that only applies when detected in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ecosystem {
    /// Mirrors the file's top-level `"$schema"` key. See the module
    /// doc comment for why this is carried rather than dropped.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub ecosystem: EcosystemMeta,
    /// `true` only for `repofolio`: the one layer with no marker to
    /// detect, active in every repository unconditionally.
    /// `#[serde(skip)]` rather than `#[serde(default)]`: a `default`
    /// still lets an incoming file set the field explicitly (just
    /// supplies a fallback when it is absent), whereas `skip` removes it
    /// from the deserialized schema entirely, so an `always = true` key
    /// in a loaded `folio.ecosystem.toml` is silently not a field this
    /// type reads. No ecosystem loaded from a file — built-in, local, or
    /// remote — can grant itself unconditional activation this way.
    #[serde(skip)]
    pub always: bool,
    pub markers: Markers,
    /// Empty for both M1 instances. Present in the type because v0.4
    /// ecosystems (and the future `folio dispatch` verbs) need it, and
    /// adding a field to a type already frozen at v1.0 would be a
    /// breaking change.
    #[serde(default)]
    pub commands: Commands,
}

/// The `[ecosystem]` table: identity and file-format version, nested to
/// match the on-disk envelope exactly (see the module doc comment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcosystemMeta {
    pub name: String,
    /// The `folio.ecosystem.toml` format version, e.g. `1` — not a
    /// semantic version of the ecosystem's own tooling.
    pub version: u32,
}

/// Paths that give both ecosystem detection and the `FOLIO-101`
/// (required) / `FOLIO-102` (recommended) conformance rules (Part 6 of
/// the task-model design). Empty for both M1 instances — `rust`'s
/// activation rule is the dedicated `[workspace]` content check below,
/// not marker presence, and step 7 is where the required/recommended
/// path lists themselves are added.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Markers {
    #[serde(default)]
    pub must: Vec<String>,
    #[serde(default)]
    pub should: Vec<String>,
}

/// The `commands` table: verb -> build_type -> command
/// (`commands.build.debug = "cargo build"`), for most ecosystems. The C
/// ecosystem needs one more level, keyed by build system, before it
/// reaches a verb (`commands.cmake.build.debug = "..."`).
///
/// Rather than fix `Commands` at two levels of nesting and reshape it
/// when C is implemented, this type is a recursive tree of string keys
/// bottoming out in a command string at whatever depth an ecosystem
/// needs: two levels for `rust`/`deno`, three for `c`. The type does not
/// know which level means "verb" versus "build system" — that
/// interpretation belongs to whatever walks the tree to run a command
/// (a later milestone), so adding C's extra level is a matter of a
/// deeper `folio.ecosystem.toml` file, never a change to this enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Commands {
    Command(String),
    Group(BTreeMap<String, Commands>),
}

impl Default for Commands {
    /// The empty group, used by both M1 instances: `commands` is
    /// present in the type but carries no entries yet.
    fn default() -> Self {
        Commands::Group(BTreeMap::new())
    }
}

/// The `repofolio` ecosystem: always active, no marker to detect. Not
/// loaded from any `folio.ecosystem.toml` file, so `schema` is `None`.
pub fn repofolio_ecosystem() -> Ecosystem {
    Ecosystem {
        schema: None,
        ecosystem: EcosystemMeta {
            name: "repofolio".to_string(),
            version: 1,
        },
        always: true,
        markers: Markers::default(),
        commands: Commands::default(),
    }
}

/// The `rust` ecosystem: not always active, detected by
/// [`detect_rust`]. Not loaded from any `folio.ecosystem.toml` file, so
/// `schema` is `None`.
pub fn rust_ecosystem() -> Ecosystem {
    Ecosystem {
        schema: None,
        ecosystem: EcosystemMeta {
            name: "rust".to_string(),
            version: 1,
        },
        always: false,
        markers: Markers::default(),
        commands: Commands::default(),
    }
}

/// Detects whether the `rust` ecosystem is active: a root `Cargo.toml`
/// declaring a `[workspace]` table.
///
/// A `Cargo.toml` alone is not the signal — a single, non-workspace
/// crate has one too, and must not activate the ecosystem — so this is
/// a dedicated content check rather than a `markers.must` path-presence
/// test. A missing or unparseable `Cargo.toml` is treated as "not
/// detected" rather than an error: manifest-shaped failures are the
/// concern of `FOLIO-001`/`FOLIO-002` against the *project* manifest,
/// not this activation check.
pub fn detect_rust(repo_root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(repo_root.join("Cargo.toml")) else {
        return false;
    };
    let Ok(doc) = text.parse::<toml_edit::DocumentMut>() else {
        return false;
    };
    doc.contains_key("workspace")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repofolio_is_always_active_with_no_markers() {
        let ecosystem = repofolio_ecosystem();

        assert_eq!(ecosystem.ecosystem.name, "repofolio");
        assert!(ecosystem.always);
        assert_eq!(ecosystem.markers, Markers::default());
        assert_eq!(ecosystem.commands, Commands::default());
    }

    #[test]
    fn rust_is_not_always_active() {
        let ecosystem = rust_ecosystem();

        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert!(!ecosystem.always);
    }

    #[test]
    fn detects_workspace_cargo_toml() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\"]\n",
        )
        .unwrap();

        assert!(detect_rust(temp.path()));
    }

    #[test]
    fn does_not_detect_non_workspace_cargo_toml() {
        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        assert!(!detect_rust(temp.path()));
    }

    #[test]
    fn does_not_detect_when_cargo_toml_is_absent() {
        let temp = tempfile::tempdir().expect("create temp dir");

        assert!(!detect_rust(temp.path()));
    }

    /// Pins that `Commands` nests to arbitrary depth without a new
    /// type: a `rust`-shaped two-level `verb -> build_type -> command`
    /// table and a `c`-shaped three-level
    /// `build_system -> verb -> build_type -> command` table both
    /// round-trip through the same `Commands` enum.
    #[test]
    fn commands_nest_to_arbitrary_depth_for_the_c_ecosystem() {
        let two_level_json = serde_json::json!({
            "build": { "debug": "cargo build", "release": "cargo build --release" }
        });
        let three_level_json = serde_json::json!({
            "cmake": { "build": { "debug": "cmake --build build --config Debug" } },
            "meson": { "build": { "debug": "meson compile -C build" } }
        });

        let two_level: Commands =
            serde_json::from_value(two_level_json.clone()).expect("two-level commands parse");
        let three_level: Commands =
            serde_json::from_value(three_level_json.clone()).expect("three-level commands parse");

        assert_eq!(
            serde_json::to_value(&two_level).expect("serializes"),
            two_level_json
        );
        assert_eq!(
            serde_json::to_value(&three_level).expect("serializes"),
            three_level_json
        );
    }

    /// Proves the mirror actually holds: the literal on-disk shape from
    /// Part 6 of the task-model design (`"$schema"`, `[ecosystem]` with
    /// `name`/`version`, `[markers]`, `[commands.<verb>]`) deserializes
    /// directly into `Ecosystem` with no glue code.
    #[test]
    fn deserializes_the_real_on_disk_ecosystem_file_shape() {
        let toml = r#"
"$schema" = "https://driftsys.github.io/schemas/folio-ecosystem/v1.json"

[ecosystem]
name = "rust"
version = 1

[markers]
must = ["Cargo.toml"]
should = ["rust-toolchain.toml"]

[commands.build]
debug = "cargo build"
release = "cargo build --release"
"#;

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml).expect("on-disk shape parses");

        assert_eq!(
            ecosystem.schema.as_deref(),
            Some("https://driftsys.github.io/schemas/folio-ecosystem/v1.json")
        );
        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert_eq!(ecosystem.ecosystem.version, 1);
        assert_eq!(ecosystem.markers.must, vec!["Cargo.toml".to_string()]);
        assert_eq!(
            ecosystem.markers.should,
            vec!["rust-toolchain.toml".to_string()]
        );
        // Not part of the file format: always is never set by a loaded file.
        assert!(!ecosystem.always);
    }

    /// A malicious or careless `folio.ecosystem.toml` cannot grant
    /// itself unconditional activation: `always` is `#[serde(skip)]`,
    /// so an `always = true` key in the input is not read as this
    /// field at all — it is simply an unrecognized key the (permissive,
    /// non-`deny_unknown_fields`) deserializer ignores.
    #[test]
    fn always_cannot_be_set_from_a_loaded_file() {
        let toml = r#"
[ecosystem]
name = "evil"
version = 1

always = true

[markers]
"#;

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml).expect("file parses");

        assert!(!ecosystem.always);
    }
}
