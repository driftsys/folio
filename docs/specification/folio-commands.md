# folio command surface — specification

_Status: **forward-looking**. This document specifies the command surface for
folio v0.2 through v0.5. Only `folio check` and `folio registry` are
implemented today; each has its own as-built specification alongside this one.
The reviewed behavior below is agreed but not yet built; remaining schema
and flag-combination details are identified at the end._

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

_Revised 2026-09-12 after command-surface review. These forward-looking
semantics supersede the archived design where they differ; they do not claim
that the current CLI implements them._

## Command resolution

folio implements native work verbs (`fmt`, `fix`, `lint`, `check`) and
management commands (`init`, `add`, `tools`, `doctor`, `version`, and others
listed in the roadmap). Ecosystems also provide standard recipes such as
`build`, `test`, and `bench`. Repositories can override or extend built-in
behavior, including native commands.

- `folio <name>` selects a repository recipe first, falling back to the
  built-in command or ecosystem recipe when no override exists.
- `folio run <name>` explicitly invokes recipe resolution.
- `folio run folio::<name>` always selects the unshadowable built-in.
  `folio::` covers the full implicit tree, including qualified ecosystem
  operations such as `folio::rust::build`.
- Custom recipes remain directly callable, for example `folio deploy`, even
  when their names are outside the standard command vocabulary.

A recipe calling its own unqualified command recurses. Cycles must produce
an actionable error naming the cycle; extensions call `folio::` explicitly.

## Governing principles

1. **Purpose defines the verb; duration is a performance target.** `check`
   validates repository structure, configuration, syntax, and types. `lint`
   performs additional quality and policy analysis. Neither is classified
   by a fixed number of seconds.
2. **Preserve ecosystem defaults.** Bare `test`, `bench`, and `build` use
   the ecosystem's configured commands and settings, with project overrides.
   Suite, target, and profile selection are explicit refinements.
3. **`all` is comprehensive local verification, also usable in CI.** It
   includes audits and all declared test and benchmark targets. It may need
   network access; unavailable targets warn and skip rather than fail.
4. **Configuration owns resources and scheduling.** Ecosystems define
   defaults; projects customize them under `config.folio`. Folio does not
   infer a build profile, backend, or signing identity from a target name.
5. **The core layer is an ecosystem.** `repofolio` is always active, and
   `repofolio::check` supplies repository conformance through the same rule
   mechanism as other ecosystems.

## Standard operations

| Operation | Purpose | Example ecosystem mapping |
| --- | --- | --- |
| `fmt` | Apply formatting | `cargo fmt`, `deno fmt`, prim for its supported files |
| `fix` | Apply supported corrections | `cargo fix`, `clippy --fix`, `deno lint --fix` |
| `check` | Baseline validity and sanity checks | Repository conformance plus `cargo check`, `deno check` |
| `lint` | Additional quality and policy analysis | `cargo clippy`, `deno lint`, `clang-tidy` |
| `build` | Produce build outputs | `cargo build`, configured C/C++ build command |
| `test` | Run ecosystem-default tests, or selected suites and targets | Cargo, Deno, Gradle, Atest, CTest |
| `bench` | Measure performance | `cargo bench`, `deno bench`, Android benchmark harnesses |
| `audit` | Run configured security and dependency-policy checks | Vulnerability and license checks |
| `pack` | Produce distributable artifacts | Cross-compilation, app bundles, package archives |
| `sign` | Sign artifacts and attestations | Configured signing tool |
| `publish` | Make packages or release downloads available | Package registries and release assets |
| `all` | Run all configured verification | See below |

`spec` and `device` are not separate standard commands: use test suites and
named test targets. `bench` stays separate from `test`; there is no
`test --bench`. `deploy` is not a standard operation but can be a custom
repository recipe. Commands shown here are semantic mappings, not literal
PATH-based invocations: folio resolves managed tools by absolute path.

### Sanity checks and hooks

