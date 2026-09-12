# folio — roadmap (Rust era)

_Drafted 2026-08-29. **Supersedes** the Deno-era `folio-roadmap.md`.
Rev. 2 same day: aligned to the binary-orchestration decision — folio invokes
pinned tool binaries via CLI + JSON (no crate linking); `tools`/`doctor` land
at v0.2; upskill joins as the fourth orchestrated binary at v0.6._
_Companions: `folio-plan.md` (start plan + gap analysis, incl. the Integration
architecture decisions), `claude/git-std-backlog.md`, `claude/prim-backlog.md`,
`context/folio-architecture-decisions.md` (D1–D6), `REPOFOLIO_SPEC.md`._

**Renumbering vs the old roadmap:** v0.1 narrows to `check` + `init` (fmt moves
to v0.2 because it requires prim binaries and a stable CLI contract);
the task runner slides v0.2 → v0.3. Everything later shifts accordingly.
Rationale: v0.1 must have **zero upstream dependencies** so folio ships and
starts dogfooding immediately.

**Versioning of folio itself:** semver, pre-1.0 downshift per git-std's rule;
`project.toml` `category: tool`. Each minor below is a shippable release with a
theme, not a sprint.

---

## Command-surface revision — 2026-09-12

[The command specification](../specification/folio-commands.md) records the
agreed semantics for recipe overrides, test suites/targets, profiles,
verification, and reports. These are future behavior; only `check` and
`registry` are currently implemented. The completed check slice is not the
whole v0.1 scope: `init` and `add` remain part of that release.

- Repository recipes override built-ins and can extend them before/after
  through unshadowable `folio::` calls; cycles are errors.
- `check` is sanity/validity checking, including ecosystem type checks once
  delegation lands. Defaults: pre-commit `fmt`; pre-push `check`; CI repeats
  verification independently. Fixed runtime promises are removed.
- `test` and `bench` preserve ecosystem defaults, with project overrides.
  `test` offers `--unit`, `--spec` (including integration), `--smoke`, and
  custom `--suite`; both commands offer named `--target`, `--all`, `--list`,
  and aggregate `--fail-fast`. Separate `spec`/`device` commands are removed.
- `all` verifies formatting without changing it and includes checks, lint,
  builds, audits, other configured verification, `test --all`, and
  `bench --all`. It may use network/hardware: unavailable targets warn and
  skip; actual failures fail. Independent work continues unless fail-fast.
- Ecosystems configure target scheduling and lifecycle; projects override
  defaults under `config.folio`. Cleanup runs after failures and releases
  only resources created by the run. Optional `--profile` selects named
  profiles; there is no mandatory debug/staging/release ladder.
- Human-readable output is the default, JSON supports automation, TAP supports
  verification, and JUnit report export takes priority for CI integration.
  Reports are written on failed runs as well. Structured stdout stays clean.
- Recipe dry runs are execution previews; built-in mutations require effect
  plans. `version` reads the project version; `version bump` mutates it.
  `publish` covers package and release-asset publication; `deploy` remains a
  possible custom recipe, not a standard command.

The schema for suite/target configuration and detailed flag combinations needs
an implementation design before code; this review settles behavior rather than
inventing schema fields. Broader test/report features belong with runner and
ecosystem work, not the already implemented v0.1 check slice.

## Ecosystem rollout

Built-ins are developed in order: **Rust/Cargo → Deno/TypeScript → C/C++**.
The initial conformance/scaffolding scope starts with Rust, then Deno; C/C++
breadth follows at v0.4. This order applies within the relevant milestones,
not a requirement to finish every future Rust feature before any Deno work.

