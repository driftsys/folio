//! Ecosystem registry — M1 check-plan step 6
//! (docs/archive/plans/2026-09-06-m1-check-plan.md).
//!
//! `Ecosystem` mirrors the v0.1 scaffold envelope of the on-disk
//! `folio.ecosystem.toml` format described in Part 6 of
//! `docs/archive/specs/2026-09-06-folio-task-model-design.md` — `"$schema"`,
//! `[ecosystem]`, `[markers]`, and `commands` — field for field, so the
//! v0.4 ecosystem loader deserializes a file with only that shape
//! straight into this type with no translation layer. It does not yet
//! mirror every field Part 6 describes: `[[rules]]` and the future
//! inputs/templates/patches/sections shape are not part of this type,
//! which is exactly why the type is `#[non_exhaustive]` rather than
//! frozen in its current field set.
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
//! **Which fields a real file may omit — audited field by field against
//! `driftsys/repofolio/ecosystems/rust/folio.ecosystem.toml`, the one
//! real reference file that exists at the time of this audit:**
//!
//! - `schema` (`"$schema"`) — optional (`Option`, defaults to `None`).
//!   It is a pointer to the schema the file was authored against, not
//!   structural data; a file missing it is still a well-formed ecosystem
//!   definition.
//! - `ecosystem` (the `[ecosystem]` table) — **required, no default.**
//!   Every real file declares identity; there is no sensible default
//!   name or format version to fall back to, so a file missing this
//!   table should fail to deserialize rather than silently become a
//!   nameless ecosystem.
//!   - `EcosystemMeta.name` — required. No sensible default.
//!   - `EcosystemMeta.version` — required. No sensible default; the one
//!     real file sets it explicitly (`1`), and there is no evidence yet
//!     of a real file omitting it.
//! - `always` — not part of the on-disk format at all (`#[serde(skip)]`),
//!   so omission/presence in a file is moot.
//! - `markers` (the `[markers]` table) — **optional (`#[serde(default)]`,
//!   defaults to `Markers::default()`).** The real `rust` scaffold file
//!   omits `[markers]` entirely (its own comment: "Inputs, templates,
//!   patches, sections, and commands are intentionally omitted from the
//!   v0.1 scaffold"). Without this default, that file fails to
//!   deserialize with "missing field `markers`" — the defect a review
//!   round of this batch found and this comment records.
//!   - `Markers.must`, `Markers.should` — each optional
//!     (`#[serde(default)]`, empty `Vec` when absent).
//! - `commands` (the `commands` table) — **optional (`#[serde(default)]`,
//!   defaults to the empty `Commands::Group`).** Omitted by the same
//!   real scaffold file, for the same reason as `markers`.
//!
//! M1 populates two instances by hand — `repofolio` (always active) and
//! `rust` (activated by a root `Cargo.toml` declaring `[workspace]`).
//! Step 6 shaped both with empty `markers` and `commands`; step 7
//! (docs/archive/plans/2026-09-06-m1-check-plan.md) fills in `markers` with the
//! required and recommended path lists that back `FOLIO-101`/`FOLIO-102`.
//! `commands` stays empty — no milestone through M1 wires ecosystem
//! commands.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// One ecosystem: the core `repofolio` layer, or an ecosystem such as
/// `rust` that only applies when detected in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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
    /// The real `ecosystems/rust/folio.ecosystem.toml` scaffold in
    /// `driftsys/repofolio` omits `[markers]` entirely (its own comment
    /// says templates/sections/commands "are intentionally omitted from
    /// the v0.1 scaffold") — without `default` here, that file fails to
    /// deserialize with "missing field `markers`". See the field-by-field
    /// audit in the module doc comment.
    #[serde(default)]
    pub markers: Markers,
    /// Empty for both M1 instances, and omitted by the real
    /// `rust` scaffold file for the same reason `markers` is. Present in
    /// the type because v0.4 ecosystems (and the future `folio dispatch`
    /// verbs) need it, and adding a field to a type already frozen at
    /// v1.0 would be a breaking change.
    #[serde(default)]
    pub commands: Commands,
}

/// The `[ecosystem]` table: identity and file-format version, nested to
/// match the on-disk envelope exactly (see the module doc comment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EcosystemMeta {
    pub name: String,
    /// The `folio.ecosystem.toml` format version, e.g. `1` — not a
    /// semantic version of the ecosystem's own tooling.
    pub version: u32,
}

