# M1 execution ledger — rulings and deferred findings

_Working memory from the 2026-09-06 subagent-driven execution of
2026-09-06-m1-check-plan.md. Preserved because it holds every ruling made
on the author's behalf during execution, with the reasoning and cost of each.
Garden into docs/decisions/ alongside the design document; the deferred-minor
list should become tickets rather than durable records._


## Pre-flight scan

Pairs sharing a file or interface:

| Pair | Produces -> consumes | Finding |
| --- | --- | --- |
| 1 -> 2 | discovered path + format -> parser | clean |
| 2 -> 3 | serde_json::Value -> jsonschema | clean |
| 3 -> 4 | bundled schema file -> parity test | clean |
| 5 -> 7 | Report type -> rules emit into it | **CONFLICT 1** |
| 6 -> 7 | Ecosystem registry -> rules read markers per layer | clean |
| 5 -> 8 | Report -> pipeline assembles | **CONFLICT 1** |
| 6 -> 8 | registry -> pipeline detection | clean |
| 3 -> 8 | validation -> pipeline skips 002 when 001 failed | **CONFLICT 1** |
| 7 -> 9 | findings -> exit-code grading | clean |
| all -> 10 | dogfood expectations | **CONFLICT 3** |

Self-consistency, per task: 1 clean, 2 clean, 3 clean (the four schema test
cases exist and were verified), 4 clean, 5 see conflict 1, 6 clean,
7 **CONFLICT 2**, 8 clean, 9 clean, 10 see conflict 3.

Ruling: FOLIO-002 skipped state — step 8 requires FOLIO-002 to report as
"skipped" when FOLIO-001 already failed, but the step 5 Report type mirrors
schemas/markspec/diagnostics/v1.json, whose severity enum is only
error|warning|info. There is no skipped severity to use. Decided: emit
FOLIO-002 at severity "info" with a message stating it was skipped and why.
It stays expressible in the mirrored shape, it does not affect exit-code
grading, and it maps cleanly to SARIF later as level "note" with kind
"notApplicable". Cost if wrong: a consumer counting info-severity findings
as real diagnostics misreads a skip; the alternative, adding a fourth
severity, would break the mirror the plan requires.

Ruling: FOLIO-101 must not own manifest presence — step 7 listed "the
manifest" among repofolio's required paths, but FOLIO-001 already owns
manifest presence. A repository with no manifest would report the same fact
twice under two codes. Decided: remove the manifest from FOLIO-101's
required list; FOLIO-001 is its sole owner. Cost if wrong: none identified;
the codes stay disjoint either way.

Ruling: step 10's dogfood expectation is stale — it lists LICENSE among the
files this repository lacks, but LICENSE was added in commit d752839.
Decided: correct the expected finding set to bootstrap, runw,
.gitattributes, .editorconfig and scripts/. Cost if wrong: an implementer
would chase a finding that cannot occur and might weaken an assertion to
match.

## Execution

Ruling: batch plan steps 1-4 into one dispatch — they are one crate
(repofolio-manifest), one dependency chain (discover -> parse -> validate ->
parity), and one review surface. Four separate dispatches would mean four
review rounds over roughly two hundred lines. Cost if wrong: a defect in an
early step is found later in the batch review rather than immediately.

Batch A (steps 1-4): dispatched sonnet, BASE 5c7bef9. Implementer produced
the work but stalled before formatting, committing or reporting — it ended
waiting on a background process that never arrived. Controller verified the
uncommitted tree directly: bundled schema byte-identical to
../schemas/project/v1.json, 15 unit tests plus the parity test passing,
clippy clean, `cargo fmt --check` failing. Resumed the same implementer to
format, commit per step, and write its report rather than finishing the work
in the controller session.

Batch A: implementer DONE, commits 3c7edad..09cdf6e (4 commits). 19 tests
(18 unit + 1 parity), clippy and fmt clean. Implementer additionally verified
the parity test fails on a deliberate byte change before restoring it, which
is the check that makes that test meaningful.

