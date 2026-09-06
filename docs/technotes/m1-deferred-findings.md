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
   ignored — a genuine conformance smell with no code in the four-entry
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
