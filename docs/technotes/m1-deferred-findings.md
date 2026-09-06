# M1 deferred findings

Minor findings surfaced during the M1 (`folio check`) implementation and
deliberately deferred rather than fixed in the same milestone. This
repository has no issue tracker wired up, so they are recorded here as a
tracked list instead of being silently dropped. None of these are blockers
for M1; each names the file it applies to so a future milestone can pick it
up.

1. **Ambiguous multi-manifest presence has no diagnostic code.**
   `discover_manifest` (`crates/repofolio-manifest/src/discover.rs`)
   selects `project.toml` over `project.yaml` over `project.json` by
   silent priority when more than one is present. A repository holding
   more than one project manifest gets no warning that one is being
   ignored — a genuine conformance smell with no code in the five-entry
   `FOLIO-` registry to express it. Consider a dedicated code the next
   time the registry is extended.
2. **`validate_manifest` recompiles the schema on every call**
   (`crates/repofolio-manifest/src/schema.rs`) instead of caching the
   compiled validator behind a `OnceLock`. Irrelevant at one manifest per
   `folio check` run; would matter if `repofolio-core` validates many
   manifests within one process.
3. **`yaml_key_to_string` has no direct test**
   (`crates/repofolio-manifest/src/parse.rs`) — the branch converting a
   non-string YAML mapping key to its JSON-object-key form is exercised
   only indirectly, if at all, by the existing test suite.
4. **The human-readable report omits the JSON report's `layer` field**
   (`crates/repofolio/src/main.rs`, `print_human`). Harmless at this
   milestone — `repofolio` and `rust` have no overlapping marker paths, so
   the file name alone disambiguates which layer produced a finding — but
   becomes a real ambiguity once a colliding-name ecosystem lands at v0.4.
5. **No acceptance test covers `folio check` with no path argument**
   (`crates/repofolio/tests/check.rs`), the documented default of using the
   current directory.
6. **`MarkerSpec::AnyOf` accepts an empty alternatives group**
   (`should = [[]]`, `crates/repofolio-core/src/ecosystem.rs`), which
   evaluates as permanently unsatisfied. Deliberately left permissive in
   the Rust type (see `docs/decisions/0005-marker-alternation-modeled-as-data.md`);
   rejecting it is deferred to a `minItems` constraint in
   `schemas/folio-ecosystem/v1.json`, which does not yet exist.
7. **`tests/public_api.rs` is narrower than its name suggests**
   (`crates/repofolio-core/tests/public_api.rs`). `#[non_exhaustive]` on an
   enum does not block variant construction from outside the crate, so the
   compiler forced only `Markers`'s fields into this regression test, not a
   `MarkerSpec` construction from outside the crate — the test still
   matches on `MarkerSpec`, so it is not vacuous, but it no longer
   demonstrates constructing one externally.