/// Paths that give both ecosystem detection and the `FOLIO-101`
/// (required) / `FOLIO-102` (recommended) conformance rules (Part 6 of
/// the task-model design). `rust`'s own activation rule is the dedicated
/// `[workspace]` content check below, not marker presence — these lists
/// exist for the conformance rules alone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Markers {
    #[serde(default)]
    pub must: Vec<MarkerSpec>,
    #[serde(default)]
    pub should: Vec<MarkerSpec>,
}

/// One entry in a `Markers` list: either a single required path, or a
/// group of acceptable spellings satisfied by any one of them (Part 6 of
/// the task-model design shows the plain-array form; the nested-array
/// alternative form is this type's addition, recorded there too).
///
/// `#[serde(untagged)]` reads a bare TOML string as `One` and a nested
/// array of strings as `AnyOf`, so an ecosystem author writes
///
/// ```toml
/// [markers]
/// should = ["rust-toolchain.toml", ["rustfmt.toml", ".rustfmt.toml"]]
/// ```
///
/// and gets exactly two recommendations, the second satisfied by either
/// spelling — rather than a third, unauthored, entry produced by
/// splitting a string like `"rustfmt.toml or .rustfmt.toml"` on a literal
/// separator. That convention was this type's predecessor: it read fine
/// in a Rust doc comment but was invisible to a third-party ecosystem
/// author writing a plain `folio.ecosystem.toml`, who would naturally
/// write `should = ["rustfmt.toml", ".rustfmt.toml"]` as two independent
/// entries and get exactly the double-warning the convention existed to
/// prevent. Expressing the alternation in the data instead of in a
/// separator closes that gap: the shape itself says "any one of these",
/// with no separate prose convention to miss.
///
/// The rule that reads this list (`repofolio_core::rules::path_rules`,
/// step 7 of docs/archive/plans/2026-09-06-m1-check-plan.md) treats an `AnyOf`
/// group as satisfied when any one alternative is present, and reports a
/// missing group as a single finding naming every alternative — never one
/// finding per alternative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum MarkerSpec {
    One(String),
    AnyOf(Vec<String>),
}

impl std::fmt::Display for MarkerSpec {
    /// Renders a single path as itself, and an alternative group joined
    /// by `" or "` — the finding text a missing marker reports, e.g.
    /// `"rustfmt.toml or .rustfmt.toml"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarkerSpec::One(path) => write!(f, "{path}"),
            MarkerSpec::AnyOf(paths) => write!(f, "{}", paths.join(" or ")),
        }
    }
}