Ruling: discovery selects silently by priority when more than one manifest
is present, rather than raising an error. The plan specified the search order
(toml, yaml, json) and required a test for the multiple-present case, but did
not say ambiguity is an error, and M1 has no diagnostic code for it. Decided:
silent priority selection stands for M1. Cost if wrong: a repository holding
both project.toml and project.yaml gets no warning that one is being ignored,
which is a genuine conformance smell. Recorded below as a candidate rule.

Task A: minor (deferred): a repository containing more than one project
manifest is ambiguous and arguably deserves its own diagnostic code. Out of
M1 scope — no code exists for it in the four-code registry. Consider when the
FOLIO- registry is next extended.

Task A: minor (deferred): anyhow, snapbox and assert_cmd were deliberately
not added. They belong to the CLI crate at plan step 9, outside this batch.

Note: the repository's post-commit review hook fired inside the implementer's
session and asked it to dispatch review subagents. It correctly declined,
per the no-subagents clause in its dispatch. The hook is written for a
top-level session and does not account for running inside a delegated
implementer; worth knowing if hook behaviour is revisited.

Batch A task review: spec compliant. Reviewer independently re-ran the suite
and re-checked schema byte-identity rather than trusting the report, and
confirmed the four schema fixtures are byte-identical to the canonical
driftsys/schemas test set. Task quality: needs fixes — two Important.

Batch A: minor (deferred): validate_manifest recompiles the schema on every
call instead of caching the compiled validator behind a OnceLock. Irrelevant
for one manifest per run; would matter if repofolio-core validates many.

Batch A: minor (deferred): yaml_key_to_string, covering non-string YAML
mapping keys, has no direct test.

Batch A: fix round 1/5 dispatched, FIX_BASE 09cdf6e. Findings: (1) verbatim
duplicated temp-directory helper across discover.rs and parse.rs test code;
(2) YAML conversion swallows Yaml::BadValue and failed Real parses into
Value::Null.

Ruling: finding 2 is load-bearing, not cosmetic. This batch's binding
constraint is that parse errors and schema errors stay distinct because they
map to FOLIO-001 and FOLIO-002. A swallowed parse failure never becomes
FOLIO-001 at all, and the manifest's `metadata` field is
additionalProperties:true with no type constraint, so a malformed anchor
inside it produces a null that passes schema validation. Cost if wrong: the
fix propagates ParseError::Yaml where yaml-rust2 flags a bad value, which
could in principle reject a manifest some other implementation accepts.

Batch A: fix round 1/5 (2 addressed pending re-review, 0 open; commits
09cdf6e..145b1ee). tempfile dev-dependency replaced the duplicated helper and
brought Drop-based cleanup with it. yaml_to_json now returns Result and
propagates Yaml::BadValue and unparseable or non-finite Real as
ParseError::Yaml. Implementer reproduced the BadValue case against
yaml-rust2 0.12.0 source and added a test nesting the malformed value under
`metadata`, confirming the validation bypass was real rather than
theoretical. 22 tests, clippy and fmt clean.

Batch A: fix round 1/5 re-review — both findings ADDRESSED, no new breakage.
Re-reviewer independently confirmed all three new tests fail against the
reverted code (they assert expect_err plus a ParseError::Yaml match, not a
null return), and read the bundled schema to confirm the metadata bypass was
real. Implementer also propagated Yaml::Alias, beyond what was asked.

Batch A: complete (commits 3c7edad..145b1ee, review clean)

Batch B (steps 5-6, Report and Ecosystem types): dispatched sonnet,
BASE 145b1ee. Human checkpoint follows this batch per the plan.

Batch B: implementer DONE_WITH_CONCERNS, commits bc07561..6862d46. 10 tests
including a JSON snapshot pinning multi-severity Report output. Clippy and
fmt clean.

Checkpoint (human): approved the Report type as built. Two choices worth
recording — Report::new derives count and by_severity from the findings list
so the summary cannot drift from it, and `code` is a String rather than an
enum so plugin ecosystems can mint their own codes (RUST-001, UNITY-001); a
closed enum would have made third-party ecosystems impossible.

Note: the Report carries an additive `layer` field, and markspec's
diagnostics schema sets additionalProperties:false. folio diagnostics are
therefore structurally consistent with the house shape but will not validate
against markspec's own schema. Accepted; folio gets its own schema.

