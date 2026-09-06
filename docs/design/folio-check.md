# `folio check` — design

Architecture and detailed design for the `check` verb, spanning three
crates: `repofolio-manifest`, `repofolio-core`, and the `repofolio` binary.
See `docs/specification/folio-check.md` for the requirements this design
satisfies, and `docs/decisions/` for the individual choices called out below.

## Crate boundaries

- **`repofolio-manifest`** — manifest discovery (`discover_manifest`),
  parsing to a canonical `serde_json::Value` regardless of source format
  (`parse_manifest`), and schema validation against the bundled schema
  (`validate_manifest`). Discovery failure (`DiscoverError`), parse failure
  (`ParseError`), and schema failure (`SchemaError`) are three distinct
  types, because they map to three different diagnostic codes one layer up:
  `FOLIO-001` (missing), `FOLIO-003` (unparseable), and `FOLIO-002` (schema)
  respectively.
- **`repofolio-core`** — the `FOLIO-` rule registry (`rules::manifest_rules`,
  `rules::path_rules`), the ecosystem registry (`ecosystem` module), the
  diagnostic report type (`report` module), and the pipeline that runs all
  of it in order (`pipeline::check`). `repofolio-manifest` is an
  implementation detail of `manifest_rules`; nothing outside
  `repofolio-core` depends on it directly.
- **`repofolio`** (bin `folio`) — clap CLI front end. Owns the exit-status
  contract and the two output renderers (JSON, human-readable). No business
  logic: `main.rs` calls `repofolio_core::check` and formats what comes
  back.

## The check pipeline

`crates/repofolio-core/src/pipeline.rs`, function `check`:

```
discover -> parse -> validate -> core rules -> ecosystem detection and rules -> report
```

No stage aborts the run. A repository with a missing or unparseable
manifest still runs every later stage: the path-presence rules
(`FOLIO-101`/`FOLIO-102`) do not depend on the manifest having parsed or
validated, and a report describing every independent problem is more useful
than one that stops at the first.

```
manifest_rules(root)                     // FOLIO-001, FOLIO-003, FOLIO-002
  + path_rules(root, repofolio_ecosystem())   // FOLIO-101/102, layer "repofolio", always
  + (detect_rust(root) ? path_rules(root, rust_ecosystem()) : [])  // layer "rust"
  -> Report::new(all diagnostics)
```

Ecosystem detection is a flat `if` today because exactly one optional layer
(`rust`) exists at this milestone; a future milestone that adds more
ecosystems replaces this with a loop over a registry, not a change to the
pipeline's stage order.

## Diagnostic report type (`repofolio-core::report`)

`Report { count, by_severity, diagnostics: Vec<Diagnostic> }`, camelCase on
the wire (`bySeverity`). Frozen at v1.0: every rule builds on this shape
without reshaping it, so SARIF can be added later as a serializer rather
than a breaking contract change.

- `Report::new(diagnostics)` is the only constructor. It derives `count` and
  `by_severity` from the list, so the two can never disagree with what the
  list actually contains — there is no path that builds a `Report` with
  hand-computed counts.
