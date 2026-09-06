# folio task model — design

_Status: working memory. Drafted 2026-09-06. Garden into `docs/decisions/`
when the affected milestones land._

_Scope: the verb and recipe model for `folio run`, its addressing scheme,
its interaction with git hooks and CI, and the ecosystem plugin format.
Binds folio v0.2–v0.5 and requires changes to the Repofolio standard in
`driftsys/repofolio`. Does not change folio v0.1 (M1); see the companion
plan `2026-09-06-m1-check-plan.md`._

## Context

The roadmap fixed the milestone order but left the task vocabulary
underspecified, and the parts that were specified conflict across
documents. This design settles the vocabulary against four consumers —
git hooks, CI, agents, and the editor/LSP — rather than against any one
of them.

## Governing principles

These generated every decision below. Where a later question arises, apply
these rather than re-arguing from cases.

1. **Cost and trigger decide the verb, not subject matter.** A rule that
   completes in seconds belongs to `check`; in tens of seconds, to `lint`;
   in minutes, to `build`. This is measurable, so contributors can place a
   new rule without debate.
2. **Name the default for what it is; name exceptions for why they are
   exceptional.** `test` is the host default. `device` is named for the
   resource that makes it an exception.
3. **`all` is what runs offline, at a developer's desk, with nothing
   attached.** Any recipe needing network, isolation, or hardware is
   excluded from it.
4. **Resource requirements are declared in `config.folio`, never encoded
   in a verb name.** One name, one meaning; scheduling data lives in
   configuration.
5. **The core layer is an ecosystem.** `repofolio` is an always-active
   ecosystem, so conformance is `repofolio::check` and the core layer uses
   the same rule mechanism as every other layer.

## Part 1 — Verbs

### Work verbs

| Verb | Layer | Cost | Rust | Deno | C |
| --- | --- | --- | --- | --- | --- |
| `fmt` | ecosystem | seconds | `cargo fmt` | `deno fmt` | `clang-format` |
| `fix` | ecosystem | seconds | `cargo fix`, `clippy --fix` | `deno lint --fix` | `clang-tidy --fix` |
| `check` | core + ecosystem | seconds | `cargo check` | `deno check` | compile syntax pass |
| `lint` | ecosystem | tens of seconds | `cargo clippy` | `deno lint` | `clang-tidy` |
| `build` | ecosystem | minutes | `cargo build` | none (JIT runtime) | `cmake --build` |
| `test` | ecosystem | minutes | `cargo test` | `deno test` | `ctest` |
| `spec` | ecosystem | minutes | spec crate | — | — |
| `device` | ecosystem | minutes | `probe-rs` | — | HIL runner |
| `bench` | ecosystem | minutes | `cargo bench` | `deno bench` | google-benchmark |
| `audit` | ecosystem | seconds + network | `cargo audit` | — | dependency-check |
| `pack` | ecosystem | minutes | cross-compile per target | `deno compile --target` | `cpack` |
| `sign` | ecosystem | seconds | cosign | cosign | cosign |
| `publish` | ecosystem | seconds + network | crates.io | JSR | — |
| `deploy` | ecosystem | seconds + network | release assets | release assets | — |

Not every ecosystem implements every verb. The fan-out skips what does not
exist, and `folio run` lists only what is available in the current
repository.

### Moment recipes

```makefile
pre-commit:  fmt check                    # about 2 seconds
pre-push:    lint                         # about 30 seconds
all:         fmt lint check build test    # minutes; offline, no hardware
```

`audit`, `bench`, `device` and the whole release tail are excluded from
`all` by principle 3.

### Listing

No `default` recipe is shipped in the `folio:core` managed section.
Grammar clause 6.2.2 then applies: bare `folio run` lists available
recipes and exits 0. The listing is filtered to top-level recipes and
annotated with availability; qualified recipes are shown behind a flag.
This matches `deno task` and `npm run`, and it prevents a bare `folio run`
from starting a multi-minute build.

### Verbs that folio does not have

