# CLAUDE.md — folio

Read `docs/planning/folio-plan.md` and `docs/planning/folio-roadmap.md` in full
before writing code. This file is a pointer + a condensed cheat sheet, not a
replacement for them. If anything below conflicts with the plan/roadmap, the
plan/roadmap win — this file may lag.

## What this repo is

`folio` — the reference CLI implementation of the **Repofolio standard**
(a tool-agnostic spec that lives in the separate `driftsys/repofolio` repo).
Part of the `driftsys` org, alongside `git-std`, `prim`, `upskill`, `dock`,
`schemas`, and `repofolio`.

**Repo split (decided 2026-08-29, second pass):** `repofolio` holds the
spec markdown, published schema, and the normative `tests/fixtures/`
(compliant/partial/empty — language-agnostic, not folio-specific). This
repo holds the CLI, its own crate tests, and the Foliofile diagnostic
corpus (v0.3+). folio's tests pin `repofolio`'s fixtures as a dependency
(submodule or vendoring script — not yet decided; pick the simpler one
when you get there).

## Non-negotiable architecture decisions

1. **Binary orchestration, not crate linking.** folio invokes pinned tool
   binaries (git-std, prim, upskill, shfmt, shellcheck, rustfmt, …) via their
   CLIs and consumes `--format json` — the same contract agents and CI use.
   **Do not** add git-std/prim/upskill as Cargo dependencies to link their
   Rust APIs. The only sanctioned exception is a possible `prim-core` link
   inside a future LSP (v0.8), decided on measured latency, not now.
2. **Sync only, everywhere except `mcp`.** No tokio in `folio-core`,
   `folio-manifest`, `folio-templates`, `folio-tools`, `folio-fmt`,
   `folio-release`, or `folio-cli`'s non-mcp paths. Async is confined to the
   future `folio mcp` subcommand (rmcp), feature-gated.
3. **Plan/apply for every mutating verb.** Any verb that changes state must
   support a dry-run that emits a JSON plan, and applying an unchanged plan
   must produce exactly the stated effects. `folio version` verifies this by
   diffing actual git-std output against the plan it approved.
4. **Facades own policy, tools stay ignorant.** `folio-fmt` wraps `prim` via
   `Command` + JSON; `folio-release` wraps `git-std` the same way. Policy
   (single-version mode, `FOLIO-` code mapping, walking boundary) lives in
   these facade crates, not upstream.
5. **folio owns repo walking.** Facades pass explicit file lists to spawned
   tools; tools don't walk on folio's behalf.
6. **Tool contract, 5 clauses** — every orchestrated tool must eventually
   have: stable diagnostic codes, `--format json`/SARIF on check verbs,
   dry-run/plan JSON on mutating verbs, a `<tool> registry --format json`
   dump, and installable per-platform release assets + checksums. folio
   itself must meet this contract from v0.1 onward (dogfooding it is the
   point).
7. **No shims, no PATH mutation.** `folio tools` invokes everything by
   absolute path from `~/.cache/folio/tools/<name>/<version>/<target>/`.

## Workspace layout

```
crates/
  folio-cli        — bin, thin clap front-end, no business logic
  folio-core       — check engine, scoring, FOLIO- rule registry
  folio-manifest   — parse project.toml (+yaml/json), bundled-schema validation
  folio-templates  — handlebars ecosystem scaffolding, managed sections
  folio-tools      — installer: resolve/fetch/verify/cache/invoke pinned tools
  folio-fmt        — facade: spawns prim + shfmt/shellcheck
  folio-release    — facade: spawns git-std (version/changelog/hooks)
```

`folio-task` (Foliofile runner, v0.3) and `folio-mcp`-shaped code (v0.6) are
not yet scaffolded — add them when their milestone starts, per the roadmap.

## Milestone you are almost certainly working on: M1 / v0.1 "check + init"

Zero upstream dependencies. Scope, in order:

1. `folio-manifest`: parse `project.toml` (toml first; yaml/json can follow),
   validate against a **bundled** copy of the schema (never fetched at
   runtime — see `docs/planning/folio-plan.md` verification follow-up #6 on
   keeping the bundled copy in parity with the published one).
2. `folio-core`: core-layer MUST/SHOULD/MAY rules, ecosystem-marker
   detection, a `FOLIO-xxx` code per rule, a `Report` type that serializes
   cleanly to the tool-contract JSON shape.
3. `folio-cli`: `folio check [--format json]` wired end to end. `folio init`
   / `folio add <ecosystem>` come after `check` has a vertical slice — don't
   parallelize these inside one session.
4. Fixtures: reuse `tests/fixtures/` (compliant / partial / empty) from the
   existing repofolio repo if present; otherwise create minimal ones first.
5. **Dogfood gate for this milestone:** `folio check` should run against
   this very repo and against a local checkout of `git-std` before the
   milestone is called done.

Do not start `folio-tools`, `folio-fmt`, or `folio-release` work yet — they
belong to v0.2 and v0.5 and depend on upstream stories (prim F3, git-std C3)
that may not be done. Check `docs/planning/folio-plan.md` Part 1 before
touching either.

## Conventions

- Conventional Commits (this repo will eventually dogfood git-std itself
  once v0.5 lands; until then, just follow the convention by hand).
- `cargo clippy --workspace -- -D warnings` and `cargo fmt --check` must be
  clean before any commit — treat clippy as your primary iteration signal,
  not an afterthought.
- Every new public function in a lib crate gets a doc comment stating which
  plan/roadmap phase it belongs to, if non-obvious.
- When a design question isn't answered in the two planning docs, stop and
  ask rather than guessing — several boundary questions (walking ownership,
  version-file detection split) are noted as "decided" in the plan; don't
  re-litigate them silently.

## Do not

- Add `standard-version`, `standard-changelog`, `standard-githooks`,
  `standard-commit`, or any `prim-*` crate as a dependency anywhere in this
  workspace. If you think you need one, re-read Integration architecture
  decision #1 in `folio-plan.md` first.
- Introduce `curl | sh` install flows for anything `folio tools` is meant to
  manage.
- Add new top-level `project.toml` fields for tool-specific config — those
  live under `config.<tool>` (see the manifest namespacing principle).