`check` asks whether the repository and code are valid. It validates structure,
configuration, syntax, and types without intentionally changing source files.
It should be suitable for frequent use, but its runtime depends on repository
size, build state, and the ecosystem. For Rust it includes `cargo check`.
This is the future delegated behavior; v0.1 remains structure-only as described
in [folio-check.md](folio-check.md).

Default moment recipes:

```makefile
pre-commit: fmt
pre-push:   check
```

CI independently runs formatting verification, `check`, `lint`, tests, builds,
audits, and other configured verification, because local hooks can be skipped.

### Full verification

`folio all` verifies formatting without rewriting it and runs `check`, `lint`,
`build`, `audit`, `test --all`, `bench --all`, and other configured verifications
such as secret scanning, license policy, and generated-file consistency.
Formatting differences fail verification. Normal build outputs and caches are
allowed; this operation does not apply source fixes. Packaging, signing,
publishing, and custom deployment actions are not part of verification.

`all` is an aggregate verification operation, not a fail-on-first-error recipe
containing `fmt`. It continues independent work after a failure, while steps
whose prerequisites failed are reported as blocked. `--fail-fast` stops
subsequent work after the first failure. Target cleanup still runs.

Every available declared test/benchmark target is exercised. Unavailable targets
are warned about and skipped without failing the aggregate. This supersedes
the earlier offline-only definition and hardware exclusion. Missing audit tools
or failed audit/network operations are verification/infrastructure failures,
not target skips. Reports distinguish successful work from work not performed.

### Listing

No `default` recipe is shipped in the `folio:core` managed section. Bare
`folio run` lists top-level recipes and exits 0 unless a repository explicitly
defines a default. Qualified recipes are shown behind a listing flag.
`folio test --list` and `folio bench --list` list their configured suites,
targets, defaults, compatibility, and resource requirements without running
verification or provisioning targets.

## Tests, benchmarks, and targets

### Defaults and suite selection

Bare `folio test` fans out to each active ecosystem's default test command,
preserving discovery, exclusions, and configured settings. Projects may
override or extend these defaults. This is not a universal host-only guarantee:
although host execution is common, ecosystem/project defaults decide.
Native coverage such as Cargo documentation tests must not be dropped merely
because it does not have a dedicated standard suite flag.

The standard suite vocabulary is mandatory to recognize, not mandatory to
implement in every project:

| Selector | Meaning |
| --- | --- |
| `--unit` | Isolated component tests |
| `--spec` | Requirements and behavioral specifications, including integration and acceptance scenarios |
| `--smoke` | Quick checks of essential functionality |
| `--suite <name>` | A project/ecosystem-defined suite |

There is no separate standard `integration` suite: those tests map to `spec`.
Multiple suite selectors combine: `folio test --unit --spec` runs both suites,
each on its configured default target unless target selection is overridden.
Repeated selectors do not execute the same suite twice.
Each suite can declare a default target and compatible targets. Selecting a
suite uses its configured default target unless explicitly overridden. Bare
`test` still preserves the ecosystem's native default command; it is not
implicitly rewritten as a fixed list of suite selectors.

```sh
folio test
folio test --unit
folio test --spec --target staging
folio rust::test --spec
folio test --suite regression --target phone-staging
folio test --all
```

### Named targets

A test target is a named setup selecting the system under test, runner,
settings, and supported suites. It can describe a host, emulator, physical
device, or server environment. The test process itself may run on the host
while exercising a remote service. This is distinct from a compilation target
triple and from the build profile selected by `--profile`.

Ecosystems define commands, suite mappings, supported target kinds, scheduling,
and resource constraints. Project settings under `config.folio` provide
concrete targets and override ecosystem defaults. Repository recipes can
replace or extend the resulting behavior. Configuration does not add new
top-level project manifest fields.

Configuration ownership is explicit: ecosystem definitions supply Folio's
defaults for suites, targets, profiles, and orchestration; projects customize
those defaults under `config.folio`. Native tool settings remain in native
configuration files such as `Cargo.toml`, `deno.json`, and
`CMakePresets.json`. Folio configuration selects and orchestrates native
capabilities rather than duplicating their configuration schemas.