`folio` implements four work verbs natively: `fmt`, `fix`, `lint`, `check`.
Everything else is dispatched to an ecosystem's declared command. `folio
build` resolves as an alias for `folio run build`.

Resolution rule, which prevents infinite recursion when a recipe body
calls a folio verb:

> A native verb wins. Any other name falls through to `folio run <name>`.

Escape hatches: `folio check` always means the native verb; `folio run
check` always means the recipe.

## Part 2 — Addressing

```text
build                          all ecosystems (fan-out)
rust::build                    one ecosystem
rust::repofolio-manifest::build  one module
rust::*::build                 glob at segment level
folio::build                   the unshadowable built-in
```

Grammar:

```abnf
recipe-name = segment *("::" segment)
segment     = identifier / "*"
```

### Why `::` and not `-` or `:`

- `-` is a legal identifier character (grammar 4.3), so it cannot separate
  segments. `rust-repofolio-manifest-build` is ambiguous: this workspace's
  own crate names (`repofolio-manifest`, `repofolio-templates`) make the
  ambiguity concrete, not hypothetical.
- A single `:` is `just`'s dependency separator. `just` treats `rust : build`
  and `rust:build` identically, so folio could not distinguish a qualified
  name from a recipe with a dependency when reading an existing justfile.
- `just` raises a hard parse error on `::` and never misinterprets it.
  Verified against just 1.38.0.

Consequences of `::`:

- No filename-based dialect switch is needed to read existing justfiles.
- The managed-section marker `folio:rust` (one colon, grammar 4.8) stays
  lexically distinct from the recipe path `folio::rust::build`.
- A Foliofile using no qualified names remains a valid justfile.
- Ordering is ecosystem first, verb last, matching Gradle's `:module:task`
  and Maven's `plugin:goal`.
- Resolution requires the ecosystem registry: `rust::build` can only be
  read as ecosystem-then-verb by knowing that `rust` is an ecosystem.

### Extension rather than override

`folio::` namespaces the whole implicit tree, not only the aggregate. This
is what allows a repository to add to a built-in instead of replacing it:

```makefile
build: folio::build          # run the built-in, then add a step
    ./scripts/package.sh

build: prepare folio::build  # or add a step before it
```

Pre-step ordering works because grammar 6.1.2 guarantees dependencies run
left to right, each completing before the next. Gradle would require
`mustRunAfter` for the same effect.

## Part 3 — Build types

Three types, following the AOSP variant ladder:

| Type | Optimisation | Symbols | Signing | Platform |
| --- | --- | --- | --- | --- |
| `debug` | none | full | debug keys | host |
| `staging` | as release | retained, not obfuscated | staging keys | userdebug |
| `release` | full | stripped, obfuscated | production keys | user |

The build type is derived from the verb, not chosen separately:

| Verb | Build type |
| --- | --- |
| `check`, `lint`, `test`, `spec`, `build` | debug |
| `device`, `bench` | staging |
| `pack`, `sign`, `publish`, `deploy` | release |

`bench` uses staging rather than release because staging has release
optimisation with symbols retained, which yields valid timings and
attributable profiles. This is the same reasoning behind AGP's
`isProfileable`. One staging artifact therefore serves both `device` and
`bench`, so the expensive cross-compilation is paid once.

Rust:

```toml
[profile.staging]
inherits = "release"
debug = true            # symbols; no runtime cost
strip = "none"
overflow-checks = false # keep staging timings faithful to release
```

Android: `initWith(release)`, `-dontobfuscate` in the ProGuard rules,
`isProfileable = true`, a staging `signingConfig`, and `matchingFallbacks
= ["release"]` so dependencies without a staging type still resolve.

**Security invariant:** only `release` uses production signing keys. This
is checkable, and it prevents production keystores from being wired into
per-merge-request CI jobs.

## Part 4 — Hooks

Two implicit hook recipes:

```makefile
folio::pre-commit:  fmt check
folio::pre-push:    lint
```

`git-std` retains the hook runtime and its prefix semantics (`~` stash,
run, re-stage, restore; `!` required; `?` advisory). folio owns the
content. The `.hooks` files reduce to one line each:

```text
# pre-commit.hooks
~ folio run pre-commit