Ruling: accept the recursive Commands tree (Command(String) | Group(map)).
It lets C's build-system level nest without reshaping the frozen type, at the
cost of being untyped — nothing distinguishes commands.build.debug from
commands.debug.build, and a malformed file deserialises happily and fails at
lookup. Shape enforcement therefore moves to schemas/folio-ecosystem/v1.json,
which does not yet exist and is now load-bearing rather than optional. Cost
if wrong: a malformed ecosystem file produces a confusing runtime lookup
failure instead of a parse error, until that schema is written.

Batch B: fix round 1/5 dispatched, FIX_BASE 6862d46. Finding: Ecosystem does
not mirror the on-disk envelope — name is flat where the file nests it under
[ecosystem], and version is absent entirely, so a v0.4 loader would need the
translation layer the mirror exists to prevent. Also directed: make `always`
serde(skip) rather than serde(default), because a third-party file could
otherwise set always = true and force itself active in every repository.

Batch B: fix round 1/5 (1 addressed pending re-review, 0 open; commit
6862d46..a83596f). Ecosystem now nests name and version under an
`ecosystem: EcosystemMeta` field matching the real [ecosystem] table;
version typed u32 and documented as the file-format version rather than a
semantic version. `always` is serde(skip). An optional `$schema` field was
added so a loaded file round-trips and a strict deserialiser does not reject
the real file on an unknown key. 12 tests.

Batch B: fix round 1/5 re-review — 1 addressed, 2 open. `always` lockdown
ADDRESSED and the doc-comment claim about serde default versus skip verified
accurate. Main finding NOT ADDRESSED: the reviewer deserialised the actual
reference file and it fails with "missing field `markers`", because
Ecosystem.markers lacks serde(default) while the real scaffold file omits
both [markers] and [commands]. Directive (b) NOT ADDRESSED: the mirror test
used a fabricated literal that populates [markers] and [commands.build],
sections the real file does not have, so it passed while the genuine file
failed.

Note for the record: that is the failure mode the checkpoint exists to catch.
The type compiled, twelve tests passed, clippy and fmt were clean, and the
type still could not read the only real file of its own format. No automated
gate in this plan would have caught it; only deserialising the actual file
did.

Batch B: fix round 2/5 dispatched, FIX_BASE a83596f. Findings: (1) add
serde(default) to markers and audit every other field for whether a real file
may omit it; (2) replace the fabricated mirror test with one using the
reference file's content verbatim, plus one that reads the actual sibling
file when present and skips when absent, matching the schema parity test's
pattern in repofolio-manifest.

Batch B: fix round 2/5 re-review — all findings ADDRESSED. Reviewer verified
by execution in a disposable worktree rather than by reading: reverted
serde(default) on markers and reran each test to see which actually catch the
regression. Byte-for-byte diffed the verbatim test's literal against the live
reference file. Confirmed the corrected sibling path depth (three levels, not
the four I specified).

Batch B: complete (commits bc07561..4de3065, review clean)

Batch B: minor (deferred), and it generalises beyond this batch: the
skip-when-absent tests provide zero protection in an environment without the
sibling checkouts. Two tests are affected — the ecosystem sibling-file test
here, and the schema parity test in repofolio-manifest. In CI cloning only
folio, both silently report ok without exercising anything, so bundled-schema
drift would never be caught automatically. The verbatim literal test is the
only unconditional guard. Candidate fix when CI is set up: have CI check out
driftsys/schemas and driftsys/repofolio, or make the skip an error when an
environment variable such as FOLIO_STRICT_PARITY is set, so the skip is a
local-developer convenience only.

Batch C (steps 7-8, rules and the check pipeline): dispatched sonnet,
BASE 4de3065.

Batch C task review: spec ❌ on one Critical, otherwise compliant. Reviewer
confirmed by execution: the no-abort property holds, FOLIO-001 alone owns
manifest presence (enforced in three independent places), per-rule tests pin
code, severity, layer and file together.