Project overrides resolve suites, targets, and profiles by name. A new name
adds an entry; an existing name overrides only supplied fields and inherits
the remaining fields. Lists replace inherited lists rather than implicitly
appending. Removing an inherited entry requires explicitly disabling it;
omission preserves inheritance. For example, overriding the `spec` suite's
default target to `staging` retains its ecosystem-provided test command.
The exact configuration field syntax remains to be specified.

For example, a project may map Rust `spec` to its integration-test executables,
Deno `spec` to acceptance tests against staging, and Android `spec` to
instrumentation on an emulator. Android native tests can use a configured
native harness; AOSP can select host/device test modules through Atest rather
than treating Soong itself as the test runner. These are mappings, not
requirements to classify every instrumented test as a whole-system test.

`folio test --spec` fans out to the ecosystems defining that suite, each on its
default target. `folio rust::test --spec` narrows execution to Rust. A target
override must be compatible with each selected suite; it never silently falls
back to another target. An ecosystem without the requested aggregate suite is
visibly skipped; an explicitly qualified undefined operation is an error.

Target names resolve separately within each selected ecosystem. For example,
`folio test --spec --target staging` uses Rust's configured `staging` target
for Rust's `spec` suite and Deno's configured `staging` target for Deno's
`spec` suite; their runners may differ. A selected ecosystem that defines the
suite but lacks the requested target produces a configuration error, not a
fallback or an unavailable-target warning.

`--all` runs all **declared compatible suite–target combinations**, not the
Cartesian product of every profile, suite, and device. It preserves native
default test coverage as well when no suite selector narrows the run. An
explicit suite selector narrows `--all` to that suite: `folio test --spec`
runs `spec` on its default target, while `folio test --spec --all` runs it
on every declared compatible target. Unavailable targets still warn and skip
under this suite-scoped `--all`.

Ecosystems declare which suites and native test coverage each command covers.
Folio uses those relationships to plan `--all` without automatically running
both the default command and overlapping suite commands. Coverage such as
Rust documentation tests remains included where applicable to the selection.
Coverage relationships apply within the selected target and configuration;
running tests on one target does not cover the same suite on another target.

An explicit target also narrows `--all`: `folio test --all --target staging`
runs every suite compatible with `staging`, on that target only. Explicit
suite and target selectors both constrain the selection when combined.
Because the target was explicitly requested, its unavailability fails even
with `--all`; the aggregate warning-and-skip policy does not override an
explicit target request.

`--target` is repeatable. For example,
`folio test --spec --target host --target staging` runs `spec` on both
targets, once per target. Repeated target names do not duplicate execution.
Each requested target is explicit: an unavailable target or an incompatible
explicit suite–target selection counts as a failure rather than a skip.

Undefined explicit suites/targets and incompatible
explicit selections fail clearly. If no ecosystem defines an explicitly requested
suite, that is an error rather than a successful empty run. A configured but unavailable target warns
and skips under aggregate `--all`; explicitly selecting that target fails.
An explicitly invoked aggregate with no applicable operations fails rather
than claiming successful verification. Within `all`, unsupported ecosystem
operations are visibly skipped. If declared targets are all unavailable, report all as skipped
with warnings, preserving the non-failing unavailable-target policy.

Bare `folio test` preserves the native runner's behavior when it discovers
zero tests. An explicitly selected suite that discovers zero tests fails with
a clear diagnostic. This is distinct from an undefined suite or a target
skipped because it is unavailable.

### Benchmarks

Bare `folio bench` likewise preserves ecosystem-default benchmark behavior.
It shares named targets and lifecycle/reporting infrastructure with `test`,
but remains a separate command:

```sh
folio bench
folio bench --target phone-staging
folio bench --all
folio bench --list
```

Targets declare support for tests, benchmarks, or both. Reports preserve
benchmark measurements, not just pass/fail; execution errors or configured
performance-threshold violations fail the operation.