# pre-push.hooks
! folio run pre-push
```

Rationale: recipes have no advisory mode (grammar 6.1.5 terminates on any
non-zero exit), and grammar section 11 deliberately excludes attributes,
so an `[advisory]` annotation would reverse a stated exclusion. Keeping
the prefixes in `git-std` avoids that.

`conformance` at pre-commit is affordable because `repofolio::check` is
file-system input/output only, with no compilation, no network and no tool
spawning.

Note: `pre-push.off` is present in both `git-std` and `repofolio` today, so
that tier currently runs nowhere. Enabling it is a behavioural change.

**PATH rule.** folio is the only binary required on `PATH`. It resolves
every pinned tool internally by absolute path. Without this, hooks — fired
by git with a minimal environment — would either embed cache paths that
break on every version bump, or silently violate architecture decision 7.

## Part 5 — Release path

`pack` and `publish` are parallel channels, not sequential steps.
`cargo publish` and `deno publish` send source to a registry for
developers; cross-compiled binaries go to release assets for end users.
`git-std` does both independently.

```text
pack     produce artifacts per target, plus SBOM and provenance
sign     signatures over the artifacts and attestations
publish  to package registries      (developers)
deploy   to stores or servers       (end users)
```

SBOM and provenance are generated by `pack` because that step holds the
resolved lockfile and the artifact together. `sign` then does one thing.
A repository without signing keys simply does not implement `*::sign`.

The manifest's `category` selects the path:

| `category` | Path |
| --- | --- |
| `library` | `publish` to a registry |
| `application`, `service` | `deploy` to a store or server |
| `tool`, `binary` | both — `git-std` declares `["tool", "binary"]` |
| `prebuilts` | the committed `releases/vX.Y.Z/` tree |

`publish` and `deploy` are both irreversible and outward-facing, so both
require a dry-run plan under architecture decision 3.

`dist` was considered and rejected: its work splits between `pack`
(artifacts) and `deploy` (staging for upload), and `release` is already
overloaded three ways.

## Part 6 — Ecosystems and plugins

Built-in: `rust`, `deno`, `c`. Android, AOSP and Unity become plugins.

An ecosystem is a data directory containing no code:

```text
ecosystems/<name>/
├── folio.ecosystem.toml
├── templates/
└── sections/
```

```toml
"$schema" = "https://driftsys.github.io/schemas/folio-ecosystem/v1.json"

[ecosystem]
name = "unity"
version = 1

[markers]
must   = ["ProjectSettings/ProjectVersion.txt", "Assets"]
should = [".editorconfig"]

[commands.build]
debug   = "unity -batchmode -quit -projectPath ."
release = "unity -batchmode -quit -projectPath . -release"
```

Markers give both detection and the FOLIO-101/102 conformance rules.
`commands` is keyed `verb → build_type → command`.

Three sources, layered like tool source classes: built-in (compiled in),
repository-local (`ecosystems/` in the repository), and remote (a git
repository at a pinned tag). Local shadows built-in.

**An ecosystem definition is executable content.** It contains command
strings that will be run. Remote ecosystems therefore require the same
pinning and checksum discipline `folio tools` applies to binaries: exact
tag, sha256 in `.folio.lock`, no version ranges.

### Declarative rule vocabulary

Marker presence does not cover every rule an ecosystem needs. Rather than
allow plugin code, extend a closed vocabulary:

`path-present` · `path-absent` · `gitignored` · `content-match` ·
`content-absent`

```toml
[[rules]]
id       = "UNITY-001"
severity = "error"
kind     = "path-absent"
paths    = ["Library/", "Temp/"]
message  = "Unity regenerates these; they must not be committed"

