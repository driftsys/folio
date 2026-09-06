# AD-0004 — a skipped `FOLIO-002` is reported at `Info` severity, not a fourth `Severity` variant

## Context

`FOLIO-002` (manifest fails the bundled schema) has nothing to validate
once the manifest failed to be discovered or parsed (`FOLIO-001` for a
missing manifest, `FOLIO-003` for one that fails to parse) — there is no
parsed value left to check. The `Report`/`Diagnostic`/
`Severity` shape (`crates/repofolio-core/src/report.rs`) mirrors the house
diagnostics shape at `schemas/markspec/diagnostics/v1.json`, whose
`severity` enum is only `error`/`warning`/`info`, and is frozen at v1.0 so
that SARIF can later be added as a serializer rather than as a breaking
change to the wire contract.

## Options considered

- **Add a fourth `Severity::Skipped` variant.** Rejected: breaks the mirror
  to the house diagnostics shape, and every consumer of `by_severity` would
  need a fourth bucket it does not otherwise need.
- **Omit the `FOLIO-002` finding entirely when skipped.** Rejected: loses
  the signal that this code conceptually ran and had nothing to check,
  versus never having run at all — a consumer cannot distinguish "clean"
  from "skipped" from the report alone.
- **Emit `FOLIO-002` at `Info` severity**, with a message stating it was
  skipped and why.

## Decision

Emit the skipped `FOLIO-002` finding at `Severity::Info`
(`crates/repofolio-core/src/rules.rs`, `skipped_schema_diagnostic`), with
the message `"skipped: {reason} already failed, nothing to validate"`, where
`{reason}` is `FOLIO-001` or `FOLIO-003` depending on which stage failed.
This stays expressible in the mirrored three-variant shape, does not affect
exit-code grading (only an error-severity finding moves
`report.by_severity.error` above zero), and maps cleanly to a future SARIF
`level: note`, `kind: notApplicable`.

## Consequences

A consumer that counts every `info`-severity finding as a real diagnostic,
rather than distinguishing a skip marker from one, will misread a skip as a
finding. This is the accepted cost of not adding a fourth severity variant
to a type frozen at v1.0.

## Satisfies

`crates/repofolio-core/src/report.rs` (`Severity`);
`crates/repofolio-core/src/rules.rs` (`skipped_schema_diagnostic`,
`manifest_rules`); `crates/repofolio-core/src/registry.rs`
(`code_registry`'s `FOLIO-002` entry lists `Severity::Error` as the
code's nominal severity, per this decision, not the variable skip
outcome); `docs/specification/folio-check.md` (`FOLIO-002` row);
`docs/specification/folio-registry.md` (`FOLIO-002` entry).