8. **A TOML date/datetime literal under the manifest's freeform
   `metadata` object does not parse to the same value as its YAML or JSON
   equivalent** (`crates/repofolio-manifest/src/parse.rs`,
   `parse_manifest`'s TOML branch). `toml_edit::de::from_str::<Value>`
   deserializes a date/datetime into a private wrapper object
   (`{"$__toml_private_datetime": "..."}`) rather than a plain string,
   because `serde_json::Value` has no native date type; the equivalent
   `project.yaml`/`project.json` manifest produces a plain string for the
   same logical value. This breaks the "same manifest parses identically
   across formats" invariant `same_logical_manifest_parses_identically_
   across_formats` pins, for date-typed values specifically — that test
   does not cover a date field today. `metadata` accepts any properties,
   so this is reachable by any manifest author writing a bare date under
   it. Fixing it needs a dedicated TOML-to-JSON conversion (mirroring
   `parse.rs`'s existing hand-rolled `yaml_to_json`) that converts a TOML
   datetime to a string explicitly, rather than deserializing generically
   into `serde_json::Value`.
9. **Two YAML mapping keys that stringify identically collide silently**
   (`crates/repofolio-manifest/src/parse.rs`, `yaml_key_to_string`). A
   non-string key (e.g. the integer `1`) is rendered through its JSON
   form (`"1"`); a manifest with both an integer key `1` and a string key
   `"1"` under `metadata` has one silently overwrite the other in the
   resulting `serde_json::Map`, with no parse error.
10. **`parse_yaml` silently drops every document after the first in a
    multi-document YAML stream** (`crates/repofolio-manifest/src/parse.rs`,
    `parse_yaml`'s `docs.remove(0)`). A `project.yaml` containing a stray
    `---` separator has everything after it discarded rather than
    rejected, so a malformed or accidentally-multi-document manifest
    parses "successfully" with silently missing content.
11. **`detect_rust`'s doc comment overstates its own check**
    (`crates/repofolio-core/src/ecosystem.rs`). It says activation
    requires a root `Cargo.toml` "declaring a `[workspace]` table", but
    the implementation is `doc.contains_key("workspace")`, true for a
    top-level `workspace` key of any TOML value type, not only a table.
    Not reachable through a real repository today — `workspace = <scalar>`
    is not valid Cargo syntax, so `cargo` itself would already reject such
    a `Cargo.toml` — but the doc comment and the check disagree, and no
    test covers "`workspace` key present with the wrong shape".
12. **`MarkerSpec` marker paths are joined onto `repo_root` with no check
    that they are relative** (`crates/repofolio-core/src/rules.rs`,
    `marker_spec_exists`). `Path::join` silently discards `repo_root` when
    given an absolute path, so an absolute marker path would be checked
    against the host filesystem root instead of the repository being
    scanned. Not reachable in M1 — both hardcoded ecosystems (`repofolio`,
    `rust`) only ever use relative marker paths — but `MarkerSpec` derives
    `Deserialize` specifically so the v0.4 ecosystem loader can feed it
    marker paths from a third-party `folio.ecosystem.toml`, which is
    exactly the untrusted-input case this becomes a real concern for.
13. **The `FOLIO_STRICT_PARITY` gating pattern is duplicated verbatim**
    between `crates/repofolio-core/src/ecosystem.rs` and
    `crates/repofolio-manifest/tests/schema_parity.rs` (constant, reader,
    predicate, and an identical `strict_parity_toggles_skip_vs_fail`
    test in each). If the gating rule ever changes in one copy, there is
    no compiler or test signal pointing at the other.
14. **`location_label` silently drops a set `column` when `line` is
    absent** (`crates/repofolio/src/main.rs`). `Location { file, line:
    None, column: Some(_) }` is a legal, constructible value of the
    public `Location` type, but the wildcard arm of `location_label`'s
    match prints only the bare file name in that shape, with no
    indication data was lost. No M1 rule produces this combination, so
    the branch is untested; a future content-match rule that supplies
    only a column would hit it silently.
15. **`marker_spec_exists`'s directory-marker matching
    (trailing-slash paths like `"docs/"`, `".githooks/"`) is unverified
    on Windows** (`crates/repofolio-core/src/rules.rs`). On Unix, a
    trailing separator forces `Path::exists()` to reject a same-named
    plain file, correctly requiring a real directory; whether Windows'
    path APIs enforce the same distinction is unconfirmed, and this repo
    has no Windows CI target. Not a demonstrated failure — folio does not
    yet ship a Windows build (release assets land at v0.5+) — but worth
    checking before it does.
16. **A file-shaped marker (no trailing slash, e.g. `"README.md"`) is
    satisfied by a same-named directory** (`crates/repofolio-core/src/
    rules.rs`, `marker_spec_exists`). `Path::exists()` is true for any
    filesystem entry at that path, file or directory; nothing distinguishes
    the two for a marker that isn't already using the directory
    convention. A repository with an empty directory literally named
    `README.md` would satisfy `FOLIO-101`'s required-path check with no
    readable content behind it — a narrow, unlikely-in-practice edge
    case, but a real one, and distinct from finding 15's directory-marker
    case.
17. **The ecosystem registry (`folio registry`) and the check pipeline
    (`folio check`) are two independently maintained lists of which
    ecosystems exist**, tied together only by doc-comment promises, not
    by structure. `registry::ecosystem_registry()`
    (`crates/repofolio-core/src/registry.rs`) and `pipeline::check()`'s
    hardcoded `repofolio_ecosystem()`/`detect_rust()`-gated calls
    (`crates/repofolio-core/src/pipeline.rs`) both currently list exactly
    `repofolio` and `rust`, but nothing enforces that a third ecosystem
    added to one is also added to the other. Related: `Ecosystem.always`
    is `#[serde(skip)]` (see decision 0006) specifically so no on-disk
    file can set it, but as a side effect it is also absent from
    `folio registry`'s JSON output entirely — a consumer of the registry
    dump cannot currently tell which listed ecosystems are unconditionally
    active versus marker-gated.
18. **`Markers` deserializes permissively — a misspelled field name
    silently produces an empty marker list rather than a deserialization
    error** (`crates/repofolio-core/src/ecosystem.rs`, `must`/`should`
    both `#[serde(default)]`, no `deny_unknown_fields`). Not reachable in
    M1 (no ecosystem-file loader exists yet), but `Markers`/`MarkerSpec`
    already derive `Deserialize` specifically for the v0.4 loader, and
    nothing in this milestone guards against a third-party ecosystem
    author's typo silently disabling every marker check for their
    ecosystem. Consider `deny_unknown_fields`, or validation in
    `schemas/folio-ecosystem/v1.json` (see finding 6, same open
    dependency).
19. **No automated test confirms `cargo package --list` still includes
    the fixture trees** (`crates/repofolio-core/tests/fixtures/`). The
    packaging fix (renaming fixture manifests to `Cargo.toml.fixture`,
    materialized back at test time by `materialize_fixture`) was verified
    manually once, by running `cargo package --list` in a scratch
    directory (see the execution ledger). A future fixture tree added
    with a literal checked-in `Cargo.toml` (forgetting the `.fixture`
    convention) would silently be dropped by `cargo package` again — the
    exact defect this fix addressed — with no test or CI signal.
20. **"Stays `#[non_exhaustive]`" is unpinned for `Report`, `Diagnostic`,
    `Location`, `BySeverity` (`crates/repofolio-core/src/report.rs`), and
    `CodeEntry`/`Registry` (`registry.rs`)**. `tests/public_api.rs` pins
    exactly this property for `Markers`/`MarkerSpec` (it relies on being
    unable to build a `Markers` literal from outside the crate), because
    that property was once actually lost by accident. No comparable
    regression has happened for these six types, and the standard way to
    pin "does not compile" is a `trybuild`-style compile-fail test, a new
    dev-dependency this milestone doesn't otherwise need — recorded
    rather than added speculatively; revisit if `public_api.rs`'s pattern
    is ever extended.

## Process note: three false-pass incidents

Verification-in-place — reading code, running the test suite as committed
— gave a false pass three separate times during M1 execution, each caught
only by a fresh clone or by mutation testing rather than by review:

1. A fabricated literal test for `Ecosystem` claimed to mirror "the real
   on-disk shape" while never parsing anything the one real reference file
   (`driftsys/repofolio/ecosystems/rust/folio.ecosystem.toml`) actually
   contains. It passed while the real file failed to deserialize ("missing
   field `markers`") — see
   `docs/decisions/0006-ecosystem-envelope-mirrors-on-disk-format.md`.
2. `crates/repofolio-core/tests/fixtures/compliant/Cargo.lock` sat
   untracked in the working tree: the root `.gitignore`'s unanchored
   `Cargo.lock` line matched it. The fixture suite passed locally, on the
   strength of a stray untracked file, and failed on a fresh clone.
3. `always_cannot_be_set_from_a_loaded_file`'s test TOML placed `always =
   true` under `[ecosystem]` rather than at the document root, so the
   assertion stayed inert: reverting the `#[serde(skip)]` fix back to
   `#[serde(default)]` still left every test green. A prior re-review had
   read the test and misdescribed it as using a top-level key; only
   mutation testing — deliberately reverting the fix and confirming the
   test then fails — caught it.

All three were fixed within the same milestone; none is an open defect.
The pattern is recorded here as institutional knowledge for future
milestones in this repository: a passing suite, read rather than executed
against a fresh clone or a deliberate mutation, is not proof.
