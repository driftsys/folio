# folio — start plan & upstream gap analysis (rev. 2)

_Drafted 2026-08-29; **rev. 2 same day** — binary orchestration replaces
crate-linking, `folio tools` installer added, upskill added as fourth pillar,
D1 (Rust) re-examined and held. Companion to `claude/git-std-backlog.md`,
`claude/prim-backlog.md`, `context/folio-architecture-decisions.md`,
`context/companion-tools.md`, and `folio-roadmap.md` (Rust era). Scope:
(1) what is missing on git-std, prim, and upskill **today**, graded by what it
blocks; (2) a phased plan for starting folio and developing it in the right
direction._

**Governing insight:** folio v0.1 (`check` + `init`) depends on **nothing**
upstream. Delegation dependencies land at later milestones and are pulled
**just-in-time** — never a prerequisite wall in front of the first commit.

Priority/size legend as in the backlogs: P0/P1/P2 · S ≈ days, M ≈ 1–2 weeks,
L ≈ multi-week. Story IDs reference the existing backlogs.

---

## Integration architecture (rev. 2026-08-29)

Supersedes the rev-1 crate-linking design. Seven decisions, assumed everywhere
below.

1. **Binary orchestration, not crate linking.** folio invokes **pinned tool
   binaries** through their CLIs and consumes `--format json` — the *same*
   contract agents and CI consume. One contract instead of two (no parallel
   Rust-API surface to drift), permanently dogfooded by folio itself, and
   **uniform** across driftsys tools and the external tier
   (shfmt/shellcheck/rustfmt/…) — no internal/external split in how tools are
   pinned, verified, invoked, or debugged. Every folio invocation is
   reproducible by hand (`folio which <tool>` prints the exact binary).
2. **Sync tools, plan/apply, registries-as-data — unchanged in spirit.** The
   plan JSON (`bump --dry-run --format json`) **is** the delegation interface
   now, not serde on a linked type. Plan→approve→apply stays the choreography
   for every mutating verb, folio's own included.
3. **Facades stay folio-side** (`repofolio-fmt`, `repofolio-release`): they wrap
   `Command` + JSON parsing + `FOLIO-` code mapping + policy (single-version
   mode, walking boundary). The facade is the anti-corruption layer; that it
   spawns rather than links is an implementation detail, swappable later
   without touching callers.
4. **Lib option = targeted back-pocket optimization, not architecture.**
   prim **H1 (library split) is deleted.** The one realistic future candidate
   is `prim-core` inside `folio lsp` (v0.8) if format-on-save latency demands
   in-process calls — decided then, on measurement, not now.
5. **`folio tools` — the installer is core folio.** Pins resolved into
   `.folio.lock` (`version` + `source` + per-target `sha256`); fetch → verify
   → atomic land into `~/.cache/folio/tools/<name>/<version>/<target>/`;
   **absolute-path invocation, zero PATH mutation, no shims**. Three source
   classes: `github-release` (driftsys tools, shfmt, shellcheck, actionlint,
   gitleaks — raw release assets, never `curl | sh`), `toolchain-managed`
   (rustfmt/clippy via rustup; folio drives the owner, pins via
   `rust-toolchain.toml`), `system` (git, bash — verify-only, never install).
   TOFU checksum capture on first resolution; cosign verification wired
   wherever the publisher signs. `folio bundle` / `--from-bundle` +
   `FOLIO_TOOLS_MIRROR` for air-gapped/regulated installs (covers the shell
   tier too — something linking never could). Scope guard: exact pins only, no
   version ranges, no shims, no PATH/env activation, no registry service.
   **Consequences:** (a) publisher **checksum files become a fetch
   prerequisite**, cosign+SBOM strongly preferred (target ≤ v0.5, hard
   deadline v0.9); (b) `bootstrap` shrinks to *git plumbing + install folio*;
   `folio bootstrap` ensures everything else — including git-std, then
   `git std hooks install`. One installer instead of one-and-a-half.
6. **upskill = fourth orchestrated binary.** Same contract, no lib, no special
   case. upskill **owns the agent-layout map** (which agent reads which
   directory/format) and skill materialization; folio never hardcodes an agent
   path. folio drives `upskill install` (bootstrap), `upskill status
   --format json` (check rules: declared ⇒ installed ⇒ current), `upskill
   registry --format json` (mcp/explain payload). Declaration lives in
   upskill's own config file, not `project.toml` (per `config.<tool>`
   namespacing). Drift rule: **registries hold facts, MCP serves facts, skills
   hold procedure** — a skill is a thin pointer to the CLIs + MCP, never a
   copy of rule content.