**Gradle-based Kotlin/Java support is an optional first-party JVM plugin**,
using Maven-compatible repositories, not the Maven build tool. It is not a
fourth built-in; its release timing is separate from core milestones. Android,
AOSP, and Unity remain specialized plugins with the same command/reporting
contracts. See the [ecosystem baseline](../specification/folio-commands.md#built-in-rollout-and-first-party-plugins)
for the agreed C/C++ tools and plugin boundary.

## v0.1 — Conformance (the reason folio exists)

**Verbs:** `check` · `registry` · `init` · `add`
- `check`: manifest parse (toml/yaml/json) → bundled-schema validation →
  core-layer MUST/SHOULD/MAY rules → ecosystem-marker detection →
  conformance report with stable `FOLIO-xxx` codes, `--format json`,
  CI-gradable exit codes. Structure-only in v0.1; no tool spawning.
- `registry`: machine-readable diagnostic and ecosystem registry.
- `init` / `add <ecosystem>`: handlebars ecosystem folders
  (`folio.ecosystem.toml` + `templates/` + `sections/`), managed-section
  writer, idempotent re-runs, `.folio.lock`. Develop **rust** first, then
  **deno**, as the initial built-in ecosystems; generates `.githooks/*.hooks`, `.editorconfig`, CI skeletons.
- `bootstrap` glue: the environment half of the repo `bootstrap` flow.

**Upstream deps:** none. **Grammar deps:** none.
**Exit criteria (dogfood gate):** repofolio and git-std pass `folio check`, or
every failure is a ticketed exception; JSON output snapshot-tested; the tool
contract (codes · JSON · exit semantics) holds from this release onward.

## v0.2 — Tools & hygiene (installer + first delegation)

**Verbs:** `tools` · `doctor` · `fmt` · `lint` · `fix` (+ `bootstrap` re-shaped)
- **`folio tools`:** pins in `.folio.lock` (version + source + per-target
  sha256), fetch → verify → atomic land into
  `~/.cache/folio/tools/<name>/<version>/<target>/`, absolute-path invocation,
  `folio which <tool>`; source classes github-release / toolchain-managed /
  system-verify-only; TOFU checksums, cosign when signed. No shims, no PATH
  mutation, exact pins only.
- **`doctor`:** lock vs cache vs default toolset vs PATH-skew (standalone
  prim vs pinned prim). **`bootstrap`** shrinks upstream: repo script does
  git plumbing + installs folio; `folio bootstrap` ensures everything else —
  including git-std, then `git std hooks install`.
- **`fmt`/`lint`/`fix`:** `repofolio-fmt` spawns the pinned `prim` binary (batch
  file lists, one spawn per verb, JSON in/out); shell tier shfmt + shellcheck;
  aggregated report, upstream codes passed through.
- Delegated `check` adds ecosystem validity (for example `cargo check`) to
  repository conformance once the pinned-tool execution path exists.

**Upstream deps:** prim **F3 (P0 — prebuilt assets + checksums; folio cannot
ensure prim without them)**, **G1** (verb model), **A1** (hygiene contract),
**D2** (JSON/SARIF). git-std: nothing new (checksummed assets already ship).
**Exit criteria:** dprint-Wasm plan retired permanently; `folio fmt`
byte-identical with standalone pinned `prim fmt`; a clean-machine
`bootstrap` ends with every declared tool ensured and `doctor` green.

## v0.3 — Task runner (Foliofile)

**Verb:** `run`
- Own Rust parser for the runner (small ABNF); `tree-sitter-foliofile`
  proceeds separately as the editor/LSP grammar — two consumers, one normative
  grammar, both conformance-tested against the shared corpus.
- Execution via `deno_task_shell`; recipes, dependencies, ecosystem-qualified
  names (`rust::build`); repository recipe precedence, `folio::` extension,
  and cycle detection. No generated default: bare `run` lists recipes unless
  the repository explicitly defines a default.
- Standard test/benchmark dispatch follows ecosystem defaults and configured
  suites/targets; `all` aggregates verification using the revised contract
  above. Scheduling, cleanup, target skips, fail-fast, argument forwarding,
  structured results, and failed-run report export are runner acceptance work.
- Follow the reviewed [test selection and lifecycle contract](../specification/folio-commands.md#tests-benchmarks-and-targets):
  ecosystem-declared coverage avoids duplicate default/suite runs; explicitly
  selected empty suites fail; availability includes targets preparation can
  create; fail-fast governs Folio scheduling. Data-only ecosystem definitions
  select runner adapters with explicit capabilities and honest result detail.
- Deliver test execution in two iterations: first selection, targets,
  lifecycle, timeouts, cancellation, and reporting; then the already specified
  retry configuration, native delegation, attempt reporting, strict retry
  policy, and retry-aware fail-fast. The first iteration rejects explicit
  Folio retry settings as unsupported and preserves native runner behavior.
- Recipe dry runs preview execution; delegated effect plans remain distinct.
- Start the 30–50-file diagnostic corpus (first-class test artifact).

**Upstream deps:** none. **Gate:** the four original grammar review points
closed (U+2500 vs ASCII · `default` semantics location · parenthesized dependency
calls · managed-sections normative status), plus alignment on recipe precedence,
cycle detection, aggregate continuation, and target cleanup.
**Exit criteria:** Justfile retired in repofolio and git-std; corpus wired
into CI for both parser consumers; configured suite/target execution and
failed-run reporting verified.

## v0.4 — Ecosystem breadth + migrations

**Verbs:** `migrate` · `self-update`; `add` grows ecosystems
- Add the **C/C++ built-in** after Rust and Deno: CMake/presets, Ninja,
  CTest, optional Conan 2, Clang/GCC, clang-format/clang-tidy. Existing
  toolchains remain respected; language versions and sanitizer profiles are
  project-selected. Meson/Bazel/xmake and vcpkg are deferred.
- Plugin infrastructure supports optional first-party **Gradle JVM**
  (Kotlin/Java) and specialized **Android AGP**, **AOSP Soong**, and Unity
  integrations. Their release timing is tracked separately; they are not
  all required to ship with this milestone. Remote ecosystems use pinned
  git releases cached under `~/.cache/folio/ecosystems/`.
- Suite/target mappings and lifecycle requirements follow the shared test
  model rather than a separate `device` command. Safety-oriented policy
  builds on C/C++ support rather than naming the language ecosystem certified.
- `migrate`: spec-version migrations, diff-then-apply (plan→approve→apply
  applies to folio's own mutations too); `self-update` with the same
  install-method awareness git-std has.

**Upstream deps:** none.
**Exit criteria:** a fixture repo per ecosystem passes `check`; migration
golden tests old-layout → current.

## v0.5 — Version & hooks (second delegation: git-std)

**Verbs:** `version` · `hooks`
- `repofolio-release` drives the **pinned `git-std` binary**, single-version mode
  pinned by config folio writes/validates; folio owns the manifest's
  `version`/`versioning` fields, git-std computes.
- `folio version` reads the project version; `folio --version` identifies the
  executable. `folio version bump [--dry-run]` owns the mutation workflow.
- Choreography: `git std bump --dry-run --format json` → present `BumpPlan` →
  human approve → `git std bump` → **verify actual effects against the plan**
  (fidelity check; loud failure on divergence).
- `hooks` generates/validates files + shims; the git-fired runtime stays the
  `git-std` binary.

**Upstream deps:** git-std **A1 → C3 (stable CLI + JSON schemas) → B2 (plan
JSON + fidelity AC)**, with **A2** alongside. cosign/SBOM (git-std D1, prim
F2) targeted by here.
**Exit criteria:** release of folio itself is cut by `folio version bump`
(dogfood); polyglot fixture produces exactly one tag + one changelog; an
injected plan/apply divergence is caught by the fidelity check.

## v0.6 — Knowledge layer (the AI surface)

**Verbs:** `mcp` · `explain` (+ skills governance in `check`/`bootstrap`)
- `folio mcp` — the suite's **single** MCP server, guidance-only: repofolio
  rules + `FOLIO-` codes, `prim registry --format json` (H1′), `git std
  registry --format json` (G4), upskill's agent-layout map + skill inventory
  (U2), and the plan→approve→apply recipes per verb. All payloads are the
  tools' own JSON dumps — registries hold facts, MCP serves facts, skills
  hold procedure. Agent executes the CLIs; tokio/rmcp feature-gated here and
  nowhere else.
- `explain <code>` — human/agent explanation for any diagnostic code across
  the suite, from the same dumps.
- **Skills governance:** spec agent-layer paragraph (`.agents/skills/`,
  SHOULD) live; `folio bootstrap` runs `upskill install`; `folio check` rules
  declared ⇒ installed ⇒ current via `upskill status --format json`. upskill
  owns the agent-layout map; folio never hardcodes an agent path.

**Upstream deps:** git-std **G1 + G4 + B3**, prim **B1 + H1′**, upskill
**U1–U3** (contract conformance, registry dump, declaration file — all
pending repo verification).
**Exit criteria:** an agent session completes a full governed release
(check → fmt → version bump plan → human approve → apply) driven purely by MCP
guidance + shell execution, with the repo's skills installed by folio.

## v0.7 — Pages (documentation portal)

**Verb:** `pages`
- Assembles mdbook + rustdoc + Dokka + TypeDoc into one portal; deployment
  per `pages-deployment.md`; repofolio's own book is the first tenant.

**Upstream deps:** none. **Exit criteria:** driftsys.github.io portal built by
`folio pages` in CI.

## v0.8 — Editor (LSP)

**Verb:** `lsp`
- Diagnostics for Foliofile + manifest; reuses the pinned
  `tree-sitter-foliofile` WASM artifact; documented Helix/Zed/Neovim/VS Code
  setup (the audience skews to these editors — the reason tree-sitter won).

**Upstream deps:** `tree-sitter-foliofile` at a tagged release.
**Exit criteria:** corpus diagnostics render identically in CLI and LSP.
*Note:* this is the **one sanctioned re-entry point** for the prim library
option — if measured format-on-save latency demands in-process calls, a
`prim-core` split is decided here, on numbers, and stays invisible behind
`repofolio-fmt`.

## v0.9 — Hardening & (optional) forge

- **Air-gap & mirror:** `folio bundle --target <triple>` /
  `folio bootstrap --from-bundle` (full pinned toolset + checksums, zero
  network) and `FOLIO_TOOLS_MIRROR` — the §22-traceability install path for
  the regulated audience, covering the shell tier too.
- **Cosign verification on by default** in `folio tools` — publisher
  deadline: git-std D1, prim F2, upskill U4 all signed + SBOM'd by here
  (target was v0.5; v0.9 is the hard gate).
- Supply-chain formalization for folio itself: cosign + SPDX SBOM + SLSA
  provenance + `cargo audit` — folio meets every requirement `folio check`
  enforces on `prebuilts`-category repos.
- **`forge check` (experimental):** remote GitHub/GitLab drift-check —
  branch protection, labels, repo metadata vs `project.toml` — via gh/glab
  shell-outs (pinned like any other tool), read-only first; `forge apply`
  only after `check` proves out.

**Exit criteria:** a clean air-gapped machine bootstraps from a bundle to a
green `doctor`; every orchestrated tool verifies signature + checksum.

## v1.0 — Freeze

- Repofolio spec 1.0 (schema `v1.json` frozen; additive-only thereafter),
  Foliofile grammar 1.0, folio CLI surface + `FOLIO-` code registry declared
  stable, and the **JSON-schema contracts** of every orchestrated tool
  (git-std, prim, upskill) at 1.0 in lockstep.
- Gate: everything in the ecosystem (repofolio, git-std, prim, dock, upskill,
  schemas) governed by folio, releasing via `folio version bump`, documented via
  `folio pages`, agent-operable via `folio mcp`.

---

## Cross-cutting workstreams (every release)

- **Tool contract compliance** — codes, `--format json`, dry-run on every
  mutating verb, **installability (assets + checksums)** — from v0.1, folio
  included. Arbitrary recipes emit execution previews; exact effect plans
  apply to built-in mutators and supported delegated operations.
- **Dogfood-first** — repofolio + git-std + prim are the standing fixtures;
  a release that its own repos fail doesn't ship. folio consumes the same
  CLI + JSON front doors agents and CI use — permanently.
- **Walking & scoping** — folio owns repo walking (gitignore-aware,
  `--since`/`--staged`) and passes file lists to spawned tools.
- **Diagnostic corpus** — grows continuously from v0.3; shared by runner
  parser, tree-sitter grammar, and LSP.

## Dependency ladder (condensed)

Historical backlog IDs below map to live GitHub stories in the
[2026-09-12 upstream reconciliation](folio-plan.md#upstream-story-reconciliation--2026-09-12).
Use that dated inventory for tracking; several original prerequisites already
have closed issues, while released-contract verification remains open.

| folio | pulls first |
|---|---|
| v0.1 | — |
| v0.2 | prim **F3 (P0)** · G1 · A1 · D2 |
| v0.3 | Foliofile grammar points closed |
| v0.4 | — |
| v0.5 | git-std A1 · A2 · C3 (JSON) · B2 (+fidelity) |
| v0.6 | git-std G1 · G4 · B3 — prim B1 · H1′ — upskill U1–U3 |
| v0.8 | tree-sitter-foliofile tagged |
| v0.9 | git-std D1 — prim F2 — upskill U4 (signing deadline) |