### Execution and lifecycle

On Ctrl+C, Folio stops scheduling new work, requests termination of running
processes, and attempts cleanup within its configured timeout. It writes
partial reports marking interrupted work as cancelled and exits nonzero.
A second Ctrl+C forces immediate termination without waiting for running
work, cleanup, or report writing. Cleanup and complete partial reports are
not guaranteed after this forced exit.

Folio validates the complete requested selection before starting target
preparation or test/benchmark execution. Unknown suite or target names and
incompatible explicit selections fail this validation without starting any
selected work. For example, requesting `host` and misspelled `stagin` reports
the unknown target before running `host`. Runtime failures retain the
continuation and `--fail-fast` behavior below.

`all`, `test --all`, `bench --all`, and test/benchmark runs with multiple
explicit suite or target selections continue independent work by default and
support `--fail-fast`. For example, `folio test --unit --spec` still runs
`spec` if `unit` fails, and the final command fails; `--fail-fast` stops
new work from being scheduled after that failure. Already-running tests and
benchmarks finish and report their results; `--fail-fast` does not cancel
them. Cleanup still runs after active work finishes. Individual recipes retain
dependency ordering and stop on failure. Ecosystems configure scheduling, parallelism, and resource
constraints; project configuration can override those defaults. Folio follows
that configuration rather than imposing a universal concurrency default.

Folio's fail-fast guarantee applies to scheduling Folio operations.
Already-running native commands finish according to their own settings;
Folio does not imply control over their internal test scheduling. Finer
runner-level control requires an explicit adapter capability.

Targets can define preparation and cleanup, such as starting a test server or
emulator. Cleanup runs after failures and fail-fast completion, releasing only
resources created by that run, not pre-existing devices or services. Failure to
prepare an otherwise selected target or to clean it up is an execution failure,
not a passing test result.

Ecosystems supply timeout defaults for target preparation, test/benchmark
execution, and cleanup; projects can override those defaults. A timeout
counts as an execution failure. Cleanup is still attempted after preparation
or execution times out, subject to its own timeout and the resource-ownership
boundary above.

Retry behavior below is specified now but delivered in a second implementation
iteration. The first iteration implements selection, targets, lifecycle,
timeouts, cancellation, and reporting without adding Folio retries. It rejects
explicit Folio retry settings as unsupported rather than silently ignoring
them, while preserving the native runner's existing behavior. The second
iteration adds retry configuration, native delegation, attempt reporting,
strict retry policy, and retry-aware fail-fast.

Automatic retries are disabled by default. Ecosystems or projects may
explicitly configure retries. Automatic retries apply only to the failed test
operation; preparation and cleanup failures are not automatically retried.
Retry granularity follows the underlying runner's capabilities: a retry may
rerun one failed test or an entire suite. Reports explicitly identify what
was rerun and must not describe a suite rerun as an individual-test retry.
When the underlying runner already handles retries, Folio delegates to that
mechanism rather than adding another retry loop. Reports expose attempt
details when the runner provides them; missing native attempt details must
not be invented.
Strict retry mode requires reliable information about whether retries occurred.
If the native runner hides that information, Folio rejects strict retry mode
with a clear explanation rather than claiming to enforce it.
Reports retain every observable attempt and disclose when the native runner
does not expose attempt history. A known pass after retry remains
distinguishable from a known first-attempt pass; missing history is not
evidence of a first-attempt pass.
A test that passes after retrying succeeds by default and is visibly reported
as "passed after retry". Projects may enable a strict policy that counts such
a result as a failure.
With retries configured, `--fail-fast` waits for the retry sequence to finish
before deciding whether that operation triggers a stop to new scheduling.
The decision uses the final result after applying the project's strict retry
policy; an intermediate failed attempt alone does not trigger fail-fast.

Availability checks determine whether a target can be used or prepared, not
just whether it is already running. A stopped emulator that the configured
preparation can start is available; a required physical phone that is not
connected is unavailable.

