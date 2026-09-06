# AD-0003 — manifest parsing and validation libraries: `jsonschema`, `yaml-rust2`

## Context

`folio check` validates a project manifest that may be written in any of
three formats through one pipeline: parse to `serde_json::Value`, then
validate that value against a bundled JSON Schema (draft-07). Two library
choices for this pipeline were reconsidered on 2026-09-06 against evidence
checked on crates.io (release dates and download volumes), rather than
assumed from an earlier, less-verified pass.

## Options considered

**JSON Schema validation:** `boon`, the earlier choice, has fewer
transitive dependencies, but version 0.6.1 was last released 2025-01-07
with 174,000 recent downloads. `jsonschema` 0.54.0 was released 2026-09-05
with 20 million recent downloads.

**YAML parsing:** every fork of `serde_yaml` is stale or contested —
`serde_yaml` itself is author-deprecated (2024-03-25), `serde_yaml_ng` last
released 2024-05-26, `serde_norway` 2024-12-21, and `serde_yml` is a 0.0.x
fork with a contested history. `yaml-rust2` 0.12.0 was released
2026-08-18 and is actively maintained.

## Decision

**`jsonschema`, not `boon`.** A validator enforcing the manifest contract
should not be twenty months stale; dependency count does not outweigh
that.

**`yaml-rust2`, not any `serde_yaml` fork.** This is not a compromise:
folio does not need serde integration for YAML. The pipeline is `YAML ->
serde_json::Value -> jsonschema`; no type ever derives `Deserialize` from
YAML, so parsing to an abstract syntax tree and converting it to
`serde_json::Value` by hand (`crates/repofolio-manifest/src/parse.rs`,
`yaml_to_json`) is sufficient, and the stale/contested `serde_yaml`-fork
lineage is avoided entirely rather than merely tolerated.

`toml_edit` (already a workspace dependency) gained its `serde` feature for
the TOML path; it is format-preserving, which is why it was already chosen
over a plain TOML parser — `folio init` at a later milestone needs to write
into an existing manifest without destroying comments or key ordering, a
requirement this pipeline does not yet exercise but does not want to
foreclose.

## Consequences

- A YAML value `yaml-rust2` itself cannot resolve — an unresolved anchor
  alias (`Yaml::BadValue`), an unsupported alias (`Yaml::Alias`), or a
  `Real` scalar that does not parse to a finite `f64` — must surface as a
  parse error (`ParseError::Yaml`), not silently become `Value::Null`. The
  manifest's `metadata` object accepts any properties
  (`additionalProperties: true`), so a silently nulled value there would
  otherwise pass schema validation instead of being caught as the parse
  failure it is.
- Fixing the fix propagates `ParseError::Yaml` in cases some other YAML
  implementation might accept; this was judged an acceptable, narrow
  widening of what folio rejects, in exchange for closing a real schema-
  validation bypass.

## Satisfies

`Cargo.toml` (`[workspace.dependencies]`: `jsonschema`, `yaml-rust2`,
`toml_edit`); `crates/repofolio-manifest/src/schema.rs`;
`crates/repofolio-manifest/src/parse.rs`; `docs/specification/folio-check.md`
(manifest parsing, schema validation).