7. **D1 (Rust) re-examined — holds on revised grounds.** The crate-linking
   argument is gone; Rust stands on: the back-pocket lib option (#4) only
   exists in Rust; the v0.8 LSP endgame (tower-lsp, native tree-sitter);
   suite coherence — the agent building folio mimics the sibling repos'
   established Rust patterns; binary weight/startup (folio installs the
   others and sits near hook paths); and **agentic velocity** — with Claude
   writing and Sebastien verifying, rustc+clippy are the agent's iteration
   harness and the human's mechanical pre-review; the scarce resource is
   review attention, and strictness is review infrastructure. ADR note:
   "re-examined 2026-08-29 after the orchestration decision; holds."

**The binaries are the product.** Git-fired hooks run `git std hooks run`
(git is the caller); interactive `git std commit` stays a direct user command;
agents shell-execute per the tool contract; and folio now consumes the very
same front doors.

---

## Part 1 — What's missing today

### 1.1 git-std (v0.11.12)

**Tier 1 — blocks folio v0.5 "version + hooks" delegation:**

- **A1 · Single-canonical-version mode contract (P0, S).** Unchanged — the
  policy seam. `monorepo = false` ⇒ one version / one tag / one changelog,
  documented + golden-tested.
- **A2 · Workspace version-parity check (P1, S).** Unchanged.
- **C3 (retitled) · Stable CLI + JSON contract (P1, S).** Was "stable crate
  API." Now: documented, versioned **JSON schemas** for `version --format
  json`, `bump --dry-run --format json` (B2), `lint --format json` (B1),
  `hook run --format json` (E3); a schema-stability policy (additive within a
  minor, breaking ⇒ major + changelog callout). The same discipline the crate
  API would have needed — different serialization. No crate work required.
- **B2 · JSON for `bump --dry-run` plan (P1, S).** **Promoted to the
  delegation interface itself.** Add one AC: **plan/apply fidelity** — folio
  runs dry-run → human approves → folio runs the real `bump` and diffs actual
  effects vs plan, failing loudly on divergence; git-std documents that an
  unchanged repo state yields plan-identical apply.

**Tier 2 — blocks the knowledge layer (folio v0.6):**

- **B1 · SARIF + JSON for `lint` (P0, M)** · **B3 · exit/diagnostic-code
  contract (P1, S)** — unchanged.
- **G1 · Lint rule registry (P1, M)** + **NEW G4 · `git std registry
  --format json` (P1, S).** The registry-via-crate-API clause becomes
  registry-via-CLI: one dump of lint rules + codes + the repo's *effective*
  commit convention (resolved `.git-std.toml`). Feeds `folio mcp`/`explain`.

**Tier 3 — reprioritized by the installer:**

- **D1 · Signing + SBOM + provenance — promoted (P1→installer-adjacent).**
  Checksummed release assets already ship (musl/darwin/msvc + SHA-256) —
  the fetch prerequisite is **met today**; cosign/SBOM target ≤ folio v0.5.
- **Install-story contradiction — decided by architecture.** Canonical =
  GitHub release assets (folio fetches them; the human one-liner wraps them).
  `cargo install` demoted to unsupported-convenience or removed — reconcile
  README vs SPEC §1.2 accordingly (was follow-up #5).
- F1/F2/F3, E1 unchanged, non-blocking.

### 1.2 prim (v1 complete)

**Tier 1 — blocks folio v0.2 "tools + hygiene":**

- **G1 · verb model** · **A1 · hygiene contract** · **D2 · JSON/SARIF** —
  unchanged.
- **H1 deleted** (no library split). Replaced by:
- **NEW H1′ · `prim registry --format json` (P1, S).** Rule/code registry as
  data — the same table D2 and G3 already require, exposed as one dump.
  Feeds `folio mcp`/`explain`.
- **F3 · Prebuilt binaries + checksums — REPRIORITIZED P2→P0 (S).** folio
  fetches per-platform release assets; prim currently has no prebuilt story.
  Without F3, `folio tools` cannot ensure prim. This is now prim's most
  urgent story alongside G1. (F2 cosign/SBOM: target ≤ folio v0.5.)

**Tier 2 — deepens folio lint / feeds v0.6:** B1, G2/G3, C1 — unchanged.

**Tier 3:** E1/E2 walking (boundary now trivial: folio walks, passes file
lists to the spawned binary; prim walks only standalone), D1 LSP, E3 —
unchanged. **Deferred:** the library split re-enters *only* at folio v0.8 on
measured LSP latency need.

### 1.3 upskill (NEW — status unverified)

Needed for folio v0.6, all conditional on repo verification:

- **U1 · Tool-contract conformance (P1, M).** `install` (with dry-run plan),
  `status --format json`, stable codes, exit contract.
- **U2 · Agent-layout map as data (P1, S).** `upskill registry --format json`:
  known agents → directories/formats, installed-skill inventory.
- **U3 · Declaration file (P1, S).** upskill-owned config declaring the
  repo's skills; folio's check rule is only declared ⇒ installed ⇒ current.
- **U4 · Prebuilt release assets + checksums (P0, S).** Same installer
  prerequisite as prim F3.

**Verification required:** current CLI surface, manifest/install model, and
whether skills install from git-hosted packages — confirm against
`driftsys/upskill` before writing the spec paragraph.

### 1.4 Cross-cutting

1. **Tool contract v1 gains clause 5 — installability.** Every orchestrated
   tool ships per-platform release assets + a checksum file (cosign + SBOM to
   follow), alongside the original four clauses (codes · JSON/SARIF ·
   dry-run plan on mutators · registry dump; MCP stays folio-only).
2. **Repofolio spec — agent layer (SHOULD).** `.agents/skills/` joins the
   progressive file set; skills declared (upskill config) and installed by
   `folio bootstrap` via upskill.
3. **Doc debt.** ADR refresh now covers: Rust re-examined (#7), binary
   orchestration (#1), installer scope (#5), folio-is-the-MCP, agent layer.
   Roadmap/overview Deno framing still to purge; `repofolio-roadmap.md`
   question still parked.
4. **Foliofile grammar — 4 open review points.** Unchanged; gate M3.

---

## Part 2 — folio plan

### Phase M0 — Decisions & scaffold (days, no feature code)

1. **Workspace.** **`driftsys/folio` — own repo** (re-examined 2026-08-29,
   second time same day; supersedes the earlier one-repo call — see
   rationale below). Cargo workspace: `repofolio` (bin `folio`), `repofolio-core`
   (check engine + scoring), `repofolio-manifest`, `repofolio-templates`,
   **`repofolio-tools`** (installer: resolve/fetch/verify/cache/invoke),
   facades `repofolio-fmt` + `repofolio-release` (Command + JSON + policy),
   later `repofolio-task`.
   **`repofolio` stays spec-only:** the standard's markdown, the published
   schema, and the normative `tests/fixtures/` (compliant/partial/empty) —
   a language-agnostic corpus any implementation tests against, not folio's
   private data. folio's own crate tests and the Foliofile diagnostic corpus
   (M3) live in `driftsys/folio`, pinning `repofolio`'s fixtures as a
   dependency (submodule or vendoring script — decide in-repo).
   **Rationale for the split:** folio now ships installable per-platform
   binaries under the same tool contract (clause 5) as git-std/prim/upskill
   — a software-release cadence (v0.1→v1.0, build matrix, signing) that
   doesn't belong in a spec repo versioned by schema discriminator; matches
   the pattern every sibling tool already follows; keeps `repofolio`
   legible to someone who just wants to read the standard.
2. **ADR refresh** per 1.4 #3.
3. **Tool contract v1** — five clauses, applies to folio itself from v0.1;
   include the skills drift rule (facts vs procedure).
4. **Spike (replaces the crate-API spike): JSON-surface + asset inventory.**
   Per tool: which verbs already emit JSON vs the contract (git-std: lint ✓,
   version ✓, bump-plan ✗, registry ✗; prim: none yet; upskill: unknown);
   which publish fetchable checksummed assets (git-std ✓; prim ✗ → F3;
   shfmt/shellcheck/actionlint ✓; upskill ?). Output: a gap table that *is*
   the upstream work order.
5. **Reconcile the git-std install docs** to the architecture-decided answer
   (release assets canonical).

### Phase M1 — v0.1 "check + init" (zero upstream deps)

Unchanged from rev. 1: `check` (manifest 3-formats → bundled schema →
core-layer MUST/SHOULD/MAY → ecosystem detection → `FOLIO-` codes,
`--format json`, CI exit grades), `init`/`add` (handlebars ecosystems,
managed sections, `.folio.lock`), structure-only — no tool spawning yet.
**Dogfood gate:** repofolio + git-std pass `folio check` or carry ticketed
exceptions.

### Phase M2 — v0.2 "tools + hygiene" (installer + first delegation)

The first release that *spawns* tools is the release that *installs* them:

- **`folio tools` epic:** lock resolution, fetch/verify/land, absolute-path
  invocation, `folio which`; source classes; TOFU checksums; offline
  fails-fast with manual-fetch instructions. (Bundle/mirror deferred to v0.9.)
- **`folio doctor`:** lock vs cache vs default toolset vs PATH-skew — the
  scattered skew ACs get their home. **`folio bootstrap`** re-shaped per
  consequence 5b (ensures git-std; runs `git std hooks install`).
- **`fmt` / `lint` / `fix`:** `repofolio-fmt` spawns pinned prim (batch file
  lists, one spawn per verb); shell tier shfmt/shellcheck; aggregated report,
  upstream codes passed through.

**Pull upstream first:** prim **F3 (P0 — prebuilts/checksums) + G1 + A1 +
D2**. git-std needs nothing new here (assets already checksummed).
**Exit:** dprint-Wasm retired; `folio fmt` byte-identical with standalone
pinned `prim fmt`; a clean-machine bootstrap ends with every tool ensured.

### Phase M3 — v0.3 "task runner"

Unchanged: own Rust parser (tree-sitter-foliofile stays the editor grammar;
shared conformance corpus starts here), `deno_task_shell` execution, explicit
`default`. **Gate:** the 4 grammar points closed. **Exit:** Justfiles retired
in repofolio + git-std.

### Phase M4 — v0.5 "version + hooks" (second delegation)

- `repofolio-release` drives the pinned `git-std` binary: `git std bump --dry-run
  --format json` → present `BumpPlan` → approve → `git std bump` → **verify
  actuals against plan** (fidelity AC from B2). Single-version mode pinned by
  config folio writes/validates. `hooks`: generate/validate files + shims;
  runtime stays the git-fired binary.

**Pull upstream first:** git-std **A1 → C3(JSON) → B2** (+ A2). cosign/SBOM
(git-std D1, prim F2) targeted by here.
**Exit:** folio's own release cut by `folio version`; polyglot fixture ⇒ one
tag, one changelog; a divergent plan/apply is caught by the fidelity check.

### Phase M5 — v0.6 "knowledge layer"

- **`folio mcp`** (guidance-only; rmcp/tokio feature-gated here and nowhere
  else) serving: repofolio rules + `FOLIO-` codes, `prim registry` (H1′),
  `git std registry` (G4), upskill inventory (U2), and the
  plan→approve→apply recipes per verb — all consumed as the tools' own JSON
  dumps, cached per repo.
- **`folio explain <code>`** across the suite, from the same dumps.
- **Skills governance:** spec agent-layer paragraph live; `check` rules
  declared ⇒ installed ⇒ current via `upskill status`.

**Pull upstream first:** git-std **G1 + G4 + B3**, prim **B1 + H1′**,
upskill **U1–U3** (post-verification).
**Exit:** an agent completes a governed release (check → fmt → plan →
human approve → apply) via MCP guidance + shell execution only.

### Phase M6+ — later

v0.4 ecosystem breadth + `migrate`/`self-update` (unchanged, dep-free —
sequenced between M3 and M4 at will); v0.7 `pages`; v0.8 `lsp` (**the one
sanctioned re-entry point for the prim lib option, on measurement**); v0.9
hardening — `folio bundle`/`--from-bundle`, mirror override, cosign
verification on-by-default (publisher deadline), folio meets every §22
requirement it enforces; v1.0 freeze — spec, grammar, CLI surface, `FOLIO-`
registry, and the *JSON schema* contracts of all orchestrated tools at 1.0 in
lockstep.

### Dependency map (milestone → upstream prerequisite)

| folio milestone | prim | git-std | upskill |
|---|---|---|---|
| v0.1 check+init | — | — | — |
| v0.2 tools+hygiene | **F3(P0)**, G1, A1, D2 | — (assets ✓) | — |
| v0.3 runner | — | — (grammar gate) | — |
| v0.5 version+hooks | — | A1, A2, C3(JSON), B2 | — |
| v0.6 knowledge | B1, H1′ | G1, G4, B3 | U1–U3 (verify first) |
| v0.9 hardening | F2 | D1 | U4 |

### Suggested first slice (~2–3 weeks)

1. **M0 items 1–5** — scaffold, ADRs, contract v1 (5 clauses), the
   JSON/asset inventory spike, install-docs reconciliation.
2. **`folio check` walking skeleton** — vertical slice to green/red on
   fixtures with JSON output.
3. In parallel: **prim F3** (prebuilts + checksums — now the critical-path
   upstream item, replacing the deleted H1) and **verify upskill** (1.3).

## Verification follow-ups

1. JSON-surface inventory results per tool vs contract (M0 spike) — becomes
   the upstream work order.
2. Release-asset/checksum matrix: prim F3 delivery; upskill U4; confirm
   shfmt/shellcheck/actionlint asset naming is stable enough to pin.
3. upskill ground truth: CLI surface, install model, git-hosted skill
   packages (gates 1.3 and the spec agent-layer paragraph).
4. Plan/apply fidelity mechanism on `git std bump` (B2 AC) — folio-side diff
   vs git-std-side guarantee; settle the split.
5. Foliofile grammar review points closed before any M3 code.
6. Bundled `project` schema vs published `v1.json` parity, asserted in
   folio CI.
7. Dogfood-gate criteria: which SHOULD-level findings repofolio/git-std may
   carry at M1 exit.