impl MarkerSpec {
    /// The single path a caller should use when it needs one filesystem
    /// path rather than this spec's full human-readable form — the first
    /// (and only) path for `One`, or the first alternative for `AnyOf`.
    ///
    /// `repofolio_core::rules::path_diagnostic` uses this for
    /// `Location::file`, which maps to SARIF `artifactLocation.uri`: the
    /// full `Display` form of an `AnyOf` group (e.g.
    /// `"rustfmt.toml or .rustfmt.toml"`) is not a URI and cannot be
    /// opened, so it stays in the diagnostic's `message` instead, where
    /// it already appears.
    pub(crate) fn primary_path(&self) -> &str {
        match self {
            MarkerSpec::One(path) => path,
            MarkerSpec::AnyOf(paths) => paths.first().map(String::as_str).unwrap_or_default(),
        }
    }
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
#[non_exhaustive]
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
///
/// `markers` carries the core-layer conformance list
/// (docs/archive/plans/2026-09-06-m1-check-plan.md step 7). Manifest presence is
/// deliberately absent from `must`: that fact belongs to `FOLIO-001`
/// alone, so a repository with no manifest does not also report it as a
/// missing `FOLIO-101` path.
pub fn repofolio_ecosystem() -> Ecosystem {
    Ecosystem {
        schema: None,
        ecosystem: EcosystemMeta {
            name: "repofolio".to_string(),
            version: 1,
        },
        always: true,
        markers: Markers {
            must: vec![
                MarkerSpec::One("README.md".to_string()),
                MarkerSpec::One("LICENSE".to_string()),
                MarkerSpec::One("bootstrap".to_string()),
                MarkerSpec::One("runw".to_string()),
                MarkerSpec::One(".gitignore".to_string()),
                MarkerSpec::One(".gitattributes".to_string()),
                MarkerSpec::One(".editorconfig".to_string()),
                MarkerSpec::One("docs/".to_string()),
                MarkerSpec::One("scripts/".to_string()),
            ],
            should: vec![
                MarkerSpec::One("Foliofile".to_string()),
                MarkerSpec::One("CHANGELOG.md".to_string()),
                MarkerSpec::One("CODEOWNERS".to_string()),
                MarkerSpec::One("CONTRIBUTING.md".to_string()),
                MarkerSpec::One(".githooks/".to_string()),
            ],
        },
        commands: Commands::default(),
    }
}

/// The `rust` ecosystem: not always active, detected by
/// [`detect_rust`]. Not loaded from any `folio.ecosystem.toml` file, so
/// `schema` is `None`.
///
/// `markers` carries the `rust`-layer conformance list
/// (docs/archive/plans/2026-09-06-m1-check-plan.md step 7). The recommended
/// `rustfmt.toml`/`.rustfmt.toml` spellings are one `MarkerSpec::AnyOf`
/// entry, not two independent entries — see the `MarkerSpec` doc comment.
pub fn rust_ecosystem() -> Ecosystem {
    Ecosystem {
        schema: None,
        ecosystem: EcosystemMeta {
            name: "rust".to_string(),
            version: 1,
        },
        always: false,
        markers: Markers {
            must: vec![
                MarkerSpec::One("Cargo.toml".to_string()),
                MarkerSpec::One("Cargo.lock".to_string()),
            ],
            should: vec![
                MarkerSpec::One("rust-toolchain.toml".to_string()),
                MarkerSpec::AnyOf(vec![
                    "rustfmt.toml".to_string(),
                    ".rustfmt.toml".to_string(),
                ]),
            ],
        },
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
/// concern of `FOLIO-001`/`FOLIO-003` against the *project* manifest,
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

    /// Set to any non-empty value to require the sibling
    /// `driftsys/repofolio` checkout: with it set, a missing checkout
    /// fails `deserializes_the_real_rust_scaffold_file_from_the_sibling_checkout`
    /// rather than skipping it, so a CI job that is supposed to have the
    /// sibling checked out catches a misconfiguration instead of that
    /// test quietly asserting nothing. Unset (the default), a missing
    /// checkout still skips, so local development without the sibling
    /// checkout works.
    const FOLIO_STRICT_PARITY: &str = "FOLIO_STRICT_PARITY";

    /// Pure decision logic for the constant above, factored out of
    /// [`strict_parity_required`] so [`strict_parity_toggles_skip_vs_fail`]
    /// can exercise both branches directly without mutating the process
    /// environment (which would race
    /// `deserializes_the_real_rust_scaffold_file_from_the_sibling_checkout`'s
    /// own read of the same variable under parallel test execution).
    fn strict_parity_from(value: Option<&str>) -> bool {
        value.is_some_and(|v| !v.is_empty())
    }

    fn strict_parity_required() -> bool {
        strict_parity_from(std::env::var(FOLIO_STRICT_PARITY).ok().as_deref())
    }

    /// Exercises both branches of the gating decision
    /// `deserializes_the_real_rust_scaffold_file_from_the_sibling_checkout`
    /// uses when the sibling checkout is absent: unset (or empty) skips,
    /// any non-empty value fails. Testing the pure predicate this way
    /// pins both branches without requiring the sibling checkout to
    /// actually be removed from this machine to observe the fail branch.
    #[test]
    fn strict_parity_toggles_skip_vs_fail() {
        assert!(!strict_parity_from(None));
        assert!(!strict_parity_from(Some("")));
        assert!(strict_parity_from(Some("1")));
        assert!(strict_parity_from(Some("0")));
        assert!(strict_parity_from(Some("false")));
    }

    #[test]
    fn repofolio_is_always_active_with_no_commands() {
        let ecosystem = repofolio_ecosystem();

        assert_eq!(ecosystem.ecosystem.name, "repofolio");
        assert!(ecosystem.always);
        assert_eq!(ecosystem.commands, Commands::default());
    }

    /// Pins the exact required/recommended path lists
    /// (docs/archive/plans/2026-09-06-m1-check-plan.md step 7). Manifest presence
    /// is deliberately absent from `must` — that belongs to `FOLIO-001`
    /// alone.
    #[test]
    fn repofolio_markers_match_the_step_7_conformance_list() {
        let ecosystem = repofolio_ecosystem();

        assert_eq!(
            ecosystem.markers.must,
            vec![
                MarkerSpec::One("README.md".to_string()),
                MarkerSpec::One("LICENSE".to_string()),
                MarkerSpec::One("bootstrap".to_string()),
                MarkerSpec::One("runw".to_string()),
                MarkerSpec::One(".gitignore".to_string()),
                MarkerSpec::One(".gitattributes".to_string()),
                MarkerSpec::One(".editorconfig".to_string()),
                MarkerSpec::One("docs/".to_string()),
                MarkerSpec::One("scripts/".to_string()),
            ]
        );
        assert_eq!(
            ecosystem.markers.should,
            vec![
                MarkerSpec::One("Foliofile".to_string()),
                MarkerSpec::One("CHANGELOG.md".to_string()),
                MarkerSpec::One("CODEOWNERS".to_string()),
                MarkerSpec::One("CONTRIBUTING.md".to_string()),
                MarkerSpec::One(".githooks/".to_string()),
            ]
        );
    }

    #[test]
    fn rust_is_not_always_active() {
        let ecosystem = rust_ecosystem();

        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert!(!ecosystem.always);
    }

    /// Pins the `rust`-layer required/recommended path list. The
    /// recommended `rustfmt.toml`/`.rustfmt.toml` spellings are one
    /// `MarkerSpec::AnyOf` entry, not two independent entries.
    #[test]
    fn rust_markers_match_the_step_7_conformance_list() {
        let ecosystem = rust_ecosystem();

        assert_eq!(
            ecosystem.markers.must,
            vec![
                MarkerSpec::One("Cargo.toml".to_string()),
                MarkerSpec::One("Cargo.lock".to_string()),
            ]
        );
        assert_eq!(
            ecosystem.markers.should,
            vec![
                MarkerSpec::One("rust-toolchain.toml".to_string()),
                MarkerSpec::AnyOf(vec![
                    "rustfmt.toml".to_string(),
                    ".rustfmt.toml".to_string()
                ]),
            ]
        );
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

    /// Deserializes a *fully populated* ecosystem file, illustrated by
    /// Part 6 of the task-model design (`"$schema"`, `[ecosystem]` with
    /// `name`/`version`, `[markers]`, `[commands.<verb>]`) — a shape a
    /// real ecosystem file may legitimately have once it grows past a
    /// bare scaffold. This is a fabricated literal loosely based on that
    /// illustration, not a copy of any real file — see
    /// `deserializes_the_real_rust_scaffold_file_verbatim` and
    /// `deserializes_the_real_rust_scaffold_file_from_the_sibling_checkout`
    /// below for tests against the one real file that exists today,
    /// which has neither `[markers]` nor `[commands]`. A prior version of
    /// this test claimed to prove the mirror against "the real on-disk
    /// shape" while never parsing anything a real file actually
    /// contains, which is why it kept passing while the real file failed
    /// to deserialize (missing field `markers`) — recorded here so the
    /// mistake is not repeated.
    #[test]
    fn deserializes_a_populated_ecosystem_file_per_the_design_doc_illustration() {
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

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml).expect("populated shape parses");

        assert_eq!(
            ecosystem.schema.as_deref(),
            Some("https://driftsys.github.io/schemas/folio-ecosystem/v1.json")
        );
        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert_eq!(ecosystem.ecosystem.version, 1);
        assert_eq!(
            ecosystem.markers.must,
            vec![MarkerSpec::One("Cargo.toml".to_string())]
        );
        assert_eq!(
            ecosystem.markers.should,
            vec![MarkerSpec::One("rust-toolchain.toml".to_string())]
        );
        // Not part of the file format: always is never set by a loaded file.
        assert!(!ecosystem.always);
    }

    /// The nested-array alternative form: a `[markers]` list may mix bare
    /// strings (`MarkerSpec::One`) with a nested array of strings
    /// (`MarkerSpec::AnyOf`), matching Part 6 of the task-model design's
    /// description of the on-disk format after this batch's fix.
    #[test]
    fn markers_should_parses_a_nested_array_as_an_any_of_group() {
        let toml = r#"
[ecosystem]
name = "rust"
version = 1

[markers]
should = ["rust-toolchain.toml", ["rustfmt.toml", ".rustfmt.toml"]]
"#;

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml).expect("nested array parses");

        assert_eq!(
            ecosystem.markers.should,
            vec![
                MarkerSpec::One("rust-toolchain.toml".to_string()),
                MarkerSpec::AnyOf(vec![
                    "rustfmt.toml".to_string(),
                    ".rustfmt.toml".to_string()
                ]),
            ]
        );
    }

    /// (a) A byte-for-byte copy of the real reference file —
    /// `driftsys/repofolio/ecosystems/rust/folio.ecosystem.toml` at the
    /// time of writing — including its leading comment line, the
    /// `"$schema"` key, and the absence of `[markers]`/`[commands]` its
    /// own trailing comment documents as intentional for the v0.1
    /// scaffold. This is the literal that exposed the "missing field
    /// `markers`" defect: `Ecosystem.markers` had no `#[serde(default)]`,
    /// so this exact text failed to deserialize until that was fixed.
    #[test]
    fn deserializes_the_real_rust_scaffold_file_verbatim() {
        let toml = r#"# ecosystems/rust/folio.ecosystem.toml
"$schema" = "https://driftsys.github.io/schemas/folio-ecosystem/v1.json"

[ecosystem]
name = "rust"
version = 1

# Inputs, templates, patches, sections, and commands are intentionally
# omitted from the v0.1 scaffold. They are filled in as folio v0.1+ work.
"#;

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml)
            .expect("the real rust scaffold file, byte for byte, must deserialize");

        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert_eq!(ecosystem.ecosystem.version, 1);
        assert_eq!(
            ecosystem.schema.as_deref(),
            Some("https://driftsys.github.io/schemas/folio-ecosystem/v1.json")
        );
        assert_eq!(ecosystem.markers, Markers::default());
        assert_eq!(ecosystem.commands, Commands::default());
    }