Target unavailability and preparation failure are distinct. Under `--all`,
a disconnected Android phone can produce a warning and skip; if the phone
is available but installing the test application fails, the run records an
execution failure. Preparation failures must not be reclassified as
unavailable-target skips. Explicitly requested targets retain their stricter
availability requirement.

### Tool argument forwarding

Arguments after `--` go to the underlying tool only when an ecosystem is
explicitly selected:

```sh
folio rust::test -- --release
folio deno::test -- --filter smoke
```

Unqualified multi-ecosystem operations reject tool-specific trailing arguments
rather than forwarding one tool's options to unrelated tools.

## Output and reports

Follow [Command Line Interface Guidelines](https://clig.dev/) for a polished,
human-readable default, actionable errors, discovery, and scriptable output.
Piping disables terminal decoration without silently selecting another format.

- `--format json` provides structured results for automation and agents.
- `--format tap` provides TAP for verification results. It is not a generic
  encoding for plans, registries, or detailed benchmark measurements.
- Progress and tool logs go to stderr, leaving structured stdout parseable.
- Formats share outcomes and exit semantics. Results identify the ecosystem,
  suite where applicable, target, status, and paths to underlying reports.
  Execution failures also produce structured results when JSON is selected.
- Where an upstream tool exposes only a suite-level result, report that
  granularity honestly instead of fabricating individual test results.

JUnit XML export is prioritized over TAP for CI test integration. Report files
are independent of console presentation:

```sh
folio test --report junit:results.xml
folio test --format json --report junit:results.xml
```

Except after forced termination such as a second Ctrl+C, reports are written
on failed runs too, preserving completed results and
identifying skipped and blocked suites. Failed or interrupted work must not be
marked as passed. Benchmark measurements remain available in the detailed/JSON
report even when a verification summary is rendered in another format.

Actual verification failures result in a nonzero exit. Unavailable targets in
aggregate runs are visible warnings/skips without causing failure; unavailable
explicitly requested targets and configuration/infrastructure errors fail.

[GitLab consumes JUnit XML](https://docs.gitlab.com/ci/testing/unit_test_reports/)
natively. GitHub Actions can publish JUnit or TAP using a reporting action such
as [test-summary/action](https://github.com/test-summary/action). Emitting TAP
in a console log alone does not publish a CI test summary.

## Execution previews and effect plans

A recipe dry run previews task/command order, working directories, and selected
profiles. Arbitrary shell commands cannot promise an exact account of their
future effects merely by printing command strings. Label this an execution
preview, not an effect plan.

Built-in mutations, including `init`, `fix`, and `version bump`, must provide
plans describing their intended effects under the plan/apply contract. A recipe
wrapping such a command can use its plan support; the wrapper does not acquire
an exact-effect guarantee for arbitrary additional shell steps.

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

### Override and pre/post extension

A repository recipe wins over built-in behavior. Calling the built-in explicitly
allows pre-steps and post-steps without separate hook syntax:

```makefile
build: prepare folio::build
    ./scripts/package.sh

prepare:
    ./scripts/generate.sh
```

Dependencies finish left to right, then the body runs: generate, built-in build,
package. A failure stops subsequent dependent steps. Omitting `folio::build`
replaces the built-in entirely. Cleanup for test targets is a lifecycle
responsibility, not an ordinary post-step that disappears on failure.

## Build profiles

Use ecosystem defaults rather than imposing debug/staging/release build types
or deriving an unchangeable profile from the verb. Ecosystems and repositories
can define named profiles, selected explicitly:

```sh
folio build
folio build --profile staging
folio test --profile release
folio pack --profile staging
```

Undefined or unsupported profiles fail clearly. A staging profile can mean an
optimized build with symbols in one project; it is not required everywhere.
Signing authorization and deployment/backend settings are separate policy;
folio must not derive credentials or a destination from a profile's name.

## Hooks

Two implicit hook recipes:

```makefile
folio::pre-commit:  fmt
folio::pre-push:    check
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

The pre-push sanity check includes ecosystem validity checks such as
`cargo check`. No fixed runtime is promised. Repository recipes can override
or extend either hook through `folio::`; CI independently repeats verification.

Note: `pre-push.off` is present in both `git-std` and `repofolio` today, so
that tier currently runs nowhere. Enabling it is a behavioural change.

**PATH rule.** folio is the only binary required on `PATH`. It resolves
every pinned tool internally by absolute path. Without this, hooks — fired
by git with a minimal environment — would either embed cache paths that
break on every version bump, or silently violate architecture decision 7.

## Version and release path

```sh
folio --version                  # Folio executable version
folio version                    # Current project version
folio version bump --dry-run     # Effect plan for a proposed bump
folio version bump               # Apply through the plan/apply workflow
```

These describe built-in behavior; recipe overrides follow the resolution rule.
`repofolio-release` delegates calculation and mutation to pinned git-std, and
verifies actual effects against the approved plan.

```text
pack     produce distributable artifacts, SBOM, and provenance
sign     sign artifacts and attestations
publish  upload packages or release downloads
```

Package-registry publication and binary-release publication are distinct
channels. `pack` is not an obligatory predecessor of source-package publication;
binary publication consumes the relevant packaged artifacts and signatures.
SBOM/provenance generation belongs with artifact production; signing then signs
those outputs. `publish` remains subject to the effect-plan contract.

There is no standard `deploy` command. Repositories may declare a custom
`deploy` recipe to install or activate software in an environment, and call it
as `folio deploy`. Test-target preparation remains part of the test lifecycle.

The project's category informs which publication channels apply: libraries
may publish source packages, tools may publish both packages and release
binaries, and applications/services may publish distributable artifacts.
Activation of an application/service is repository-specific deployment work.

## Ecosystems and plugins

### Built-in rollout and first-party plugins

Roll out built-in ecosystems in this order:

1. **Rust / Cargo** (`rust`).
2. **Deno / TypeScript** (`deno`).
3. **C/C++** (`c`), using the baseline below.

**Gradle-based JVM support is a first-party plugin**, supporting Kotlin and
Java with a project-selected JDK. Gradle handles builds, dependency resolution,
tests, and publishing; Maven-compatible repositories supply dependencies and
receive publications. This does not require the Maven build tool (`mvn`);
its support is deferred. Gradle is the first planned JVM integration, not a
fourth built-in ecosystem. Plugin installation is optional, while the command,
verification, reporting, and policy contracts are the same as for built-ins.
Android (AGP, SDK/NDK), AOSP (Soong/Atest), and Unity remain specialized plugins.

This is a rollout decision, not a claim that these integrations are implemented.
Within each applicable milestone, Rust precedes Deno, followed by C/C++;
plugin releases can follow their own schedule.

### C/C++ baseline

- **CMake with presets**, **Ninja** for new scaffolds, and **CTest** for test
  execution. Existing generators and platform toolchains remain respected.
- **Conan 2 first**, optional for projects needing external dependency
  management. Conan owns library dependency records; `.folio.lock` pins tools.
  Projects using vendored dependencies or platform SDKs need not adopt Conan.
  vcpkg integration is deferred.
- **Clang and GCC** are supported compiler families. The project's pinned
  toolchain selects the compiler, linker, and runtime; neither compiler is
  universally required and folio does not replace vendor toolchains.
- **clang-format** and **clang-tidy** provide formatting and linting, including
  GCC-built projects where compatible. Sanitizer test profiles are optional:
  AddressSanitizer/UndefinedBehaviorSanitizer where supported, ThreadSanitizer
  separately on compatible toolchains and targets.
- New test scaffolds use **GoogleTest for C++** and **Unity for C** (the C test
  framework, not the game engine), registered with CTest. Existing projects
  retain their framework. New C++ benchmark scaffolds use **Google Benchmark**
  behind the separate `bench` operation.
- C/C++ language versions are explicit project choices constrained by the
  selected compiler and target SDK. No universal newest-language requirement
  is imposed.

All baseline tools above are open source; commercial vendor compiler support
is a possible extension, not a baseline requirement. Safety/cybersecurity
policy is additional verification, not a certification implied by this stack.

### Ecosystem representation

An ecosystem is a data directory containing no code:

```text
ecosystems/<name>/
├── folio.ecosystem.toml
├── templates/
└── sections/
```

Ecosystem definitions select supported runner adapters while remaining
data-only. Adapters handle native argument mapping, result interpretation,
and capability reporting. An unfamiliar runner can use a generic command
adapter that reports an honest command-level result; it must not fabricate
test-case results or claim unsupported capabilities. The adapter interface
and configuration field syntax remain implementation-design work.

```toml
"$schema" = "https://driftsys.github.io/schemas/folio-ecosystem/v1.json"

[ecosystem]
name = "unity"
version = 1

[markers]
must   = ["ProjectSettings/ProjectVersion.txt", "Assets"]
should = [".editorconfig"]

[commands]
build = "unity -batchmode -quit -projectPath ."
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
spelling is not warned about the other. `commands` selects the ecosystem-default
command per verb. Optional named
profiles and test suite/target mappings refine that selection; no mandatory
build-type level is imposed. The concrete schema for those refinements must be
specified before the configuration loader is implemented.

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
   `**/*.rs` would defeat bounded conformance checking. Ecosystem compilation/type
   checks have their own workload; neither gets a fixed runtime guarantee.
2. **Use Rust's `regex` crate, not a backtracking engine.** Ecosystem
   definitions can be fetched from third parties, and a pattern with
   catastrophic backtracking would be a denial of service against a
   verification run. Linear-time matching makes that structurally
   impossible rather than a review problem.

**Code namespacing.** Plugins cannot mint `FOLIO-` codes, since that
registry freezes at v1.0. Codes are namespaced by ecosystem name —
`RUST-001`, `UNITY-001` — which needs no central registry and cannot
collide.

### C/C++ build-system selection

CMake is the first supported build-system integration for `c`. The existing
standard appendix also describes Meson, Bazel, and xmake markers, but folio
support for those build systems is deferred until a concrete project needs it.
Recognizing a marker must not imply a working command adapter.

Keep language-level rules separate from build-system commands so later adapters
can reuse the same C/C++ policy. Android and AOSP keep their platform entry
points rather than being forced through the general CMake adapter.

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
6. **`directory-layout.md`** — remove `assemble` and separate standard
   `spec`/`device`/`deploy` operations; add `bench`, `audit`, `pack`, `sign`,
   `publish`, `all`. Specify suites under `test` and named targets for tests
   and benchmarks. Define `check` as conformance plus ecosystem validity.
7. **`directory-layout.md`** — add `examples/` to Appendix A. It is one of
   Cargo's five target types and is already compiled by `cargo test`, so
   it needs no recipe, only a layout entry.
8. **Appendix E** — write the Deno ecosystem appendix. It does not exist,
   so a built-in `deno` ecosystem currently has no marker table.
9. **`ci-cd.md`** — default pre-commit to formatting and pre-push to sanity
   checks. CI/full local verification includes formatting verification,
   lint, checks, builds, all tests/benchmarks, audits, and configured codegen
   drift checks. Specify aggregate continuation, skips, and failed-run reports.
10. **Resolve two naming inconsistencies** with grammar 7.3: `deno` versus
    `ts`, and `c` versus `cpp`.
11. **Grammar execution/resolution** — repository recipes take precedence;
    `folio::` is unshadowable and cycles are errors. Distinguish ordinary
    fail-on-error recipe dependencies from aggregate verification continuation
    and unconditional target cleanup.
12. **Ecosystem configuration schema** — capture defaults, suite/target
    compatibility, profiles, lifecycle, scheduling, and project overrides under
    `config.folio`. The behavior above is agreed; the exact field schema and
    flag-combination grammar remain implementation-design work.
