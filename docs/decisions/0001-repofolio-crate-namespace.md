# AD-0001 — the `repofolio-*` crate namespace, binary name `folio`

## Context

The M1 workspace was scaffolded under a `folio-*` crate prefix
(`folio-cli`, `folio-core`, `folio-manifest`, `folio-templates`,
`folio-tools`, `folio-fmt`, `folio-release`). Checked against crates.io on
2026-09-06: `folio` itself was published in 2018 and every version yanked —
a yanked crate name is retained permanently, so it can never be reclaimed;
`folio-cli` was taken the day before, 2026-09-05, by an unrelated project;
`folio-core` was taken 2026-03-24. Two of the seven workspace crate names —
and the two most important ones, the front end and the core engine — were
already unavailable.

"folio" is an ordinary English word, so the namespace is naturally
contested; this is not adversarial squatting, but it means the `folio-*`
prefix is not viable as a coherent published namespace going forward.

## Options considered

- **Pick alternate suffixes under `folio-*`** (e.g. rename only the two
  taken crates). Rejected: leaves the workspace's naming convention
  inconsistent, and does nothing about the root word itself remaining
  contested for any crate added later.
- **Move the whole workspace to a coined namespace.** "repofolio" was
  checked and was fully available (`repofolio`, `repofolio-cli`,
  `repofolio-core`), and being coined rather than an ordinary word, it is
  durable rather than merely available today.

## Decision

Every internal crate moves to the `repofolio-*` namespace. The front-end
crate takes the bare name `repofolio` rather than `repofolio-cli`, so the
crate name a user actually installs is the headline one and no empty
umbrella crate has to be published to hold that name. The crate still
builds a binary named `folio` (`crates/repofolio/Cargo.toml`, `[[bin]] name
= "folio"`) — crate name and binary name are independent, the same pattern
`cargo install ripgrep` uses to install `rg`.

Applied 2026-09-06 in commit `441005c`.

## Consequences

- `repofolio` now names two different things at once: the sibling standard
  repository (`driftsys/repofolio`) and this workspace's front-end crate.
  ADR 0003 in that sibling repository already warns to distinguish
  "Repofolio" the standard from "folio" the CLI; this decision makes that
  warning structural rather than a matter of documentation discipline
  alone. See `docs/decisions/0002-consolidate-repofolio-and-folio-repositories.md`,
  which resolves the ambiguity by consolidating the two repositories.
- `cargo install repofolio` is demoted to unsupported convenience in favor
  of checksummed per-platform release assets as the canonical install path
  — a mitigation already present in the M0 plan, not newly introduced here.
- `crates/repofolio-core = { version = "0.1.0", path = "..." }` and its
  sibling entries in `Cargo.toml`'s `[workspace.dependencies]` are the
  version literal every internal crate must move together; see the comment
  there for why this cannot be inherited from `workspace.package.version`.

## Satisfies

`Cargo.toml` (`[workspace.dependencies]`, internal crate entries);
`CLAUDE.md` (Workspace layout); `docs/design/folio-check.md` (crate
boundaries).
