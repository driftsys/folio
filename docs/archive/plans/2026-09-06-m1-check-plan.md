# M1 — `folio check` implementation plan

_Status: working memory. Drafted 2026-09-06. Companion to
`2026-09-06-folio-task-model-design.md`, which covers v0.2–v0.5 and is not
implemented by this plan._

## Goal

`folio check [--format json] [path]` runs end to end against a real
repository, reporting core-layer and Rust-ecosystem conformance with
stable diagnostic codes.

## Scope

Three crates only: `repofolio-manifest`, `repofolio-core`, `repofolio`.

Also in scope, approved separately because they fall outside those crates:
workspace `Cargo.toml` dependency entries, fixtures copied into crate test
trees, a schema parity test, and this repository's own `project.toml`.

## Non-goals

`init` and `add`; templates; tool spawning; `build`, `test`, `pack` and the
rest of the dispatch verbs; the `::` addressing grammar; Foliofile parsing;
plugin ecosystem loading; the Deno and C ecosystems; diagnostic
suppression; SARIF output.

## Success criteria

1. `folio check --format json` emits a document matching the report shape,
   validated by a snapshot test.
2. Exit status is 0 when clean, 1 when any error-severity finding is
   present, 2 on usage or internal failure. Warnings alone leave the status
   at 0 so continuous integration can annotate without failing a build.
3. All three manifest formats parse: `project.toml`, `project.yaml`,
   `project.json`.
4. `cargo clippy --workspace -- -D warnings` and `cargo fmt --check` are
   clean.
5. Running against the `compliant`, `partial` and `empty` fixtures produces
   the expected findings, pinned by tests.

## Dependencies to add

To `[workspace.dependencies]`:

| Crate | Purpose |
| --- | --- |
| `jsonschema` | draft-07 validation of the manifest |
| `yaml-rust2` | YAML parsing to an abstract syntax tree, converted to `serde_json::Value` |
| `anyhow` | error handling in `repofolio` (the front end) only |
| `snapbox` or `assert_cmd` (dev) | command-line acceptance tests |

`toml_edit` gains its `serde` feature. `serde`, `serde_json` and
`thiserror` are already present.

See Part 10 of the design document for why `jsonschema` rather than `boon`,
and `yaml-rust2` rather than any `serde_yaml` fork.

## Review checkpoints

Execution stops for human review at one point:

**After step 6.** Steps 5 and 6 define the two types that v1.0 freezes — the
`Report` shape, which must let SARIF be added later as a serialiser rather
than as a breaking change to the JSON contract, and `Ecosystem`, which must
mirror the `folio.ecosystem.toml` format so the v0.4 loader produces the
same type through the same code path. Both will compile and pass their tests
while being shaped wrong, so the automated gates cannot catch a bad shape
here. Everything from step 7 onward builds on them.

Steps 1-4 and 7-10 run continuously. Their failure modes are mechanical and
the per-step verifications catch them.

## Steps

Each step is test-first: write the failing test, then the implementation,
then verify.

### 1. Manifest discovery

Find `project.toml`, `project.yaml` or `project.json` at the repository
root, in that order.

- Verify: tests covering each format found alone, none present, and more
  than one present.

### 2. Manifest parsing to `serde_json::Value`

`toml_edit` with serde, `yaml-rust2` converted to `Value`, and
`serde_json` directly. Parse failure is distinct from schema failure.

- Verify: the same logical manifest expressed in all three formats
  produces an identical `Value`. A syntactically invalid file in each
  format produces a parse error, not a schema error.

### 3. Bundled schema and validation

Copy `driftsys/schemas/project/v1.json` byte-identically to
`crates/repofolio-manifest/schema/project-v1.json`, embed it with
`include_str!`, and validate with `jsonschema`.

- Verify: the four schema test cases already in `driftsys/schemas`
  (`minimal`, `full`, `invalid-name`, `missing-required`) produce the
  expected outcomes.

### 4. Schema parity test

A test that diffs the bundled copy against
`../../../schemas/project/v1.json` when that checkout is present, and
skips when it is not.

- Verify: the test passes today, and fails if a byte is changed in the
  bundled copy.

