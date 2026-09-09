# folio command surface — specification

_Status: **forward-looking**. This document specifies the command surface for
folio v0.2 through v0.5. Only `folio check` and `folio registry` are
implemented today; each has its own as-built specification alongside this one.
Everything else here is designed and settled but not yet built._

_Promoted 2026-09-09 from
`docs/archive/specs/2026-09-06-folio-task-model-design.md`, which remains the
raw original. That document's remaining parts were gardened elsewhere: the
implementation stack into `docs/decisions/0003`, repository structure into
`docs/decisions/0001` and `0002`, and the catalogue of defects found in the
Repofolio standard into `docs/technotes/`._

_Why this is a specification rather than working memory: the design is
settled, and four milestones depend on it. Leaving it in `docs/wip/` would
either block the working-memory gate on every branch or hide the design in an
archive directory whose name says the work is finished. The lifecycle rule
keeps **unfinished** work out of shipped docs; this is finished design of
unbuilt work, and it is labelled as such._

## Two surfaces

folio implements four work verbs natively: `fmt`, `fix`, `lint`, `check`.
Everything else is a **recipe** that a repository declares and that folio
dispatches to an ecosystem's command. The two surfaces are addressed
differently and must not be conflated:

- `folio <verb>` runs a native verb when one exists, and otherwise falls
  through to `folio run <name>`.
- `folio run <recipe>` always means the recipe.

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

## Verbs

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

## Addressing

```text
build                            all ecosystems (fan-out)
rust::build                      one ecosystem
rust::repofolio-manifest::build  one module
rust::*::build                   glob at segment level
folio::build                     the unshadowable built-in
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

## Build types

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

## Hooks

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

## Release path

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

## Ecosystems and plugins

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

Markers give both detection and the FOLIO-101/102 conformance rules. A
marker entry is normally a bare path string. Nesting a list of strings in
its place declares a group of acceptable spellings, satisfied when any
one of them is present — for example, `rust`'s recommended markers are

```toml
[markers]
should = ["rust-toolchain.toml", ["rustfmt.toml", ".rustfmt.toml"]]
```

which reports one `FOLIO-102` finding naming both spellings if neither
`rustfmt.toml` nor `.rustfmt.toml` is present, not one finding per
spelling — a repository satisfying the recommendation through either
spelling is not warned about the other. `commands` is keyed
`verb → build_type → command`.

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

## Required changes to `driftsys/repofolio`

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
