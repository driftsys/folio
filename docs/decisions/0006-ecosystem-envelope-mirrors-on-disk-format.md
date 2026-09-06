# AD-0006 — `Ecosystem` mirrors the on-disk `folio.ecosystem.toml` envelope; `always` cannot be set from a loaded file

## Context

`Ecosystem` is the type both M1's two hand-built instances (`repofolio`,
`rust`) and a future v0.4 `folio.ecosystem.toml` loader must produce,
through the same code path — not a second, parallel implementation that
translates between a hand-rolled M1 shape and the real file format. A
divergence between the intended mirror and the type as first built was
found only by deserializing the one real reference file that exists,
`driftsys/repofolio/ecosystems/rust/folio.ecosystem.toml`; every earlier
review round had verified the type by reading it, not by executing it
against that file.

## Decision

`Ecosystem`'s fields nest `name`/`version` under an `ecosystem:
EcosystemMeta` field, matching the real `[ecosystem]` TOML table, rather
than flattening them onto `Ecosystem` itself — a flat field could not
deserialize that table without a second, parallel translation layer, which
is exactly what this mirror exists to avoid. `EcosystemMeta.version` is the
ecosystem *file format* version (an integer; the real file sets it to `1`),
not a semantic version of the ecosystem's own tooling.

`schema: Option<String>` mirrors the file's top-level `"$schema"` key,
carried rather than dropped: a real file missing it must still deserialize,
and its value is how a future loader tells which schema version a file was
authored against.

`markers: Markers` and `commands: Commands` are both `#[serde(default)]`,
because the real `rust` scaffold file omits `[markers]` and `commands`
entirely — its own comment states templates, sections, and commands "are
intentionally omitted from the v0.1 scaffold." Without `#[serde(default)]`
on `markers`, that real file fails to deserialize with "missing field
`markers`" — the defect a review round of this type's construction found.

`always: bool` — true only for the built-in, unconditionally active
`repofolio` layer — is `#[serde(skip)]`, not `#[serde(default)]`. A
`default` still lets an incoming file set the field explicitly (it only
supplies a fallback when the field is absent); `skip` removes the field
from the deserialized schema entirely, so an `always = true` key anywhere
in a loaded `folio.ecosystem.toml` — built-in, repository-local, or a
pinned remote source — is not read as this field at all. No ecosystem
loaded from a file can grant itself unconditional activation; only the
built-in `repofolio_ecosystem()` constructor, in code, can.

## Consequences

- `Ecosystem`, `EcosystemMeta`, and `Markers` are all `#[non_exhaustive]`:
  this type does not yet mirror every field the on-disk format's Part 6
  description names (`[[rules]]`, and the future
  inputs/templates/patches/sections shape), and marking it non-exhaustive
  keeps room to add those fields without a breaking change to a type
  frozen at v1.0.
- Shape enforcement for `commands` (e.g. distinguishing
  `commands.build.debug` from a malformed `commands.debug.build`) is not
  done by this type: a malformed file deserializes without error today and
  fails only at command lookup. That enforcement is deferred to
  `schemas/folio-ecosystem/v1.json`, which does not yet exist.
- **Process note.** A fabricated literal test can pass while the real file
  it claims to mirror fails to deserialize — this happened twice in the
  same milestone (the `Ecosystem` mirror above, and separately a fixture
  `Cargo.lock` masked by an unanchored `.gitignore` pattern), and a third
  time a test's own assertion was inert and only mutation testing caught
  it. The only reliable check for "does this type read the real file" is
  deserializing the actual reference file, or reading it live from a
  sibling checkout when present (`FOLIO_STRICT_PARITY` gates whether a
  missing sibling checkout skips or fails that read). See
  `docs/technotes/m1-deferred-findings.md` for the full account.

## Satisfies

`crates/repofolio-core/src/ecosystem.rs` (`Ecosystem`, `EcosystemMeta`,
`Markers`, `always`); `docs/design/folio-check.md` (ecosystem registry
section).