[[rules]]
id      = "RUST-001"
kind    = "content-match"
file    = "Cargo.toml"
pattern = '(?m)^resolver = "[23]"$'
message = "Workspace must declare resolver 2 or higher (Appendix A)"
```

Two constraints:

1. **Content rules must name a bounded file set.** An open glob such as
   `**/*.rs` would break the `check` budget and remove it from the
   pre-commit tier.
2. **Use Rust's `regex` crate, not a backtracking engine.** Ecosystem
   definitions can be fetched from third parties, and a pattern with
   catastrophic backtracking would be a denial of service against a
   pre-commit hook. Linear-time matching makes that structurally
   impossible rather than a review problem.

**Code namespacing.** Plugins cannot mint `FOLIO-` codes, since that
registry freezes at v1.0. Codes are namespaced by ecosystem name —
`RUST-001`, `UNITY-001` — which needs no central registry and cannot
collide.

### The C ecosystem has an extra layer

Appendix B activates on any of four markers with entirely different
commands (`CMakeLists.txt`, `meson.build`, `BUILD.bazel`, `xmake.lua`).
The language-level rules (`.clang-format`, `.clang-tidy` MUST) belong to
`c`; commands are keyed by the detected build system:

```toml
[commands.cmake.build]
debug = "cmake --build build --config Debug"
[commands.meson.build]
debug = "meson compile -C build"
```

`rust` and `deno` leave that level empty.

## Part 7 — Required changes to `driftsys/repofolio`

1. **Grammar 4.4** — replace `recipe-name = identifier [":" identifier]`
   with `recipe-name = segment *("::" segment)`, `segment = identifier /
   "*"`.
2. **Grammar section 1, goal 1 and section 6** — delete the claims that a
   Foliofile parses identically in `just` and is a valid justfile.
   Compatibility runs in one direction only: folio reads justfiles, for
   migration. Grammar section 2 already permits this via the
   `foliofile-extension` warning; promote it from a MAY to the stated goal.
3. **Grammar 6.2 clause 2** — specify that the listing is of *top-level*
   recipes, with qualified recipes behind a flag. Otherwise
   implementations will differ.
4. **Grammar 6.3** — state that resolution is registry-dependent.
5. **Grammar section 11** — note that GNU make uses `::` for double-colon
   rules, so a make user may misread the separator.
6. **`directory-layout.md`** — remove `assemble`; add `spec`, `device`,
   `bench`, `audit`, `pack`, `sign`, `publish`, `deploy`, `all`. Change
   `check` from "Run compliance check" to the per-ecosystem fast-validity
   verb.
7. **`directory-layout.md`** — add `examples/` to Appendix A. It is one of
   Cargo's five target types and is already compiled by `cargo test`, so
   it needs no recipe, only a layout entry.
8. **Appendix E** — write the Deno ecosystem appendix. It does not exist,
   so a built-in `deno` ecosystem currently has no marker table.
9. **`ci-cd.md`** — move codegen drift out of the sanity-check list. It
   re-runs the entire codegen pipeline, which is minutes, and cannot be a
   fast-fail step.
10. **Resolve two naming inconsistencies** with grammar 7.3: `deno` versus
    `ts`, and `c` versus `cpp`.

## Part 8 — Defects found in the standard

Recorded so they are not rediscovered.

1. `just` cannot parse colon-qualified recipe names. This voids the
   portability guarantee and is a fifth open grammar point, additional to
   the four already tracked.
2. The canonical `schemas/project/v1.json` and the prose in
   `project-manifest.md` disagree. The schema requires only `name` and
   `version` and sets `additionalProperties: false`; the prose additionally
   requires `$schema`, `versioning` and `category`, and describes `authors`
   as objects. `repofolio`'s own `project.yaml` fails its own schema on
   `versioning` and on object-form `authors`. The schema is newer as a
   deliberate decision (commit of 2026-03-08, "require only name and
   version"), and `git-std` follows the schema.
3. `repofolio/schemas/project-v1.json` is a stub whose description says so.
   The real schema is in `driftsys/schemas`.
4. `build` carries five meanings, two of them normative and in the same
   document set: the recipe table says compile, the CI stage table says
   compile plus lint plus format.
5. `assemble`'s contents differ between `directory-layout.md` (includes
   `check`) and the grammar's worked example (does not).
6. `spec/` and `bench/` are mandated directories with no recipe; `audit`
   is required by `security.md` with no recipe.
7. `schemas/folio-ecosystem/v1.json` is referenced by every ecosystem
   scaffold file and does not exist.
8. Four different ecosystem lists exist: the roadmap says rust and deno,
   `ecosystems/` holds rust, android and aosp, grammar 7.3 lists seven
   sections including `ts` and `unity`, and the appendices cover rust,
   c/c++, android and aosp. Only `rust` appears in all four.
9. Appendices C and D are stubs.
10. No mechanism expresses a suppressed diagnostic, yet the v0.1 exit
    criteria require "ticketed exceptions".
11. `pre-push.off` in both `git-std` and `repofolio`: that hook tier runs
    nowhere today.
12. `kitchen/` appears exactly once in the specification, has no rules and
    no adoption in any repository.
13. This repository gitignores `Cargo.lock`, which Appendix A requires to
    be committed for a workspace. A `gitignored` rule kind would catch it;
    a path-existence check does not.

## Part 9 — Open questions

1. **Suppression mechanism.** Needed before the v0.1 dogfood gate can be
   satisfied as written. Candidates: `config.folio.allow`, a baseline
   file, or accepting that an exception is a tracker ticket with no
   in-repository expression.
2. **`deno` versus `ts`, `c` versus `cpp`** as ecosystem names.
3. **Deploy environment names.** `staging` is now both a build type and
   the deploy target in the grammar's worked example. One should move.
4. **Variant filtering.** Deferred deliberately: folio knows only
   `default` and `all`. Include and exclude lists land in `config.folio`
   at v0.4, designed against a real Android repository rather than
   speculatively.

## Part 10 — Implementation stack

Verified against crates.io on 2026-09-06. Release dates and download
volumes were checked rather than assumed; three prior choices did not
survive that check and are corrected here.

### Selected crates

| Concern | Crate | Notes |
| --- | --- | --- |
| CLI parsing | `clap` | already a workspace dependency |
| TOML | `toml_edit` | format-preserving, which `folio init` needs to write into an existing manifest without destroying comments or ordering |
| YAML | `yaml-rust2` | see correction 2 |
| JSON | `serde_json` | already a workspace dependency |
| JSON Schema | `jsonschema` | see correction 1 |
| Templates | `handlebars` | 6.4.4, released 2026-08-12; actively maintained, matches the existing plan |
| Task shell | `deno_task_shell` | 0.33.3, released 2026-08-04, maintained by the Deno team |
| Repository walking | `ignore` | gitignore-aware, from ripgrep |
| Globs | `globset` | same origin, same semantics |
| Regex | `regex` | linear-time by construction; required, see below |
| HTTP | `ureq` | see the synchronous constraint below |
| Checksums | `sha2` | |
| Archives | `tar` + `flate2`, `zip` | |
| Grammar | `tree-sitter` | native bindings rather than the WebAssembly build |
| MCP (v0.6) | `rmcp` | feature-gated; the only sanctioned tokio usage |
| LSP (v0.8) | `lsp-server` | see correction 3 |
| SARIF | `serde-sarif` | when SARIF output lands |
| Errors | `thiserror` in libraries, `anyhow` in `repofolio` | |
| CLI acceptance tests | `snapbox`, `trycmd`, `assert_cmd` | same set `git-std/spec` already uses |

### Correction 1 — `jsonschema`, not `boon`

`boon` was chosen earlier for having fewer transitive dependencies. The
evidence does not support it: `boon` 0.6.1 was last released 2025-01-07
with 174,000 recent downloads, while `jsonschema` 0.54.0 was released
2026-09-05 with 20 million. A validator enforcing the manifest contract
should not be twenty months stale, and dependency count does not outweigh
that.

### Correction 2 — `yaml-rust2`, not any `serde_yaml` fork

Every fork of `serde_yaml` is stale or contested: `serde_yaml` itself is
author-deprecated (2024-03-25), `serde_yaml_ng` last released 2024-05-26,
`serde_norway` 2024-12-21, and `serde_yml` is a 0.0.x fork with a
contested history. `yaml-rust2` 0.12.0 was released 2026-08-18 and is
actively maintained.

The reason this is not a compromise: **folio does not need serde
integration for YAML.** The validation pipeline is
`YAML -> serde_json::Value -> jsonschema`. No type ever derives
`Deserialize` from YAML, so an abstract syntax tree converted to `Value`
is sufficient, and the dead lineage is avoided entirely.

### Correction 3 — `lsp-server`, not `tower-lsp` (v0.8)

The roadmap names `tower-lsp` for the editor milestone. Its last release
was 2023-08-11. `lsp-server`, maintained as part of rust-analyzer, was
released 2026-07-16 and is **synchronous**, which fits architecture
decision 2 rather than working against it. This also removes one reason to
reach for tokio outside `mcp`.

### Two constraints

**HTTP must be synchronous.** `folio tools` fetches release assets, and
`reqwest` pulls in tokio, which architecture decision 2 forbids outside
`mcp`. Use `ureq`.

**The regex engine must be linear-time.** Ecosystem definitions are
third-party content that may be fetched remotely, and content rules run
inside a pre-commit hook. A backtracking engine would allow a careless
pattern in someone's plugin to hang every commit. Rust's `regex` crate
makes that structurally impossible rather than a review problem.

### Assessment of the move from Deno

Stronger in most areas, equal in templates and HTTP, weaker in exactly
one: YAML. That weakness is contained, because YAML is one of three
accepted manifest formats rather than the backbone of the pipeline.

The clearest gains are `toml_edit` being format-preserving, `ignore`
providing gitignore-aware walking with no hand-rolled equivalent, a
linear-time regex engine, native `tree-sitter` bindings instead of a
WebAssembly build, and the `snapbox`/`trycmd` acceptance-test stack that
`git-std` already uses.

`deno_task_shell` is published as a Rust crate, so the one genuinely
Deno-shaped dependency carries over natively.

## Part 11 — Repository structure

**Decision: consolidate `driftsys/repofolio` and `driftsys/folio` into one
repository.** This reverses the split recorded in `folio-plan.md` (M0
item 1) and restores ADR 0003, `standard-and-cli-same-repo`, which is
still marked Accepted in `driftsys/repofolio/docs/adr/` and was never
updated when the plan superseded it.

The plan's technical objection was that a software release cadence does
not belong in a repository versioned by schema discriminator. That
dissolves on inspection: the standard's version discriminator is the
`$schema` URL, not the repository's `version` field. The repository
version is folio's semantic version and the schema version is
independent, so the two are orthogonal rather than in conflict.

Consequences:

- The open question in `CLAUDE.md` about pinning `repofolio`'s fixtures
  by submodule or vendoring script disappears. The fixtures are in the
  same repository.
- The bundled-schema parity check remains cross-repository, because
  `project/v1.json` lives in `driftsys/schemas`, which stays separate as
  a shared organisation-wide repository.
- ADR 0003 should be updated to record that it was superseded on
  2026-08-29 and reinstated on 2026-09-06, with this reasoning.

### Crate naming

Checked on crates.io, 2026-09-06:

| Name | Status |
| --- | --- |
| `folio` | Taken. Published 2018, all versions yanked. A yanked crate retains its name permanently. |
| `folio-cli` | **Taken 2026-09-05** by an unrelated project. |
| `folio-core` | **Taken 2026-03-24.** |
| `folio-manifest`, `-templates`, `-tools`, `-fmt`, `-release`, `-task` | Available. |
| `repofolio`, `repofolio-cli`, `repofolio-core` | Available. |
| `driftsys` | Available. |

**Two of the seven workspace crate names are already taken, and they are
the two most important ones.** The `folio-*` prefix is therefore not
viable as a coherent published namespace. This is not adversarial:
"folio" is an ordinary English word, so the namespace is naturally
contested. "repofolio" is coined, which makes it both available and
durable.

Resolution, which converges with the single-repository decision above:

```text
repository:  driftsys/repofolio
crates:      repofolio, repofolio-core, repofolio-manifest, ...
binary:      folio
```

Crate name and binary name are independent, so a crate published as
`repofolio` still installs a binary named `folio`, in the same way
that `cargo install ripgrep` installs `rg`. This also makes ADR 0003's
warning — distinguish "Repofolio" the standard from "folio" the CLI —
structural rather than a matter of documentation discipline.

Two mitigations are already in the plan: `cargo install` is demoted to
unsupported convenience in favour of release assets, and the canonical
install path is a checksummed per-platform asset.

**Applied 2026-09-06 in commit 441005c.** Every crate moved to the
`repofolio-*` namespace, and the front-end crate took the bare name
`repofolio` rather than `repofolio-cli`, so that the name users install is
the headline one and no empty umbrella crate has to be published to hold
it.
