# AD-0002 — consolidate `driftsys/repofolio` and `driftsys/folio` into one repository

## Context

On 2026-08-29 (second pass that day), the project decided to keep the
Repofolio standard and the folio CLI in two separate repositories:
`driftsys/repofolio` holding the spec markdown, the published schema, and
the normative `tests/fixtures/` (compliant/partial/empty); `driftsys/folio`
(this repository) holding the CLI, its own crate tests, and the Foliofile
diagnostic corpus. The stated rationale was that folio's software release
cadence (v0.1 → v1.0, a build matrix, signing) does not belong in a
repository versioned by schema discriminator.

On 2026-09-06, `docs/decisions/0001-repofolio-crate-namespace.md` recorded
that two of the seven workspace crate names were already taken on
crates.io, because "folio" is an ordinary word and therefore a naturally
contested namespace, while "repofolio" — coined — was fully available. That
same fact bears on the repository-split question: the namespace collision
that justified separating the two repositories' concerns does not, on
inspection, actually depend on keeping them in two repositories.

## Options considered

- **Keep the two-repository split.** This is the standing decision as of
  2026-08-29; re-examining it was prompted by the crate-naming collision,
  not by a new problem with the split itself.
- **Consolidate into one repository**, reinstating ADR 0003
  (`standard-and-cli-same-repo`) from `driftsys/repofolio`'s own
  `docs/adr/`, which was still marked Accepted there and was never updated
  when the 2026-08-29 split superseded it.

## Decision

Consolidate `driftsys/repofolio` and `driftsys/folio` into one repository.
This reverses the 2026-08-29 split decision (`folio-plan.md`, M0 item 1)
and reinstates ADR 0003.

The split's technical objection — a software release cadence does not
belong in a repository versioned by schema discriminator — dissolves on
inspection: the standard's version discriminator is the manifest's
`"$schema"` URL, not the repository's own `version` field. folio's semantic
version and the schema's version are orthogonal, not in conflict, so
sharing a repository does not entangle the two release cadences after all.

## Consequences

- `CLAUDE.md`'s open question about pinning `repofolio`'s fixtures by
  submodule or vendoring script disappears once the merge lands: the
  fixtures live in the same repository directly.
- The bundled-schema parity check (`crates/repofolio-manifest/tests/schema_parity.rs`)
  remains cross-repository regardless of this decision: `project/v1.json`
  lives in `driftsys/schemas`, a separate, organization-wide shared
  repository that this decision does not touch.
- ADR 0003 in `driftsys/repofolio/docs/adr/` should itself be updated to
  record that it was superseded on 2026-08-29 and reinstated on 2026-09-06,
  with this reasoning — that is an edit to a different repository, outside
  the scope of this record, and is raised as an offer rather than made
  here.
- **This is a documentation-level decision only.** The physical repository
  merge — moving `driftsys/repofolio`'s files into `driftsys/folio`, or
  vice versa — has not happened as of this record. `CLAUDE.md`'s "Repo
  split" section should be corrected to reflect that this decision
  supersedes it, but the two repositories remain two separate checkouts
  until the merge is actually carried out.

## Satisfies

`CLAUDE.md` ("Repo split" section — flagged for human correction, not
edited by this record); `docs/planning/folio-plan.md` (M0 item 1, not
edited — planning docs are out of scope for this gardening pass).
