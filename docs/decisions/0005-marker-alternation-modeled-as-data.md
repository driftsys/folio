# AD-0005 — marker alternation is modeled as data (`MarkerSpec`), not a string convention

## Context

`FOLIO-102` must let an ecosystem recommend a path where more than one
spelling is acceptable — for example `rustfmt.toml` or `.rustfmt.toml` —
without reporting the two spellings as independent, double-counted
warnings when neither is present. An earlier implementation modeled this as
a single string joined by the literal separator `" or "` (e.g.
`"rustfmt.toml or .rustfmt.toml"`), documented only in a Rust doc comment.

## Options considered

- **Keep the string-join convention.** Compact, but invisible to a
  third-party `folio.ecosystem.toml` author, who never reads folio's Rust
  source and would naturally write the two spellings as two separate
  marker entries — getting exactly the double warning the convention
  existed to prevent. It also mis-splits any real path that happens to
  contain the literal substring `" or "`.
- **Model the alternation in the data itself.**

## Decision

`MarkerSpec` (`crates/repofolio-core/src/ecosystem.rs`) is an untagged
enum: `One(String) | AnyOf(Vec<String>)`. A bare TOML string in a
`[markers]` list is one required path; a nested array of strings is a group
satisfied by any one member:

```toml
[markers]
should = ["rust-toolchain.toml", ["rustfmt.toml", ".rustfmt.toml"]]
```

`rules::path_rules` treats an `AnyOf` group as satisfied when any one
alternative exists on disk, and reports a missing group as exactly one
finding naming every alternative (via `MarkerSpec`'s `Display` impl), not
one finding per alternative.

## Consequences

- Part 6 of the task-model design document (now archived at
  `docs/archive/specs/2026-09-06-folio-task-model-design.md`) was updated in
  the same change so the normative description and the frozen type agree.
- `Location::file` uses `MarkerSpec::primary_path()` (the first
  alternative) rather than the full `Display` form, since the joined string
  is not a URI and cannot be opened as a file location; the full form stays
  in the finding's `message`, where it already appears.
- The deserializer is deliberately permissive: an empty alternatives group
  (`should = [[]]`) is accepted and evaluates as permanently unsatisfied.
  Rejecting it is left to a future `schemas/folio-ecosystem/v1.json`
  `minItems` constraint, which does not yet exist — consistent with
  ecosystem-file shape enforcement living in that schema rather than in
  this Rust type. See `docs/technotes/m1-deferred-findings.md`.
- `MarkerSpec` must be re-exported from `repofolio-core`'s public API
  (`lib.rs`) — a publicly exported `Markers` with a field of an
  unexported type fails to compile for any external caller
  (`repofolio_core::MarkerSpec` → `E0432`), and this is not caught by
  rustc's `private_interfaces`, `private_bounds`, or `unreachable_pub`
  lints. An integration test under `tests/` (which compiles as an external
  crate) is the only guard against this class of regression recurring.

## Satisfies

`crates/repofolio-core/src/ecosystem.rs` (`MarkerSpec`);
`crates/repofolio-core/src/rules.rs` (`path_rules`, `marker_spec_exists`,
`path_diagnostic`); `crates/repofolio-core/src/lib.rs` (public re-export);
`crates/repofolio-core/tests/public_api.rs`.