This test exists and is enforceable locally via `FOLIO_STRICT_PARITY=1`,
which closes half of verification follow-up 6 in `folio-plan.md`
("Bundled `project` schema vs published `v1.json` parity, asserted in
folio CI"). The other half — assertion in CI — stays open: this
repository has no CI pipeline as of this milestone.

### 5. Report type

`Report { count, by_severity, diagnostics[] }` mirroring
`schemas/markspec/diagnostics/v1.json`. Each diagnostic carries `code`,
`severity`, `message`, and a `location` with at least `file`.

- Verify: a snapshot test of the serialised JSON. The shape must allow
  SARIF to be added later as a serialiser rather than as a breaking change
  to the contract.

### 6. Ecosystem registry

```rust
Ecosystem {
    name: &str,
    always: bool,                 // repofolio is unconditionally active
    markers: Markers { must, should },
    commands: Commands,           // verb -> build_type -> command; empty at M1
}
```

Two instances: `repofolio` (`always: true`) and `rust` (activated by a root
`Cargo.toml` declaring `[workspace]`).

The type must mirror the `folio.ecosystem.toml` file format exactly, so
that the v0.4 loader produces the same type through the same code path
rather than becoming a second implementation. `commands` stays empty at M1
but is present in the type.

- Verify: detection tests for a workspace `Cargo.toml`, a non-workspace
  `Cargo.toml`, and no `Cargo.toml`.

### 7. Rules

| Code | Severity | Rule |
| --- | --- | --- |
| FOLIO-001 | error | Manifest missing or unparseable |
| FOLIO-002 | error | Manifest fails the bundled schema |
| FOLIO-101 | error | Required path missing |
| FOLIO-102 | warning | Recommended path missing |

FOLIO-101 and FOLIO-102 apply per active layer; the finding names the
layer and the path. This is why four codes cover what an earlier draft
split across seven.

`repofolio` required: `README.md`, `LICENSE`, `bootstrap`, `runw`,
`.gitignore`, `.gitattributes`, `.editorconfig`, `docs/`, `scripts/`.
Manifest presence belongs to FOLIO-001 alone — do not also list it here, or
a repository with no manifest reports the same fact under two codes. Recommended: `Foliofile`, `CHANGELOG.md`, `CODEOWNERS`,
`CONTRIBUTING.md`, `.githooks/`.

`rust` required: `Cargo.toml`, `Cargo.lock`. Recommended:
`rust-toolchain.toml`, `rustfmt.toml` or `.rustfmt.toml`.

- Verify: one test per rule, plus fixture-level tests pinning the full
  finding set for `compliant`, `partial` and `empty`.

### 8. Check pipeline

`discover -> parse -> validate -> core rules -> ecosystem detection and
rules -> report`. No stage aborts the run. FOLIO-002 reports as *skipped*
rather than failed when FOLIO-001 has already failed, because there is
nothing to validate. Represent that as severity `info` with a message saying
it was skipped and why: the mirrored diagnostics shape has only
error/warning/info, `info` keeps exit-code grading correct, and it maps to
SARIF later as level `note` with kind `notApplicable`.

- Verify: a repository with no manifest still produces FOLIO-101 and
  FOLIO-102 findings, and FOLIO-002 is marked skipped.

### 9. Command-line wiring

`folio check [--format json] [path]` with the exit-status contract.

- Verify: acceptance tests asserting exit status and output for a clean
  fixture, a warnings-only fixture, and an error fixture.

### 10. Dogfood

Add `project.toml` to this repository and run `folio check` against it and
against a local `git-std` checkout.

- Verify: findings are accurate. This repository is expected to fail
  honestly — it has no `LICENSE`, `bootstrap`, `runw`, `.gitattributes`,
  `.editorconfig` or `scripts/`. Rust markers pass, since `Cargo.toml` and
  `Cargo.lock` are both present, while `rust-toolchain.toml` and
  `rustfmt.toml` warn.

## Known conformance gap in this repository

`.gitignore` currently lists `Cargo.lock`, which Appendix A requires to be
committed for a workspace. A path-existence check does not catch this,
because the file exists on disk. It needs the `gitignored` rule kind from
Part 6 of the design document, which is not in M1 scope. Record it rather
than fix it silently.

## Open items

1. **Suppression.** The v0.1 exit criteria require "ticketed exceptions",
   and nothing expresses one. The sibling `driftsys/repofolio` standard
   repository's own manifest produces two genuine FOLIO-002 failures
   (object-form `authors`, an unexpected `versioning` key — the
   schema-versus-prose drift recorded in the design document at Part 8
   item 2), so the gate cannot be met as written for that repository.
   folio's own manifest validates cleanly. Decide at the dogfood gate.
2. **Crate rename — resolved 2026-09-06 in commit 441005c.** `folio-cli`
   and `folio-core` were already taken on crates.io, so the workspace moved
   to the `repofolio-*` namespace. The front-end crate took the bare name
   `repofolio` and still builds a binary named `folio`.