- `Diagnostic { severity, code, message, layer: Option<String>, location }`.
  `code` stays a plain `String`, not a closed enum: ecosystem-specific codes
  (`RUST-001`, `UNITY-001`, per the task-model design's Part 6) flow through
  the same type as the core `FOLIO-xxx` codes, and a plugin ecosystem could
  never mint a fixed Rust variant. `layer` is `None` for manifest-level
  codes (`FOLIO-001`/`FOLIO-003`/`FOLIO-002`, which apply once per
  repository) and `Some(name)` for per-layer codes (`FOLIO-101`/`FOLIO-102`).
- `Severity` is exactly `Error | Warning | Info` — see
  `docs/decisions/0004-schema-validation-skip-reported-as-info-severity.md`
  for why a skipped check does not get a fourth variant.
- `Location { file, line: Option<u32>, column: Option<u32> }`. Every finding
  at this milestone is path-only (`Location::file`); `line`/`column` exist
  for a future content-match rule kind that can point at a specific
  position, so that rule kind needs no shape change when it arrives.

This mirrors the house diagnostics shape at
`schemas/markspec/diagnostics/v1.json` (`count`, `bySeverity`,
`diagnostics[]` with `severity`/`code`/`message`/`location.file`), with two
deliberate departures: `location.line`/`location.column` are optional here
(markspec requires `line`), and `Diagnostic` carries the additive `layer`
field with no markspec equivalent — an additive field a future SARIF
serializer can carry in its `properties` bag, and one markspec's own
`additionalProperties: false` schema will not accept, which folio accepts by
having its own schema rather than validating against markspec's.

## Ecosystem registry (`repofolio-core::ecosystem`)

`Ecosystem { schema: Option<String>, ecosystem: EcosystemMeta { name,
version }, always: bool, markers: Markers { must, should }, commands:
Commands }`.

This type's field shape is not free-standing: it mirrors the on-disk
`folio.ecosystem.toml` envelope (`"$schema"`, `[ecosystem]`, `[markers]`,
`commands`) field for field, so that a future v0.4 ecosystem loader
deserializes a real file straight into this type through the same code
path, rather than becoming a second, parallel implementation that
translates between a hand-rolled M1 shape and the real file format. See
`docs/decisions/0006-ecosystem-envelope-mirrors-on-disk-format.md` for the
full rationale, including why `always` cannot be set from a loaded file.

`Markers.must`/`Markers.should` are `Vec<MarkerSpec>`, where `MarkerSpec` is
`One(String) | AnyOf(Vec<String>)` — see
`docs/decisions/0005-marker-alternation-modeled-as-data.md`. The rule that
reads this list (`rules::path_rules`) treats an `AnyOf` group as satisfied
when any one alternative exists on disk, and reports a missing group as a
single finding naming every alternative via `MarkerSpec`'s `Display` impl;
`Location::file` uses `MarkerSpec::primary_path()` (the first alternative)
instead, since the full `Display` form is not a URI and cannot be opened.

`Commands` is a recursive tree, `Command(String) | Group(BTreeMap<String,
Commands>)`, bottoming out at whatever depth an ecosystem needs — two levels
for `rust`/`deno` (`commands.build.debug`), three for `c`
(`commands.cmake.build.debug`). The type does not know which level means
"verb" versus "build system"; that interpretation belongs to whichever
future milestone walks the tree to run a command. Both M1 instances
(`repofolio_ecosystem()`, `rust_ecosystem()`) populate `commands` as the
empty `Group`, since no milestone through M1 wires ecosystem commands.
Malformed shape (e.g. `commands.build.debug` written instead as
`commands.debug.build`) deserializes without error today and fails only at
lookup — schema enforcement for this tree is deferred to
`schemas/folio-ecosystem/v1.json`, which does not yet exist.

`detect_rust(root)` is a dedicated content check against `Cargo.toml`
(`doc.contains_key("workspace")`), not a marker-presence test, because a
single, non-workspace crate also has a `Cargo.toml` and must not activate
the `rust` layer.

## Rule registry (`repofolio-core::rules`)

`manifest_rules(root)` runs the discover → parse → validate chain and turns
every stage's failure into a diagnostic rather than an early return, so the
pipeline can always continue past it. It is intentionally the only place
that decides `FOLIO-001` versus `FOLIO-003` versus `FOLIO-002`:
`discover_manifest` failure becomes `FOLIO-001`, `parse_manifest` failure
becomes `FOLIO-003`, and either is accompanied by a `FOLIO-002` "skipped"
finding naming whichever code caused the skip (see decision 0004); a
manifest that parses is handed to `validate_manifest`, whose
`SchemaError.errors` (one entry per violation) becomes one `FOLIO-002`
diagnostic per violation, not one diagnostic wrapping the list.

`path_rules(root, ecosystem)` runs `FOLIO-101`/`FOLIO-102` for one
ecosystem's markers, naming `ecosystem.ecosystem.name` as the finding's
`layer`. It is deliberately generic over `Ecosystem` rather than hardcoded
to `repofolio`/`rust`, so the same function serves both the always-active
core layer and any detected ecosystem.

`repofolio_ecosystem()`'s `markers.must` never lists the manifest filename:
manifest presence belongs to `FOLIO-001` alone, so a repository with no
manifest reports that fact once rather than twice under two different
codes. This is enforced in three independent places — `manifest_rules`
owning the manifest check, `repofolio_ecosystem()`'s marker list omitting
it, and a pinned test asserting the omission — rather than left to be
re-discovered by accident.

## CLI wiring (`crates/repofolio/src/main.rs`)

`Command::Check { path: Option<PathBuf>, format: Format }`. `path` defaults
to the current directory; a given path that is not a directory is a usage
failure (`bail!`, exit `2`). `format` is `Human` (default) or `Json`.

`run_check` calls `repofolio_core::check(&repo_root)`, then grades the exit
status purely from `report.by_severity.error`: `> 0` is exit `1`, otherwise
exit `0` — a warnings-or-info-only report exits clean, so continuous
integration can annotate a build without failing it. Any error surfaced
before that point (a usage failure, a JSON serialization failure) takes the
`Result`-based `Err` path in `main`, which prints the error and exits `2`.

The human-readable renderer (`print_human`) prints one line per finding
(severity label, code, `location_label`, message) followed by a summary
line with the total and per-severity counts. It omits the JSON report's
`layer` field — harmless at this milestone (`repofolio` and `rust` have no
overlapping marker paths, so the file name disambiguates which layer a
finding belongs to), recorded as a real ambiguity to revisit once a
colliding-name ecosystem lands, in
`docs/technotes/m1-deferred-findings.md`.
