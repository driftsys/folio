//! Ecosystem registry — M1 check-plan step 6
//! (docs/wip/2026-09-06-m1-check-plan.md).
//!
//! `Ecosystem` mirrors the on-disk `folio.ecosystem.toml` format
//! described in Part 6 of `docs/wip/2026-09-06-folio-task-model-design.md`,
//! so that the v0.4 ecosystem loader deserializes a file straight into
//! this type rather than building a second, parallel representation of
//! the same data. The file wraps `name` (and a format `version` this
//! crate does not need) in an `[ecosystem]` table; loading that envelope
//! is a one-line extraction the future loader does, not a reason to
//! duplicate the shape of `markers` or `commands`, which is the part
//! worth protecting from drift.
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
    pub name: String,
    /// `true` only for `repofolio`: the one layer with no marker to
    /// detect, active in every repository unconditionally. A loaded
    /// `folio.ecosystem.toml` never sets this — it defaults to `false`
    /// for every ecosystem detected by markers instead.
    #[serde(default)]
    pub always: bool,
    pub markers: Markers,
    /// Empty for both M1 instances. Present in the type because v0.4
    /// ecosystems (and the future `folio dispatch` verbs) need it, and
    /// adding a field to a type already frozen at v1.0 would be a
    /// breaking change.
    #[serde(default)]
    pub commands: Commands,
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

/// The `repofolio` ecosystem: always active, no marker to detect.
pub fn repofolio_ecosystem() -> Ecosystem {
    Ecosystem {
        name: "repofolio".to_string(),
        always: true,
        markers: Markers::default(),
        commands: Commands::default(),
    }
}

/// The `rust` ecosystem: not always active, detected by
/// [`detect_rust`].
pub fn rust_ecosystem() -> Ecosystem {
    Ecosystem {
        name: "rust".to_string(),
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

        assert_eq!(ecosystem.name, "repofolio");
        assert!(ecosystem.always);
        assert_eq!(ecosystem.markers, Markers::default());
        assert_eq!(ecosystem.commands, Commands::default());
    }

    #[test]
    fn rust_is_not_always_active() {
        let ecosystem = rust_ecosystem();

        assert_eq!(ecosystem.name, "rust");
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
}
