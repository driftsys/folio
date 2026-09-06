# `folio registry` — specification

`folio registry [--format json]` dumps the `FOLIO-` diagnostic code
registry and the ecosystem registry as one JSON document. It satisfies
CLAUDE.md's non-negotiable architecture decision 6 (the tool contract):
every orchestrated tool, folio included, must expose a `<tool> registry
--format json` dump from v0.1 onward, alongside stable diagnostic codes
and `--format json` on check verbs — both of which `folio check` already
provides (see `docs/specification/folio-check.md`).

Unlike `check`, a registry dump describes what exists rather than what
was found in a specific repository, so it always exits `0` — there is no
finding to grade an exit status from. `--format` accepts only `json`: a
registry is consumed by agents and MCP (per the drift rule this decision
states — registries hold facts, MCP serves facts, skills hold procedure),
not read as prose, so no human-readable rendering exists.

## Report shape

```json
{
  "codes": [
    { "code": "FOLIO-xxx", "severity": "error" | "warning" | "info", "description": "<human-readable condition>" }
  ],
  "ecosystems": [
    { "ecosystem": { "name": "<name>", "version": <n> }, "markers": { "must": [...], "should": [...] }, "commands": {} }
  ]
}
```

`codes` is the same five-entry registry documented in
`docs/specification/folio-check.md`'s "The `FOLIO-` diagnostic registry"
section, in the same numeric order. Each entry's `severity` is the code's
*nominal* severity — the severity it reports at when it actually fires —
not a variable field: `FOLIO-002`'s info-severity "skipped" case is a
message-level nuance of one particular finding (see
`docs/decisions/0004-schema-validation-skip-reported-as-info-severity.md`),
not a fact about the code itself, so it is listed here as `error`.

`ecosystems` is exactly the two `Ecosystem` values `pipeline::check` uses
(`repofolio_ecosystem()`, `rust_ecosystem()`), serialized in the same
shape a `folio.ecosystem.toml` file would round-trip through (see
`docs/design/folio-check.md`'s "Ecosystem registry" section and
`docs/decisions/0006-ecosystem-envelope-mirrors-on-disk-format.md`). The
registry dump and the check pipeline share one source of data — there is
no second list of ecosystems that could drift from the first.

## Exit status

`0` always, unless JSON serialization itself fails (an internal error,
not a usage failure) or an unrecognized `--format` value is given (a
usage failure clap rejects before the command runs) — both take the same
`Result`-based `Err` path as `check`, exiting `2`.