    /// (b) Reads the actual sibling file at
    /// `../../../repofolio/ecosystems/rust/folio.ecosystem.toml` (from
    /// `CARGO_MANIFEST_DIR`), the same present-or-skip pattern
    /// `repofolio-manifest`'s `tests/schema_parity.rs` already uses for
    /// its sibling `driftsys/schemas` checkout. Unlike the verbatim copy
    /// above, this test keeps catching drift if the real file's shape
    /// changes upstream, since it reads the file itself rather than a
    /// literal frozen at the time this test was written. Skips (rather
    /// than failing) when the sibling `driftsys/repofolio` checkout is
    /// not present, e.g. a CI clone of only this repository — unless
    /// `FOLIO_STRICT_PARITY` is set, in which case a missing checkout
    /// fails this test instead (see the constant's doc comment above).
    #[test]
    fn deserializes_the_real_rust_scaffold_file_from_the_sibling_checkout() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../repofolio/ecosystems/rust/folio.ecosystem.toml");

        if !path.is_file() {
            assert!(
                !strict_parity_required(),
                "{FOLIO_STRICT_PARITY} is set and the sibling driftsys/repofolio checkout \
                 was not found at {}",
                path.display()
            );
            eprintln!(
                "skipping: sibling driftsys/repofolio checkout not found at {}",
                path.display()
            );
            return;
        }

        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        let ecosystem: Ecosystem = toml_edit::de::from_str(&text)
            .unwrap_or_else(|e| panic!("deserialize {}: {e}", path.display()));

        assert_eq!(ecosystem.ecosystem.name, "rust");
        assert_eq!(ecosystem.ecosystem.version, 1);
    }

    /// A malicious or careless `folio.ecosystem.toml` cannot grant
    /// itself unconditional activation: `always` is `#[serde(skip)]`,
    /// so an `always = true` key in the input is not read as this
    /// field at all — it is simply an unrecognized key the (permissive,
    /// non-`deny_unknown_fields`) deserializer ignores.
    #[test]
    fn always_cannot_be_set_from_a_loaded_file() {
        let toml = r#"
always = true

[ecosystem]
name = "evil"
version = 1

[markers]
"#;

        let ecosystem: Ecosystem = toml_edit::de::from_str(toml).expect("file parses");

        assert!(!ecosystem.always);
    }
}
