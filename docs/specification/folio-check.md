# `folio check` — specification

`folio check [--format json] [path]` audits a repository's conformance to
the Repofolio standard. It reports manifest validity, then the presence of
required and recommended files, per active layer — the core `repofolio`
layer, always active, plus any ecosystem detected in the repository (`rust`
at this milestone). Findings surface as stable `FOLIO-xxx` diagnostic codes,
in either a JSON report or human-readable text, graded to a process exit
status a continuous-integration job can act on.

## Manifest discovery

The project manifest is expressed in exactly one of three serialization
formats at the repository root: `project.toml`, `project.yaml`, or
`project.json`. When more than one is present, discovery selects
`project.toml` over `project.yaml`, and `project.yaml` over `project.json`,
silently — no diagnostic reports that a second manifest file was ignored.
This is a scoped gap, not an oversight: recorded as a candidate for a future
`FOLIO-` code in `docs/technotes/m1-deferred-findings.md`.

## Manifest parsing

Each of the three formats parses to the same canonical value, so every rule
downstream operates on one representation regardless of how the manifest was
written on disk. A manifest that cannot be read from disk, or that is
syntactically invalid in its own format (malformed TOML, unparseable YAML,
malformed JSON), is a parse failure. YAML parsing additionally rejects, as a
parse failure rather than silently producing a null value, a value
`yaml-rust2` itself cannot represent: an unresolved anchor alias, or a
floating-point literal that does not parse to a finite `f64`. This matters
because the manifest's `metadata` object accepts any properties, so a
silently nulled value there would otherwise pass schema validation instead
of being caught.

## Schema validation

A manifest that parses is validated against the Repofolio project manifest
schema (JSON Schema draft-07), bundled into the `repofolio-manifest` crate at
compile time and never fetched over the network. The bundled copy must stay
byte-identical to the canonical schema published in `driftsys/schemas`. A
manifest that fails validation reports one finding per independent
violation, not one finding wrapping the whole failure, so a count of
findings reflects the real violation count.

## The `FOLIO-` diagnostic registry

Five diagnostic codes, and five only, at this milestone:

| Code | Severity | Condition |
| --- | --- | --- |
| `FOLIO-001` | error | The manifest is missing — none of `project.toml`, `project.yaml`, `project.json` exists at the repository root. |
| `FOLIO-002` | error (`info` when skipped) | The manifest parses but fails the bundled schema. Reported at `info` severity, with a message naming the reason, when `FOLIO-001` or `FOLIO-003` has already failed and there is nothing to validate. |
| `FOLIO-003` | error | A manifest file was found but is unreadable or fails to parse in its own format (malformed TOML, unparseable YAML, malformed JSON). |
| `FOLIO-101` | error | A required path for an active layer is absent from the repository. |
| `FOLIO-102` | warning | A recommended path for an active layer is absent from the repository. |

`FOLIO-001` and `FOLIO-003` were one code (`FOLIO-001`, "manifest missing or
unparseable") through the rest of this milestone's development and were
split before this branch merged: the two failures have different remediation
for a consumer (`folio init` versus fixing the manifest's syntax) and were
indistinguishable by diagnostic code. Splitting a shipped code after v1.0
would break consumers who branch on it; splitting it now, before any release,
is free.

Manifest presence belongs to `FOLIO-001` alone. A repository with no
manifest reports that fact once; the manifest filename is never also listed
among an active layer's required paths, which would report the same fact a
second time under `FOLIO-101`.

`FOLIO-101` and `FOLIO-102` apply once per active layer, and a finding names
which layer produced it. A required or recommended entry may itself be an
alternative group — satisfied by any one of several acceptable spellings
(for example `rustfmt.toml` or `.rustfmt.toml`) — in which case a missing
group produces exactly one finding naming every alternative, not one finding
per alternative: a repository satisfying the recommendation through either
spelling is not warned about the other.

### Core layer (`repofolio`) — always active

Required: `README.md`, `LICENSE`, `bootstrap`, `runw`, `.gitignore`,
`.gitattributes`, `.editorconfig`, `docs/`, `scripts/`.

Recommended: `Foliofile`, `CHANGELOG.md`, `CODEOWNERS`, `CONTRIBUTING.md`,
`.githooks/`.

### `rust` layer — active when the repository root's `Cargo.toml` declares a
`[workspace]` table

A `Cargo.toml` alone does not activate this layer: a single, non-workspace
crate has one too. Activation is a dedicated content check against
`[workspace]`, not a marker-presence test, and a missing or unparseable
`Cargo.toml` is treated as "not detected" — that failure mode belongs to
`FOLIO-001`/`FOLIO-003` against the project manifest, not to ecosystem
activation.

Required: `Cargo.toml`, `Cargo.lock`.

Recommended: `rust-toolchain.toml`; the alternative group `rustfmt.toml` or
`.rustfmt.toml`.

## Report shape

`folio check --format json` emits one JSON document:

```json
{
  "count": <total finding count>,
  "bySeverity": { "error": <n>, "warning": <n>, "info": <n> },
  "diagnostics": [
    {
      "severity": "error" | "warning" | "info",
      "code": "FOLIO-xxx",
      "message": "<human-readable finding text>",
      "layer": "<ecosystem name>",
      "location": { "file": "<path>", "line": <n>, "column": <n> }
    }
  ]
}
```

`count` and `bySeverity` are always derived from `diagnostics` and can never
disagree with it. `layer` is present only for per-layer findings
(`FOLIO-101`, `FOLIO-102`); manifest-level findings (`FOLIO-001`,
`FOLIO-003`, `FOLIO-002`) omit it, since they apply once to the whole
repository rather than once per active layer. `location.line`/
`location.column` are present only for a rule kind that can point at a
specific position within a file; no rule at this milestone populates them.

## Exit status contract

- `0` — clean, or warnings/info findings only. Continuous integration can
  annotate a build with these without failing it.
- `1` — at least one error-severity finding is present.
- `2` — a usage failure (for example, a given path that is not a directory)
  or an internal failure (for example, a report that fails to serialize).

## Dogfood requirement

`folio check` must run against this repository itself, and against a local
checkout of `git-std`, before this milestone is considered complete, and the
findings it reports for both must be accurate — not merely a run that
completes without crashing.

## Known conformance gap

This repository's `.gitignore` lists `Cargo.lock`, unanchored, which matches
every `Cargo.lock` in the tree — including the workspace root, which
`folio`'s own `rust`-layer requirement expects to be committed. A
path-existence check does not catch this, because the file exists on disk;
catching it needs a `gitignored` rule kind, which is out of scope for this
milestone. Recorded rather than fixed silently.