Critical: crates/repofolio-core/tests/fixtures/compliant/Cargo.lock is not
tracked by git. The root .gitignore line 2 is `Cargo.lock`, unanchored, so it
matches every Cargo.lock in the tree. The reviewer cloned the repository
fresh and compliant_fixture_produces_no_diagnostics fails there with a
FOLIO-101 for the missing lockfile. The suite passed only because a stray
untracked file sat in the working tree. This is the second time in this plan
that verification-in-place gave a false pass; the first was the fabricated
mirror test in batch B.

Ruling: replace the `" or "` marker convention with an untagged enum,
MarkerSpec { One(String), AnyOf(Vec<String>) }. The reviewer reached this
independently and rated it Important. The string form is a micro-DSL inside a
v1.0-frozen type, documented only in Rust doc comments that a third-party
ecosystem author never reads; Part 6 of the design doc shows plain path
arrays, so that author would write two entries and get the exact double
warning the convention exists to prevent. It also mis-splits any path
containing " or ". Cost if wrong: reshaping a type the human checkpoint
already approved — justified because the alternation requirement only emerged
in batch C, so this is new information rather than re-litigation. The
implementer's scope is extended to update Part 6 so spec and shape agree.

Batch C: fix round 1/5 dispatched, FIX_BASE 0c3e826.

Batch C: fix round 1/5 (3 addressed pending re-review; commits
0c3e826..b89eb3c). Controller verified the Critical fix independently rather
than accepting the report: cloned the repository fresh into a scratch
directory, confirmed git ls-files now lists the fixture lockfile, and ran the
fixture suite there — 3/3 pass, including compliant_fixture_produces_no_
diagnostics, which was the failing test. The .gitignore change is a single
negated pattern, !crates/*/tests/fixtures/**/Cargo.lock; the root workspace
lockfile's ignored status is untouched as directed.

MarkerSpec { One(String), AnyOf(Vec<String>) } replaces the string
convention, and Part 6 of the design document now shows the nested-array form
(`should = ["rust-toolchain.toml", ["rustfmt.toml", ".rustfmt.toml"]]`), so
the normative spec and the frozen type agree.

Batch C: fix round 1/5 re-review — all three findings ADDRESSED, verified by
execution. Reviewer cloned fresh and ran the fixture suite there (3/3 pass),
confirmed the root workspace lockfile is still ignored as directed, and
grepped the repository to confirm no `" or "` splitting logic survives
anywhere — the remaining occurrences of that string are expected-output
assertions against the new Display impl, not parsing input.

New Important introduced by the fix: MarkerSpec is absent from lib.rs's
public re-export list, so a publicly exported Markers has fields of a type
with no external path. Reviewer verified with a probe: use
repofolio_core::MarkerSpec fails with E0432. Before the fix those fields were
Vec<String> and needed no import, so this is an API regression caused by the
remediation. Fix round 2/5 dispatched, FIX_BASE b89eb3c. Also directed: a
crate-wide sweep for the same class of error, and a regression test placed in
tests/ rather than in a unit module, since only an integration test compiles
as an external crate and can see the public API.

Batch C: minor (deferred): the untagged enum accepts an empty alternatives
group, `should = [[]]`, which evaluates as permanently unsatisfied.
Ruling: leave the deserialiser permissive and reject this in
schemas/folio-ecosystem/v1.json as a minItems constraint, consistent with the
earlier ruling that ecosystem-file shape enforcement lives in the schema
rather than the Rust type. Cost if wrong: a meaningless ecosystem file
authors cleanly and produces a marker that can never be satisfied, until that
schema exists. This is the second requirement now recorded against that
schema, which still does not exist.

Batch C: fix round 2/5 (1 addressed pending re-review; commit
b89eb3c..5c33fb4). MarkerSpec re-exported; regression test placed in
tests/public_api.rs, which compiles as an external crate and therefore sees
only what lib.rs re-exports. Implementer reports the crate-wide sweep found
nothing else, and — usefully — that rustc's private_interfaces,
private_bounds and unreachable_pub lints do NOT catch this class of bug,
reproduced under those flags with zero warnings. If that holds, there is no
compiler protection for "exported type has a field of an unexported type" in
this shape, and the integration test is the only guard. Sent for independent
verification.

Batch C: fix round 2/5 re-review — ADDRESSED, no new breakage. Reviewer
reproduced the pre-fix E0432 in a disposable worktree, redid the public-API
sweep itself by grepping every pub item rather than trusting the report, and
verified the lint claim with a control the implementer had not run: it
appended a genuinely dead pub fn and confirmed unreachable_pub DOES fire on
it, proving the lints were enabled and simply do not catch this shape.

Confirmed for the record, because it applies to every crate in this
workspace: rustc's private_interfaces, private_bounds and unreachable_pub do
not flag a pub type that is reachable through another public type's field but
has no nameable path. unreachable_pub only fires on a pub item with zero uses
anywhere. An integration test under tests/ is the only guard.

Batch C: complete (commits 41b5857..5c33fb4, review clean)

Batch D (steps 9-10, CLI wiring and dogfood): dispatched sonnet, BASE 5c33fb4.

Batch D task review: spec compliant, task quality APPROVED. No Critical, no
Important. Reviewer re-executed every numeric claim — exit codes, JSON bytes
diffed against repofolio-core's own pinned snapshot, and both dogfood runs —
and all matched exactly. Confirmed the warnings-only-exits-0 test exists and
would fail if grading collapsed to "any finding means 1", and independently
validated the new project.toml against the canonical schema with Python's
jsonschema rather than through folio itself.

Batch D: complete (commits 8a22563..b46fd56, review clean)

Batch D: minor (deferred): human output omits `layer`, which the JSON
carries. Harmless in M1 — the two hardcoded layers have zero marker-path
overlap, so the file name disambiguates — but it becomes a real ambiguity
when a colliding-name ecosystem lands at v0.4.
Batch D: minor (deferred): thiserror is an unused dependency in
crates/repofolio, inherited from the M0 scaffold. Confirmed genuinely unused.
Left alone under the surgical-changes rule, but it will ship in a published
manifest.
Batch D: minor (deferred): no acceptance test covers `folio check` with no
path argument, which defaults to the current directory.
Batch D: minor (deferred): plan doc Open item 1 is now ambiguous — it says
"repofolio's own manifest produces two genuine FOLIO-002 failures", meaning
the driftsys/repofolio repository, but reads as though it means folio's.
folio's own manifest validates cleanly. For whoever gardens docs/wip/ next.

Milestone dogfood, verified by running the built binary:
  folio      exit 1 — 5 errors, 7 warnings
  git-std    exit 1 — 3 errors, 4 warnings
  repofolio  exit 1 — 1 error, 3 warnings; the error is FOLIO-002 reporting
             object-form authors and the unexpected `versioning` key, which
             is the schema-versus-prose drift recorded in the design document
             at Part 8 item 2, now demonstrated by execution rather than
             argued from documents.

Final whole-branch review (opus): no Critical, 8 Important, 12 Minor.
Headline: always_cannot_be_set_from_a_loaded_file was INERT — its
`always = true` sat under [ecosystem] rather than at the document root, so
reverting serde(skip) to serde(default) left all 66 tests passing. A previous
re-reviewer had confirmed that test by reading it and misdescribed it as
using a top-level key. Only mutation caught it. Controller independently
mutation-tested the fix: it now fails as it should.

Fix wave: 9 commits, 81c0467..b9d801f. Scoped re-review — all nine
ADDRESSED, no new breakage, 70 tests green, ready to merge and publish.

Note: my instruction on the packaging fix was wrong. `include` cannot
override cargo's exclusion of any directory containing a Cargo.toml; the
implementer found this, renamed the fixture manifests to Cargo.toml.fixture
and materialises them at test time. The reviewer independently proved the
rust layer still activates by deleting Cargo.lock from a materialised copy
and watching FOLIO-101 fire.

Deferred: tests/public_api.rs is now narrower than its name — enum-level
non_exhaustive does not block variant construction, so the adaptation was
compiler-forced only for Markers, and the test no longer demonstrates
MarkerSpec construction from outside the crate, only matching. Not vacuous.
