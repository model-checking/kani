- **Feature Name:** Structured Verification Results (`export-json`)
- **Feature Request Issue:** [#942](https://github.com/model-checking/kani/issues/942)
- **RFC PR:** [#4727](https://github.com/model-checking/kani/pull/4727)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** Implemented; the example below is the proposed schema, not a verbatim
  dump of the proof-of-concept's own output (see the note at the start of "Example").

-------------------

## Summary

Specify the machine-readable file that Kani's `--export-json <path>` flag writes. The flag shipped
in [#4472](https://github.com/model-checking/kani/pull/4472) and is unstable. The file describes one
verification run: per-harness outcome, failed properties, check and cover outcomes by status,
resource cost, and run-result evidence (tool versions, configuration, harness selection) for
interpreting that outcome. It is evidence about the run, not enough to reproduce it byte-for-byte
(`cbmc_args` is recorded verbatim but is not a replayable argv). Whether this schema supersedes the
shipped v1 shape is the first open question.

## User Impact

This RFC exists because issue #942, *"Design Machine Readable Output - RFC"*, asks for exactly this
document, and because `kani-driver` carries a TODO pointing at it:

```rust
// TODO: Record processed items and dump them into a JSON file
// <https://github.com/model-checking/kani/issues/942>
```

**Kani already has a first-party consumer that needs this, and it is fragile by construction.**
`tools/benchcomp/benchcomp/parsers/kani_perf.py` regex-scrapes Kani's own stdout for
`Runtime Solver`, `Runtime Symex` and `Generated N VCC(s)`. Every other consumer is the same: greps
for `VERIFICATION:- `, `** N of M cover properties satisfied`, `Verification Time:`. Those strings
are printing details, not an interface; when they change, a consumer's grep silently matches
nothing. The failure mode is silence, not an error.

Who this helps:

- **CI gates and dashboards:** decide pass/fail, and detect regressions, without parsing prose.
- **Standard-library and large-scale verification efforts**, where hundreds of harnesses run and the
  question is not "did the build pass" but "what is proven, and did any proof stop meaning anything".
- **Commercial and industrial pipelines**, where a verifier must report cost and outcome to systems it
  did not author. Without that, a verifier cannot be used in an automated pipeline at all.
- **Kani's own tooling**, which can stop scraping its own output.

### The specific gap: a proof can pass while proving nothing

A contradictory `kani::assume` makes every subsequent assertion unreachable. The harness reports
`VERIFICATION:- SUCCESSFUL` and exits 0, even for an assertion as obviously false as `assert!(x !=
x)`. Kani's *text* output says so (`** 0 of 2 failed (2 unreachable)`), but a program checking the
exit code — what automated consumers do — cannot tell that proof from a real one.

Kani already computes what is needed to say this: reachability checks are generated per assertion and
on by default, `Property.reach` is populated, and `update_properties_with_reach_status` already
demotes a `Success` to `Unreachable` when the result cannot be trusted. **That reasoning lives in the
presentation layer and reaches no machine-readable output.** This RFC's core proposal is to carry it
in the results file.

**Downside.** A schema is an interface, and an interface limits future change. That is why the flag is
unstable and carries an explicit version field: the shape can still be fixed while real consumers are
teaching us what it should be.

### Relationship to the shipped implementation (#4472)

[PR #4472](https://github.com/model-checking/kani/pull/4472) merged on 2026-08-12 and ships
`--export-json` on `main`, behind `-Z unstable-options`; the design discussion there shaped this
proposal. The capability exists today; what this RFC settles is the contract for it.

The shipped document differs from the schema here in shape and vocabulary: it exports count summaries
plus per-harness detail arrays correlated by `harness_id == pretty_name`, serializes status values in
Rust `Debug` casing, and gates on `-Z unstable-options` rather than a dedicated ident. Whether this
schema supersedes the shipped shape, and what migrates, is the first open question below. This RFC is
numbered `0015`, per review; #4472 merged without an RFC file, so the number is free.

## User Experience

```
cargo kani -Z export-json --export-json results.json
```

(This shows the dedicated gate this RFC proposes; the shipped flag gates on `-Z unstable-options`
today — see "On the `-Z` gate" below.)

The flag is additive: existing rendered output is unchanged and the file is written in addition to
it; omitting the flag changes nothing. One combination is rejected outright: `--output-format=old`
bypasses CBMC's structured JSON entirely (`run_terminal_timeout` mocks a success/failure result with
zero properties and treats a timeout as success), so `--export-json` under it would produce a
well-formed file indistinguishable from a real clean run, even for a run that timed out. The argument
parser rejects the combination with an explicit error before verification starts.

### Interaction with other flags

`--export-json` has a defined answer for every flag `--sarif` guards, and for the flags that change
*what* or *how many* results exist:

- **`--only-codegen`:** rejected, like `--sarif`: there are no verification results to export.
- **`--jobs N` / concurrent harnesses:** allowed. `harnesses[]` is written sorted by `(crate_name,
  file, line, name)`, not completion order, so ordering is stable and a CI job can diff exports across
  commits without spurious reordering. Volatile fields (timestamps, wall and verification times) still
  differ between runs; determinism is a promise about *structure and ordering*, not byte-identity.
- **`--output-into-files`:** independent. It controls where the *rendered text* goes; `--export-json`
  writes its own file regardless, and the two do not interact.
- **`cargo kani` over a multi-crate workspace:** one file per Kani invocation, covering every harness
  in the run; each harness carries its `crate_name`, so same-named harnesses in different crates stay
  distinct. It is not written once per crate, and a later crate does not overwrite an earlier one.
- **A path of `-`:** *not* special-cased to mean stdout; treated as a literal filename. Streaming to
  stdout is deferred as a future option (it conflicts with the atomic-write contract below — no target
  to `rename` onto).
- **A path that already exists as a directory:** this RFC proposes rejecting it at argument-parse
  time, as `--sarif` does. The shipped implementation instead fails later, when the write is
  attempted; see the write-behaviour notes below.

**On the flag's name.** `--export-json` is a verb where other artifact flags are nouns (`--sarif`), and
does not say *what* is exported; `--results-json` would read better and age better. This RFC keeps
`--export-json` to match the proof-of-concept and issue #942, and treats the name as an open decision.

**On the `-Z` gate.** As shipped, the flag gates on the generic `-Z unstable-options`. This RFC
proposes a dedicated `-Z export-json` identifier instead, matching the per-feature gating pattern
(`--coverage` gates on `SourceCoverage`), so this artifact can stabilize or be dropped independently
of unrelated unstable options. The switch is a one-line gate change, part of the migration in the
first open question.

### Example

The output below is **the proposed 0015 document**, for the vacuity case the motivation describes:
two ordinary assertions made unreachable by one contradictory `kani::assume`, so Kani's text output
reads `** 0 of 2 failed (2 unreachable)`. That text output, and the harness and its properties, are
verbatim from a real proof-of-concept run at commit `7b125f1b47e36ca4cc50c4041abeca01912f80f9`. **The
JSON is not** a verbatim dump of that commit's `--export-json` output: the PoC emits flat
`kani_version`/`cbmc_version` fields (not the grouped `tools` object) and a boolean `run_complete`
(not `run_state`), and lacks `is_bounded`, `configuration.coverage_enabled`, `tools.solvers[].source`,
and `summary`, all proposed additions covered in their own sections below. Treat the JSON as an
instance of the *proposed* schema against the PoC's real vacuity run, not a capture of what the PoC
prints today.

```rust
#[kani::proof]
fn check_contradictory_assume() {
    let x: u8 = kani::any();
    kani::assume(x > 10 && x < 5); // contradictory: never true
    assert!(x < 5);
    assert!(x > 10);
}
```

```
kani-driver src/main.rs -Z export-json --export-json out.json
```

```json
{
  "schema_version": "0.1.0",
  "kani_commit": "7b125f1b47e36ca4cc50c4041abeca01912f80f9",
  "kani_commit_dirty": false,
  "tools": {
    "kani": "0.67.0",
    "rustc": "1.98.0-nightly (14210df0e 2026-05-31)",
    "cbmc": "6.10.0 (cbmc-6.10.0)",
    "goto_cc": "6.10.0 (cbmc-6.10.0)",
    "goto_instrument": "6.10.0 (cbmc-6.10.0)",
    "solvers": [{ "name": "cadical", "version": null, "source": "builtin" }]
  },
  "machine": {
    "cpu_count": 16,
    "total_memory_bytes": 32159113216,
    "memory_limit_bytes": null,
    "os": "linux",
    "arch": "x86_64"
  },
  "enabled_unstable_features": ["export-json"],
  "harness_selection": {
    "requested_filters": [],
    "exact": false,
    "unmatched_filters": [],
    "matched_count": 1
  },
  "harness_timeout_s": null,
  "configuration": {
    "checks": {
      "memory_safety": true,
      "overflow": true,
      "unwinding": true,
      "undefined_function": true,
      "assertion_reach_checks": true,
      "ignore_global_asm": false,
      "extra_pointer_checks": false
    },
    "coverage_enabled": false,
    "cbmc_args": []
  },
  "outcome": { "kind": "COMPLETED" },
  "run_state": "COMPLETE",
  "target": "x86_64-unknown-linux-gnu",
  "started_at": "2026-08-07T06:53:13Z",
  "wall_time_s": 0.04466111,
  "harnesses": [
    {
      "name": "check_contradictory_assume",
      "crate_name": "main",
      "file": "src/main.rs",
      "line": 2,
      "contract": null,
      "is_automatically_generated": false,
      "has_loop_contracts": false,
      "is_bounded": false,
      "attributes": {
        "kind": "Proof",
        "should_panic": false,
        "solver": null,
        "unwind_value": null,
        "stubs": [],
        "verified_stubs": []
      },
      "outcome": { "kind": "COMPLETED", "verdict": "SUCCESS" },
      "resolved_solver": "cadical",
      "resolved_unwind": null,
      "generated_concrete_test": false,
      "resources": {
        "verification_time_s": 0.010809311
      },
      "n_properties": 2,
      "n_failed": 0,
      "failure_kind": "NONE",
      "failed_properties": [],
      "unsupported_constructs": [],
      "warnings": [],
      "warnings_truncated": 0,
      "checks": {
        "total": 2,
        "success": 0,
        "failure": [],
        "unreachable": [
          "check_contradictory_assume.assertion.1",
          "check_contradictory_assume.assertion.2"
        ],
        "undetermined": [],
        "error": [],
        "unknown": [],
        "other": []
      },
      "covers": {
        "total": 0,
        "satisfied": [],
        "unsatisfiable": [],
        "unreachable": [],
        "undetermined": [],
        "error": [],
        "unknown": [],
        "other": []
      }
    }
  ],
  "summary": {
    "total": 1,
    "successful": 1,
    "failed": 0,
    "checks_total": 2,
    "checks_success": 0,
    "covers_total": 0,
    "covers_satisfied": 0
  }
}
```

The vacuity case, machine-readable: `outcome.verdict` is `SUCCESS` and the exit code `0`, exactly like a
real proof, yet `checks.unreachable` names both properties that could not be exercised — so a consumer
no longer trusts the exit code alone.

### Reading the results

Every field the example shows — its type, whether it may be `null`, and its meaning — is specified
in the **Normative schema reference** appendix, along with the presence matrices (marker versus
terminal document, and per-harness by `outcome.kind`) and the closed value domains. The rest of this
section covers what a reader needs to understand the design: how a consumer detects a vacuous pass,
which fields are mandatory and why, and the completeness contract that says when the file can be
trusted.

**The harness name (`name`) is already the re-runnable selector.** `name` is the harness's
`pretty_name`: the fully qualified name the user gave the function, module path included (e.g.
`sequence::proofs::no_over_read`; for a crate-root harness it equals the bare function name). This is
exactly the string `--harness --exact` accepts to re-run the harness; it *is* the selector, not a local
identifier needing a separate "selector" field. Three things a consumer must still get right:

- **`name` is unique within a crate, not across a workspace.** Two crates in one `cargo kani` run can
  each define a harness with the same fully qualified `name`. Pair `name` with `crate_name` as the join
  key, and re-run with `-p <crate_name> --harness --exact <name>`.
- **`crate_name` is not always what `-p` wants.** `crate_name` is the *rustc* crate identifier (always
  underscored); `cargo -p` takes the *Cargo package* name, which `Cargo.toml` sets and may contain
  hyphens (`my-crate` → crate `my_crate`). This schema exports only the rustc name, so `-p <crate_name>`
  round-trips only when the two coincide; otherwise map `crate_name` back to its `Cargo.toml` package
  name first.
- **Plain `--harness` is substring-matching; only `--exact` makes `name` round-trip.** Without
  `--exact`, `--harness <name>` can match more than the one harness (see `harness_selection.exact`). A
  consumer reproducing *exactly* this result must pass `--exact`.

**The bounded-result flag (`is_bounded`).** Mandatory on every harness: a bounded result read as
unrestricted is the over-claim this schema exists to prevent. `true` exactly when `kani autoharness`
generated the harness *and* bound at least one argument to a *bounded* nondeterministic value (because
`--autoharness-bounded-arguments` was passed and the argument's type had no unbounded `Arbitrary`
strategy); then `outcome.verdict == "SUCCESS"` proves the target only for inputs within that bound. A
manually written harness, and an autoharness one that needed no bounded argument, both report `false`;
never omitted, so a consumer need not branch on `is_automatically_generated` first. Orthogonal to
`is_ctor_based` (open below) — only `is_bounded` changes whether the result reads as an unrestricted proof.

The consumer applies two named predicates (all fields per-harness: `harnesses[].outcome.verdict`,
`harnesses[].checks.*`, abbreviated), both guarded on `verdict == "SUCCESS"` and both *inapplicable*
when `configuration.checks.assertion_reach_checks` is `false`:

- **Vacuous pass (normative).** `verdict == "SUCCESS" && checks.total > 0 && checks.unreachable.len() ==
  checks.total`: every check in a passing harness is unreachable, so the proof examined nothing.
  (`success == 0` would be unsound — a passing `#[kani::should_panic]` harness has a `FAILURE` panic
  check; `checks.total > 0` excludes a checkless harness; the `SUCCESS` guard excludes failing ones.)
- **Vacuity-suspect (advisory).** `verdict == "SUCCESS" && !checks.unreachable.is_empty()`: a passing
  harness with *any* unreachable check, catching *partial* vacuity the normative rule misses. Advisory
  because noisier (a deliberately dead assertion trips it). (Reads only `checks.unreachable`; an
  unreachable *cover* lands in `covers.unreachable` and does not trip it.)

Both rules are sound only when `configuration.checks.assertion_reach_checks` is `true`: with reach-checks
off, an assertion made unreachable by a contradictory `kani::assume` reports as `SUCCESS` and inflates
`checks.success` instead of landing in `checks.unreachable`, so a consumer must treat
`assertion_reach_checks == false` as "this run cannot surface vacuity through `checks.unreachable`".

**`configuration.coverage_enabled` is mandatory too.** Like `is_bounded`, it is always present, never
omitted. It mirrors `--coverage` and is the flag that makes `code_coverage` (`COVERED`/`UNCOVERED`)
properties exist — the ones left out of `checks`, `covers`, and `n_properties`. Without it a consumer
cannot tell "no coverage properties in this schema yet" from "`--coverage` was never passed." Its
full entry, and the general rule for what belongs in `configuration`, are in the appendix.

**The completeness contract.** This proposal replaces the proof-of-concept's boolean `run_complete`
with a richer **`run_state`** field and anchors trust on it, not on file existence. `run_state` is
proposed, not yet implemented; choosing between this marker design and delete-up-front is an open
question below. The write is atomic: Kani writes the full JSON to a temp file in the target's directory,
then `rename`s it onto the target (a same-filesystem POSIX rename is atomic), so the target never holds
a partial write and a mid-write kill leaves the previous file in place (at worst an orphaned temp file
beside it, never mistaken for the target).

Once harness verification begins, Kani writes an atomic **marker**: the pre-verification fields it
already knows (schema/tool versions, `machine`, `configuration`, `harness_selection`) plus
`run_state: "INCOMPLETE"`, and *not* `summary` or `harnesses[]`, so a consumer reading the marker sees
no verdict to trust. Writing it invalidates any stale earlier file and records that a run started. On
normal completion the terminal write sets `run_state` to one of three **terminal, successfully-published**
values: `COMPLETE` (every selected harness produced a result), `PARTIAL` (some did not, e.g. a
`--fail-fast` run aborted after the first failure), or `NO_HARNESSES_SELECTED` (nothing was selected —
see "`NO_HARNESSES_SELECTED` versus a crate with no harnesses at all" below).

A consumer reads in that order: first confirm `schema_version` is supported (see Compatibility policy —
on an unrecognized major, or, pre-1.0, an unrecognized minor, refuse to parse), *then* read `run_state`:

- **`run_state == "COMPLETE"`** is the only value a consumer may treat as *complete verification
  evidence*. It is stronger than "a file exists": a `--fail-fast` run that skipped harnesses, or a
  zero-match filter that publishes a well-formed `successful: 0, failed: 0` document, each leaves an
  existing, parseable, clean-*looking* file, yet neither is `COMPLETE`.
- **`PARTIAL` / `NO_HARNESSES_SELECTED`** are *finished* exports, trustworthy for what they say (a
  partial run, or an empty selection); they simply must not be read as a complete run.
- **`INCOMPLETE`, or a missing file,** means the export did not finish: an export failure (unwritable
  path, full disk) or abnormal termination (OOM-kill, Ctrl-C) after the marker was written.

Two limits. First, the marker is written only *once verification begins*, after the crate is built, so a
failure *before* that (a compilation error, a rejected `--exact --harness` filter) does not write it and
leaves any pre-existing file untouched; existence-based staleness is defeated only from the marker
onward. Second, a legacy consumer that checks only file *presence* and never reads `run_state` finds the
marker and fails *open* on a verdict-less file, where delete-up-front failed *closed* on `ENOENT`. The
contract assumes a single writer per path: concurrent runs aiming at the same file are unsupported, last
rename wins.

The exact field presence in the marker versus a terminal document, which `run_state` values pair with
which `outcome`, and the per-harness field presence by `outcome.kind` are all tabulated in the
Normative schema reference appendix.

## Rationale and alternatives

### Why not extend `--sarif`?

Kani already emits SARIF, stable and *not* `-Z` gated, so the case for a second artifact rests on
consumer contracts, not expressive power. SARIF *could* carry this data (SARIF 2.1.0 has a rich
`result.kind` and arbitrary property bags; Kani's writer skipping covers and successes is a choice in
*our* writer, not a format limit), but the reasons to keep it separate are about who consumes each file
and how free each is to change:

- **`--sarif` is stable and must stay valid SARIF; this artifact must be free to break shape while `-Z`
  gated.** The load-bearing reason: merging them would bind a stable, standard-conformant surface to an
  unstable one's churn.
- **Opposite consumer needs.** Code-scanning tools reading `--sarif` do *not* want proof/cover/vacuity
  rows — a green run *should* be an empty `results` array; the CI gates and dashboards reading this
  artifact want exactly those rows. Adding them degrades SARIF for its consumers.
- **Vacuity has no native SARIF representation**, only a property bag — a private schema hidden inside a
  standard one, which helps neither file's consumers.

This proposal does not change `--sarif`. **How the two avoid drifting apart:** both render from the *same
upstream* per-harness `VerificationResult` (its parsed `Vec<Property>` from `cbmc_output_parser`), not a
shared results object; only the projection differs (forcing both through one intermediate object would
recreate the stable-to-unstable coupling above). Shared *interpretation* of the properties (path
relativization, solver/unwind resolution, failed-property classification) should be shared leaf logic,
with a cross-artifact conformance test — a development discipline, since the design shares only the
*inputs*.

### Summary (`summary.*`)

All seven `summary.*` fields are run-level totals over `harnesses[]`, always present and integer in a
terminal document, never `null` there — `summary` is absent entirely in the `INCOMPLETE` marker, so
"always present" is scoped to a shape that has a `summary`:

| Field | Definition |
|---|---|
| `total` | `harnesses.len()`. |
| `successful` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "SUCCESS"`. |
| `failed` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "FAILURE"`. |
| `checks_total` | Sum of `harnesses[].checks.total` over `COMPLETED` harnesses only. |
| `checks_success` | Sum of `harnesses[].checks.success` over `COMPLETED` harnesses only. |
| `covers_total` | Sum of `harnesses[].covers.total` over `COMPLETED` harnesses only. |
| `covers_satisfied` | Sum of `len(harnesses[].covers.satisfied)` over `COMPLETED` harnesses only. |

**`total == harnesses.len()`** in any document where `summary` is present (any terminal document,
whichever of the three `run_state` values): `summary` describes exactly the harnesses this document
reports on, not those selected (that comparison is `harness_selection.matched_count` vs `summary.total`,
next). The `INCOMPLETE` marker has no `summary`, so the identity is vacuous there, not violated.

**How a non-`COMPLETED` harness counts.** `successful` and `failed` both require `outcome.kind ==
"COMPLETED"`, so a `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` harness is counted in neither: it has no
`outcome.verdict`, and counting it as a failure would conflate "CBMC found a bug" with "CBMC never got
to look." So `successful + failed <= total`, not `==`; the gap is exactly the count of non-`COMPLETED`
harnesses, and equality holds iff every harness reached `COMPLETED`. The four `checks_*`/`covers_*` sums
follow the same rule: a non-`COMPLETED` harness's `checks.total`/`covers.total` is `null`, so it
contributes nothing rather than `0` — "not counted" versus "counted as zero."

**The `matched_count` / `total` / `run_state` invariant.** `harness_selection.matched_count` is the
*pre-verification* match count; `summary.total` is the *post-verification* harness count; `run_state`
says how the two relate:

- `run_state == "COMPLETE"` ⟹ `summary.total == harness_selection.matched_count`: every matched harness
  produced an entry in `harnesses[]` (with whatever `outcome.kind` it reached — `COMPLETE` does not mean
  every harness passed, only that every one was accounted for).
- `run_state == "PARTIAL"` ⟹ `summary.total < harness_selection.matched_count`: some matched harnesses
  have no entry at all, because Kani stopped before running them (e.g. `--fail-fast` after the first
  failure) — missing from the array, not present with a `TIMEOUT`/`CRASHED` placeholder.
- `run_state == "NO_HARNESSES_SELECTED"` ⟹ `harness_selection.matched_count == 0 == summary.total`.
- `run_state == "INCOMPLETE"` (the marker): `summary` is absent entirely (see the presence matrix
  in the Normative schema reference appendix), so this invariant does not apply.

### Compatibility policy

`schema_version` is a semantic version. It exists so the shape can be corrected while it is still being
learned from real consumers, and it makes the rules of that correction explicit rather than implied:

- **Minor (backward-compatible) changes** bump the minor: adding a field, or adding a value to one of
  the **explicitly open** vocabularies named in "Value domains" (today, just the solver names:
  `attributes.solver`, `resolved_solver`, `tools.solvers[].name`). A consumer of an open vocabulary
  already treats an unrecognized value as "valid, but unfamiliar", so it keeps working across such a
  bump. Every other enum is closed (next bullet); this allowance does not apply to them.
- **Major (breaking) changes** bump the major: renaming or removing a field, changing an existing field's
  meaning or type, adding a variant to a *closed* enum (`outcome.kind`, `verdict`, `failure_kind`,
  `run_state`, `status`, `attributes.kind`), or moving a status out of the `other` bucket into a named
  one. Adding a closed-enum variant is a deliberately high bar (a new `failure_kind` waits for a major
  bump); consumers are nonetheless encouraged to keep a default arm even against closed sets.
- **Forward-compatibility rule for consumers: ignore unknown fields.** This is the entire mechanism by
  which the schema grows within a major version; a consumer that rejects unknown fields forfeits it.
- **On an unknown major version, refuse to parse** rather than guess: a major bump means a relied-on
  field may have changed meaning, the silent misread this schema exists to remove.
- **`warnings` is outside these guarantees** (see its own section): its presence and shape are
  contractual; its contents, wording and size are not.

**This schema is deliberately open to growing richer, not frozen at v1.** Today's fields are the honest
core this RFC could verify against the shipped writer, not a ceiling. The additive minor-version rule is
the growth mechanism: new fields land one minor at a time as real downstream consumers surface needs v1
does not yet meet, rather than a redesign or a competing artifact. Nothing here asserts today's fields
are all a consumer will ever need.

**Semver-zero, while it is `-Z` gated.** Until stabilization the schema is `0.x`, where the minor/major
distinction above is *intent*, not a promise: a breaking change may land on a `0.x` minor bump, so a
`0.x` consumer must assert an exact minor (e.g. `== "0.1.0"`, or a small verified allow-list) and refuse
any other `0.x` value like an unknown major, rather than rely on forward-compatibility. The
backward-compatibility guarantee takes effect at `1.0`. This is why `schema_version` is checked first.

**What must be true before this leaves `-Z`** (the RFC process requires all open questions to be
resolved before stabilization; see `rfc/src/template.md`):

1. Every open question below is resolved, in particular the processed-vs-raw view, not merely narrowed.
2. At least one release cycle of feedback from a real out-of-tree consumer, and migration of Kani's own
   `benchcomp` parser onto this artifact as the first in-tree consumer.
3. The JSON-Schema-document question (the `schemars` decision below) is settled either way, not left
   open.

### Why a new artifact rather than extending an existing one?

`kani list --format json` is pre-verification metadata; coverage output is per-region data;
`--output-into-files` writes the same rendered text, split per harness; `.kani-metadata.json` is the
compiler-to-driver channel. None carries verification outcomes.

### Why is CBMC statistics data excluded?

Symex time, VCC counts and solver time exist only inside CBMC's free-text messages; extracting them means
pattern-matching human-readable output — the fragility this RFC exists to remove. `benchcomp` does
exactly this today; the right fix is structured data from CBMC, not more scraping. The `warnings` field
does carry CBMC's messages verbatim, as non-contractual free text: CBMC's `--json-ui` stream tags each
message with a `messageType` (e.g. `"WARNING"`), and this schema surfaces the `WARNING`-typed ones — for
instance the SAT backend's `warning: ignoring forall`, emitted when a `kani::forall!`/`kani::exists!`
over a symbolic bound cannot be discharged ([PR #4719](https://github.com/model-checking/kani/pull/4719)
surfaces this same warning). These strings can run to several kilobytes with no promised structure and
are not parsed by Kani; a consumer treats each as an opaque string to display, not a field to
pattern-match. Two consequences:

- **`warnings` is outside the `schema_version` compatibility guarantees.** The field's presence and its
  `[{ "message": …, "truncated": …, "original_chars": … }]` shape are contractual; the *contents* of
  `message` are not, and must not be pattern-matched or assumed stable across versions.
- **`warnings` is bounded, with explicit, structural truncation markers.** One CBMC warning already runs
  to ~11.6 KB and standard-library-scale runs can emit many; left unbounded the field could dwarf the
  file and defeat the CI consumers it serves. So each `message` is capped at a fixed,
  implementation-defined length and truncated on a character boundary, with the fact and extent carried
  as siblings `truncated: bool` and `original_chars: integer | null` (pre-truncation count, `null`
  exactly when `truncated` is `false`) rather than a suffix inside `message`. The per-harness `warnings`
  array is separately capped at a fixed count, with a sibling `warnings_truncated` integer per harness
  (`0` when nothing was dropped) recording how many *entries* were omitted. A consumer can always tell
  "no more warnings" from "we stopped recording". These structural fields are schema-versioned; the
  `message` text is not.

### What if we do nothing?

Consumers keep scraping, including Kani's own. The status quo works until an output string changes,
and then the breakage is silent: a grep that matches nothing looks exactly like a run with nothing
to report. The vacuity problem in particular stays invisible to automation. Doing nothing now has a
second cost: the shipped v1 document becomes the de-facto contract, unversioned and unspecified.

## Open questions

- **Does this schema supersede the shipped v1 shape?** #4472's document (count summaries and per-harness
  detail arrays joined on `pretty_name`, `Debug`-cased status values, gated `-Z unstable-options`) is on
  `main` today and is not this shape. The flag is unstable, so the shape can change without a deprecation
  cycle. The question: migrate the shipped writer to this schema now, or redraw this RFC around the
  shipped shape? The sections above assume the former.
- **Processed or raw view?** The property list Kani renders is already post-processed: reachability
  checks removed, some descriptions rewritten, successful checks demoted on a fundamental failure.
  This proposal exports the **processed** view, so the file agrees with Kani's exit code and text
  output. Should the raw CBMC view also be available, or is that CBMC's `--json-ui` to provide?
- Should a JSON Schema document ship alongside? That likely means a `schemars` dependency, not currently
  in the workspace — a real dependency decision, not something to add quietly.
- **Completeness: `run_state` marker, or delete the target up front?** This proposal writes a marker.
  Deleting up front destroys a user-named path before verification starts, and fails *closed* on `ENOENT`
  for a consumer that only checks existence; the marker fails *open*. The window is narrow: a stale
  `COMPLETE` can only be read during the pre-marker **build** window — once verification begins, the
  marker write invalidates the stale file immediately. Both become one-way doors as soon as anything
  consumes the file, which is why this is asked rather than decided.
- Should this cover the `autoharness` subcommand's results, whose `chosen`/`skipped` classification
  currently has no machine-readable form?
- Coverage results: include here, or leave with `kani-cov`?
- **Shipped fields dropped or transformed in this shape — keep, drop, or make mandatory?** #4472's
  document (`create_metadata_json`, `create_harness_metadata_json`, `process_cbmc_results` in
  `kani-driver/src/frontend/schema_utils.rs`) carries a number of fields this schema does not restore
  as-is. Grouped by disposition:
  - **Dropped, no successor field today:** `build_mode` (`"debug"`/`"release"`); `harnesses[].mangled_name`
    (see "Why not `mangled_name`" in the Normative schema reference appendix); the source range's `end_line` (only `original_start_line`
    survives, as `harnesses[].line`); `goto_file` (the generated modeling file path); per-check
    `description`/`location` for every property, not just the failed and `other`-bucketed ones (the
    shipped shape records full detail for every check; this schema records identities only for
    `success`/`unreachable`/`undetermined`/`error`/`unknown`, and full records only for
    `failed_properties`/`unsupported_constructs`/`other[]`); CBMC's OS banner (`cbmc_metadata.os_info`,
    distinct from the run-level `machine.os`, which is Kani's host OS); and CBMC execution statistics
    (`cbmc_stats`: symex time, VCC counts, solver time — see "Why is CBMC statistics data excluded?" and
    the "Per-check timing" future-work item).
  - **Dropped, and soundness-relevant — now `harnesses[].is_bounded`, a *mandatory* field:** resolved
    above, in "The bounded-result flag (`is_bounded`)"; no longer an open question.
  - **The effective `--object-bits` value stays an open question.** `effective_object_bits` is
    soundness-adjacent like `is_bounded` (an object-bits limit narrows what a `SUCCESS` verdict covers)
    but is not promoted: it is a per-run CBMC argument (`VerificationArgs::cbmc_object_bits`), not a
    per-harness attribute, so it has no per-harness home and harnesses in one run cannot be told apart by
    their bound. On triage (model-checking/kani#4731) a maintainer (feliperodri) classed it niche relative
    to `is_bounded`. Leaving it dropped is a real gap: adopting this RFC should file (or update) a tracking
    issue for a per-harness `object_bits` home, resolved before or as part of stabilization.
  - **`coverage.enabled` is resolved — now `configuration.coverage_enabled`, a *mandatory* field:** see
    the key-decision note above and its full entry in the appendix; no longer open. (Dropping the one flag that explains why `code_coverage` properties
    are invisible here, while keeping three others for the identical reason, is why it stopped being
    optional.)
  - **The Cargo provenance `project.workspace_root` / `output_dir`, and `harnesses[].file`'s base
    directory.** `harnesses[].file` is written relative to the invocation directory
    (`std::env::current_dir()`), not any field this document carries; for a `cargo kani` run from the
    workspace root the two coincide, but for a standalone `kani-driver` invocation or a script that `cd`s
    first they need not. Restoring `workspace_root` (or defining `file` as workspace-relative) makes
    `file` resolvable by a consumer that does not trust the launch directory. `output_dir` (the
    `target/<triple>/...` directory) has no comparable argument and can stay dropped.
  - **`is_ctor_based`** (the autoharness constructor-args flag) is confirmed present in the shipped writer
    and `HarnessMetadata`. Unlike `is_bounded` it stays dropped-but-open, not promoted: cheap to restore
    but not soundness-mandatory, since a constructor-based strategy narrows which *values* are covered
    rather than mislabeling an unrestricted result.
- **Final flag name.** `--export-json` versus `--results-json` (see the flag-name note under User
  Experience). The CLI spelling is provisional and this is a stabilization-blocking decision: the name
  must be settled before the flag leaves `-Z`.

(The earlier open question "which other flags belong in `configuration`?" is now resolved as the
**Policy for `configuration`** in the Normative schema reference appendix: a flag is recorded when it changes which properties are generated
or what a status means, with `cbmc_args` as the verbatim catch-all.)

## Out of scope / Future Improvements

The schema deliberately leaves room for these; none is proposed now.

- **Per-check timing and resource data.** Precedented elsewhere (GNATprove reports per-obligation
  prover data), but not available from CBMC's structured output today.
- **A machine-reproducible counterexample.** `--concrete-playback` already derives a concrete value
  vector; exposing it would let a consumer reproduce a failure without modifying source.
- **A finer failure classification.** Kani computes whether a failure involved unwinding assertions
  or reachable undefined functions, then collapses them; separating them would let a consumer tell
  "raise `--unwind` and retry" from "fix the code".
- **A harness-level triviality signal**, aggregating what RFC 0003 already identifies as the
  vacuity concern.
- **Aggregate coverage**, deferring to `kani-cov` and RFC 0011.
- **The contract trust chain.** A harness using `stub_verified` is sound only if the contract's own
  proof passed. Kani already enforces at compile time that such a harness exists; because this schema
  carries both edges, a consumer can check the run-time half itself today, and Kani could surface the
  resolved status later.
- **Per-harness peak memory.** A `getrusage(RUSAGE_CHILDREN)`-based approach was prototyped and rejected:
  `ru_maxrss` is a process-wide running *maximum*, not a per-child figure, so the result is
  order-dependent (only a harness that out-peaks every predecessor gets a value; a later, lighter one
  reads `null` even under real pressure) and is not attempted under `--jobs`, where the counter is shared
  across siblings. An accurate figure needs per-child accounting (e.g. `wait4()`-based rusage per child,
  or a per-child cgroup with its own `memory.peak`), which this schema does not attempt; CI consumers
  infer OOM from exit code 137 (visible via `outcome.kind == "OUT_OF_MEMORY"`).

## Normative schema reference

This section is the normative field-level contract; the body above is the design.

### Field reference

Every field the example shows, normatively. `null` always means *not measured or not applicable* (see
"Value domains"); "—" in the Null? column means the field is never absent and never `null` **in a
terminal document** (`COMPLETE` / `PARTIAL` / `NO_HARNESSES_SELECTED`). The `INCOMPLETE` marker is a
separate, narrower shape governed by its own presence matrix below: several fields marked "—" here
(`outcome`, `wall_time_s`, `summary`, `harnesses[]`) are absent from the marker entirely.

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `schema_version` | string (semver) | — | This document's own version; see Compatibility policy. |
| `kani_commit` | string | nullable | Git commit Kani was built from; `null` outside a git checkout. |
| `kani_commit_dirty` | bool | nullable | Whether the build tree had uncommitted changes; `null` exactly when `kani_commit` is `null`. |
| `tools.kani` | string | — | The Kani release that produced this document (`env!("CARGO_PKG_VERSION")` at build time). Unlike every other `tools.*` entry, this one is never `null`: Kani always knows its own version. |
| `tools.rustc` | string | nullable | rustc toolchain `kani-compiler` was built against; `null` if the probe failed. |
| `tools.cbmc` / `.goto_cc` / `.goto_instrument` | string | nullable | Each tool's own `--version` output; `null` if it could not be probed. |
| `tools.goto_synthesizer` | string | nullable, key conditional | Present only when the run *requested* loop-contract synthesis (`--synthesize-loop-contracts`), regardless of whether synthesis subsequently succeeded; see "Tool provenance" below. |
| `tools.solvers[]` | array of `{name, version, source}` | array never null, may be empty | Deduplicated, name-sorted; see "Tool provenance" below. |
| `machine.cpu_count` | integer | nullable | Logical CPUs available; `null` if undetermined. |
| `machine.total_memory_bytes` | integer | nullable | Total system RAM; Linux only today. |
| `machine.memory_limit_bytes` | integer | nullable | The enforced ceiling (cgroup/ulimit), not installed RAM; `null` when unlimited or unreadable. |
| `machine.os` / `.arch` | string | — | e.g. `"linux"` / `"x86_64"`. |
| `enabled_unstable_features` | array of strings | never null, may be empty | Sorted `-Z` flags active for this run. |
| `harness_selection.requested_filters` | array of strings | never null, may be empty | Raw `--harness` values; empty means no filter. |
| `harness_selection.exact` | bool | — | Whether `--exact` was passed. |
| `harness_selection.unmatched_filters` | array of strings | never null, may be empty | Filters that matched nothing; only populated without `--exact`. |
| `harness_selection.matched_count` | integer | — | Pre-verification match count; compare against `summary.total`/`run_state`. |
| `harness_timeout_s` | number | nullable | `--harness-timeout` value in seconds; `null` when unset. |
| `configuration.checks.*` (7 bools) | bool | — each | See the `configuration.checks.*` entries below and the "Policy for `configuration`" section. |
| `configuration.coverage_enabled` | bool | — | Whether `--coverage` was passed; see the `configuration.coverage_enabled` entry below. |
| `configuration.cbmc_args` | array of strings | never null, may be empty | Verbatim (UTF-8-lossy) `--cbmc-args`; a comparability signal, not a replayable argv. |
| `outcome.kind` (run level) | `"COMPLETED"` | — | Always `COMPLETED` in a terminal document; absent in the `INCOMPLETE` marker. There is no run-level `"CRASHED"` value: see "How `run_state` and `outcome.kind` co-occur" below for why the writer can never produce one. |
| `run_state` | `"INCOMPLETE"` \| `"COMPLETE"` \| `"PARTIAL"` \| `"NO_HARNESSES_SELECTED"` | — | See "The completeness contract" below; this is the trust anchor, not `outcome.kind`. |
| `target` | string | — | The Rust target triple Kani itself was built for. |
| `started_at` | string | — | UTC, `YYYY-MM-DDTHH:MM:SSZ` (second resolution). |
| `wall_time_s` | number | — | Seconds; volatile between runs by design (see "Interaction with other flags"). |
| `harnesses[]` | array of harness objects | never null, may be empty (e.g. under `NO_HARNESSES_SELECTED`) | Sorted by `(crate_name, file, line, name)`. |
| `harnesses[].name` | string | — | The harness's fully-qualified `pretty_name`; see "The harness name" below. |
| `harnesses[].crate_name` | string | — | Distinguishes same-named harnesses across crates in one workspace. |
| `harnesses[].file` | string | — | Path to the declaring file, relative to the directory Kani was invoked from (see the dropped-fields open question for the `workspace_root` connection). |
| `harnesses[].line` | integer | — | 1-based line the harness function begins on. The end line is not exported (see dropped fields). |
| `harnesses[].contract` | object | nullable | `null` when the harness carries no CBMC-level `assigns` contract. |
| `harnesses[].contract.contracted_function_name` | string | — (when `contract` present) | The contract's target function. |
| `harnesses[].contract.recursion_tracker` | string | nullable | Non-null only for a `#[kani::recursive]` function. |
| `harnesses[].is_automatically_generated` | bool | — | True for an `autoharness`-generated harness; see "Harnesses that are not `--harness`-selectable". |
| `harnesses[].has_loop_contracts` | bool | — | Whether the harness uses loop contracts. |
| `harnesses[].is_bounded` | bool | — | Mandatory (soundness-relevant, promoted out of the open questions); see "The bounded-result flag (`is_bounded`)" below. |
| `harnesses[].attributes.kind` | `"Proof"` \| `"Test"` \| `{"ProofForContract": {"target_fn": string}}` | — | See "Fields that are not always strings" below. |
| `harnesses[].attributes.should_panic` | bool | — | Whether `#[kani::should_panic]` is set. |
| `harnesses[].attributes.solver` | string \| `{"Binary": string}` | nullable | The *requested* solver attribute; `null` when unset (default resolution applies). Compare `resolved_solver`. |
| `harnesses[].attributes.unwind_value` | integer | nullable | `#[kani::unwind(N)]` value, if set. |
| `harnesses[].attributes.stubs[]` | array of `{original, replacement}` strings | never null, may be empty | Requested stubs. |
| `harnesses[].attributes.verified_stubs[]` | array of strings | never null, may be empty | Functions stubbed by their verified contract. |
| `harnesses[].outcome.kind` | `"COMPLETED"` \| `"TIMEOUT"` \| `"OUT_OF_MEMORY"` \| `"CRASHED"` | — | |
| `harnesses[].outcome.verdict` | `"SUCCESS"` \| `"FAILURE"` | present only on `COMPLETED`, never null there | The per-harness pass/fail call. |
| `harnesses[].outcome.code` | integer | nullable, present only on `CRASHED` | Process exit code, when known. Unlike the run level, this field is real: a per-harness crash still reaches the shared terminal write (see below), so it is actually producible. |
| `harnesses[].outcome.message` | string | nullable, present only on `CRASHED` | Free-text crash reason, scoped to this one harness (e.g. naming what CBMC crashed on); non-contractual wording. |
| `harnesses[].resolved_solver` | string | nullable | The solver CBMC actually runs with; `null` when CBMC chooses for itself (bare `--smt2`). Same spelling as `tools.solvers[].name`. |
| `harnesses[].resolved_unwind` | integer | nullable | The effective `--unwind` bound; `null` when none applies. |
| `harnesses[].generated_concrete_test` | bool | — | Whether `--concrete-playback` produced a test for this harness. |
| `harnesses[].resources.verification_time_s` | number | — | Present on every harness regardless of `outcome.kind`: on `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` it is the time elapsed until that outcome fired, not a verification duration. |
| `harnesses[].n_properties` | integer | nullable; `null` except on `COMPLETED` | `checks.total + covers.total`; excludes `code_coverage` properties. See the presence matrix below. |
| `harnesses[].n_failed` | integer | nullable; `null` except on `COMPLETED` | `failed_properties.len()`. See the presence matrix below. |
| `harnesses[].failure_kind` | `"NONE"` \| `"PANICS_ONLY"` \| `"OTHER"` \| `"ERROR"` | present only on `COMPLETED`, never null there | Raw failure classification; see the `failure_kind` truth table in "Value domains" below — **not** synonymous with `verdict != SUCCESS`. |
| `harnesses[].failed_properties[]` | array of property objects | never null, may be empty (empty on a non-`COMPLETED` harness) | Full shape below. |
| `harnesses[].unsupported_constructs[]` | array of property objects, same shape | never null, may be empty | Rust/MIR constructs Kani can't model; a reached one also appears in `failed_properties`. |
| `harnesses[].warnings[]` | array of `{message: string, truncated: bool, original_chars: integer\|null}` | never null, may be empty | See the `warnings` section; `message`'s contents are non-contractual, `truncated`/`original_chars` are structural and schema-versioned. |
| `harnesses[].warnings_truncated` | integer | — | Count of warnings dropped by the cap; `0` when nothing was dropped. Proposed, not yet in the PoC. |
| `harnesses[].checks` / `.covers` | objects | — (the object itself is never omitted) | Bucket shape below; see the presence matrix below for what the *fields inside* look like on a non-`COMPLETED` harness. |
| `summary.*` (7 integer fields) | integer | — each | Run-level totals; see the `Summary` discussion in "Rationale". |

**`failed_properties[]` / `unsupported_constructs[]` element shape** (a full property record):

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `id` | string | — | The real CBMC property id, in `<function>.<class>.<counter>` general form (two shorter forms exist: `<function>.<counter>` and `<class>.<counter>`; see below) — reconstructed to match CBMC's own id, not Kani's display rendering. Ordinals (`<counter>`) are **not** promised stable across source changes: adding or removing an assertion upstream of one can renumber it. |
| `description` | string | — | Free-form property description text. |
| `class` | string | — | The property's class (e.g. `"assertion"`, `"unsupported_construct"`). |
| `file` | string | nullable | Source file, when CBMC recorded a location. |
| `line` | string | nullable | Source line, **as a string**, not an integer — carried through verbatim from CBMC's own field (contrast `harnesses[].line`, which is an integer). |
| `trace_available` | bool | — | Whether a counterexample trace exists for this property. |
| `status` | closed `CheckStatus` value | — | |

**`checks.other[]` / `covers.other[]` element shape** (a bucketing record, not a full property):

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `id` | string | — | Same id format as above. |
| `status` | closed `CheckStatus` value | — | The actual, unbucketed status; see "the `other` bucket" in Value domains. |

`other[]` elements carry only `id` and `status`: the fuller shape is reserved for
`failed_properties[]`/`unsupported_constructs[]`. `other` exists so the partition stays exhaustive, not
to duplicate the full record for a rare, already-anomalous status.

**`checks{}` / `covers{}` bucket shape**, both objects:

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `total` | integer | nullable; `null` when the owning harness's `outcome.kind` is not `COMPLETED` | Every property this bucket accounts for. |
| `success` (checks only) | integer | nullable; `null` under the same condition as `total` | A **count**, not an identity list (see "Rationale" for why). |
| `satisfied` (covers only) | array of ids | never null, may be empty (empty, not absent, on a non-`COMPLETED` harness) | An identity list (covers are user-authored and few). |
| `failure` (checks) / `unsatisfiable` (covers) | array of ids | never null, may be empty | |
| `unreachable`, `undetermined`, `error`, `unknown` | array of ids, each | never null, may be empty | |
| `other[]` | array of `{id, status}` | never null, may be empty | See above. |

On a `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` harness, CBMC produced no property list to bucket: `total`
(and `success`, for `checks`) is `null` — never `0`, which would misread as "measured, none passed" —
and every identity-list field is `[]`, since there is nothing to name. The bucket-arithmetic invariant
below stays trivially consistent (`null` is not a number to sum) but says nothing about that harness
until it reaches `COMPLETED`.

The bucket-arithmetic invariant holds unconditionally **on a `COMPLETED` harness**: `checks.total ==
success + len(failure) + len(unreachable) + len(undetermined) + len(error) + len(unknown) +
len(other)`, and symmetrically for `covers` with `satisfied` for `success`. Combined with the
exclusion of `code_coverage` properties, this is why `n_properties == checks.total + covers.total`
holds as a result, not a separate promise (see "Rationale").

**Tool provenance (`tools`).** The `tools` object gives the version of every tool whose behaviour can
change a result — the machine-readable form of the versions Kani already prints, closing the gap in
[#2572](https://github.com/model-checking/kani/issues/2572).

`goto_synthesizer` is conditional: present only when the run requested `--synthesize-loop-contracts` (a
missing key, possible only here, means it was not requested). Every present key follows one rule: **a
version that cannot be determined is `null`, never guessed** — whether the probe binary is missing,
refuses `--version`, or prints nothing parseable.

`solvers` is the deduplicated set of solvers actually *resolved* across `harnesses[]` (not merely
requested): all-same-solver runs report one entry, a run with no named-solver harness (every harness
leaves the choice to CBMC, e.g. bare `--smt2`) reports an empty array. Entries are sorted by `name`,
so two runs resolving the same set produce the same document. Each `name` is spelled as
`harnesses[].resolved_solver` spells it: a closed lowercase name (`bitwuzla`, `cadical`, `cvc5`,
`kissat`, `minisat`, `z3`) or, for a custom `--external-sat-solver`/`Binary` override, the literal
binary path (open-ended). `version` is `null` for a solver CBMC has built in (`cadical`, `minisat`
report their own version as CBMC's) and otherwise the probed `--version` string.

A bare `null` is overloaded, so a sibling `source: "builtin" | "external"` disambiguates it: `null` on a
built-in solver means "no version of its own, by design"; on a separate-binary solver, "a probe failed."
It cannot be derived from the *name* — `--sat-solver cadical` (built-in) and a probe-failed
`--external-sat-solver cadical` both give `name == "cadical"`, `null` version — so it reflects the
resolution path (`effective_solver`/`CbmcSolver`): `"builtin"` when CBMC resolved without a separate
binary to probe (default `Cadical`/`Minisat`, or a `--sat-solver <name>` override), `"external"`
otherwise (a `CbmcSolver::Binary` path, or `Bitwuzla`/`Cvc5`/`Kissat`/`Z3`, run as a probed subprocess).

**Harnesses that are not `--harness`-selectable.** An `is_automatically_generated` harness (from `kani
autoharness`) cannot be selected by `--harness`/`--exact` at all: `find_proof_harnesses` skips
generated harnesses regardless of filter. Its `name` is still reported (a real, unique identifier), but
a consumer must not use it as a `--harness` argument when `is_automatically_generated` is `true`. A
`#[kani::proof_for_contract]` harness, by contrast, *is* selectable, and its `name` is the harness
function's own path — not the target it proves a contract for (that is
`attributes.kind.ProofForContract.target_fn`, see "Fields that are not always strings").

**Why not `mangled_name`.** `HarnessMetadata` also carries a `mangled_name` (the harness's name in
CBMC's symbol table), but this schema does not export it and it would not serve as a selector: it
identifies the harness to CBMC, not to Kani's CLI, and `--harness` does not accept it. `name`
(`pretty_name`) is chosen because it is simultaneously human-readable, module-qualified, and directly
re-runnable.

`checks`/`covers` bucket every property exhaustively by CBMC status. This partition and `n_properties`
cover exactly what they bucket; `code_coverage` (COVERED/UNCOVERED) properties under `--coverage` are
outside both and `n_properties` (no dedicated place today), so `n_properties == checks.total +
covers.total` always holds. `checks.success` is a bare count, unlike every other bucket: successful-check
identities are high-volume and low-value, whereas `covers.satisfied` names its properties because covers
are user-authored and few. Exporting successful-check identities is future work.

**The `checks` / `covers` partition criterion.** A property goes into `covers` exactly when
`property_id.class == "cover"` (`Property::is_cover_property`, `kani-driver/src/cbmc_output_parser.rs`);
every other property (including `class == "assertion"`, which carries ordinary `assert!`/panic checks)
goes into `checks`. This is a syntactic test on the CBMC-assigned class, independent of `status`: a
cover-class property carrying a status normally seen among checks (or vice versa) still partitions by
`class` and lands in the other bucket's `other[]` array rather than crossing over (see "the `other`
bucket" in Value domains). `code_coverage`-class properties are excluded from both buckets entirely.

**`failed_properties[]` membership.** `failed_properties[]` names exactly the properties in
`checks.failure` (`class != "cover"` with `status == "FAILURE"`), with the full per-property shape rather
than `checks.failure`'s bare id list. It does **not** include `checks.error` (an `ERROR` is solver-level
"could not determine," bucketed separately — see `failure_kind == "ERROR"`) or any cover-class property
(`covers.unsatisfiable` is that bucket's own list). This is why `n_failed == len(checks.failure)` on a
`COMPLETED` harness: `n_failed` is `failed_properties.len()`, whose membership *is* `checks.failure`'s.

**`warnings`** is empty in this run because no CBMC warning fired. A run that triggers one (a harness
using `kani::forall!` over a symbolic range the SAT backend cannot discharge) produces a message like
the one below; the `warnings_truncated` field and the `truncated`/`original_chars` pair are this
proposal's addition, not emitted by the proof-of-concept (which has no truncation logic today):

```json
{
  "warnings": [
    {
      "message": "warning: ignoring forall\n  * type: bool\n  0: tuple\n      * type: \n      0: symbol\n          * type: unsignedbv\n              * #source_location: \n                * file: <built-in- ...",
      "truncated": true,
      "original_chars": 11669
    }
  ]
}
```

`truncated`/`original_chars` replace a `[truncated; N chars total]` suffix inside `message`: a
structural field a consumer checks without pattern-matching free text, rather than the "parse the
string to learn a fact" this RFC argues against. `original_chars` is `null` when `truncated` is `false`
(nothing was cut). See the `warnings` discussion below for what this field does and does not promise.

**`configuration.checks.assertion_reach_checks`.** Whether Kani inserted reachability checks ahead of
ordinary assertions (`true` unless `--no-assertion-reach-checks` was passed). With reach-checks off, an
assertion made unreachable by a contradictory `kani::assume` lands in `checks.success` instead of
`checks.unreachable`, silently defeating the vacuity signal this schema carries; `configuration`
records exactly such toggles that change how a result must be read.

**`configuration.checks.ignore_global_asm` and `.extra_pointer_checks`.** Two more flags recorded for
the same reason. `ignore_global_asm` mirrors `--ignore-global-asm`: when `true`, Kani did not error on
`global_asm!`, so any behavior reachable only through that inline assembly is absent from the model and
a proof can pass vacuously with respect to its effects. `extra_pointer_checks` mirrors
`--extra-pointer-checks`: when `true`, Kani adds obligations for invalid pointers in relational
operations and pointer-arithmetic overflow, so two runs differing only in this flag check different
property sets, and `checks.total`/`checks.success` are not comparable between them.

**`configuration.checks.memory_safety`, `.overflow`, `.unwinding`, and `.undefined_function`.** The
remaining four `checks` bools, each mirroring a `--no-*-checks` flag (all default `true`; the field
name records the check being *on*, not the flag that turns it off):

- `memory_safety` mirrors `--no-memory-safety-checks` (and `--no-default-checks`, the whole group).
  `false` leaves out-of-bounds accesses and invalid-pointer dereferences unchecked; such a run can pass
  while containing exactly the memory-safety bugs Kani exists to find.
- `overflow` mirrors `--no-overflow-checks`. `false` leaves arithmetic overflow, NaN production, and
  division by zero all unchecked.
- `unwinding` mirrors `--no-unwinding-checks`. `false` stops CBMC asserting the loop/recursion bound
  covered every execution, so a bounded proof can silently miss behavior past the bound.
- `undefined_function` mirrors `--no-undefined-function-checks`. `false` drops calls to bodyless
  functions instead of asserting on them, so behavior reachable only through an unmodeled function is
  absent from the proof.

All seven meet the same "changes which properties are generated" test as the three above.

**`configuration.coverage_enabled`.** Mirrors `--coverage` (mandatory bool, default `false`). It is what
makes `code_coverage` (`COVERED`/`UNCOVERED`) properties exist — the ones carved out of `checks`,
`covers`, and `n_properties` — so it meets the `configuration` policy, and without it a consumer cannot
tell "no coverage properties in this schema yet" from "`--coverage` never passed." It sits alongside
`checks.*`, not nested under it.

**Policy for `configuration`.** A flag belongs in this block when it changes *which properties are
generated* or *what a status means* — when two runs differing only in it are not apples-to-apples
comparable, or when it can make a passing result mean less than it appears to. The seven `checks` flags
and `coverage_enabled` meet that test; future flags are added under this rule, each a minor schema
change (a new field). `cbmc_args` is the catch-all for what this rule cannot name individually: anything
passed straight to CBMC via `--cbmc-args` can change results in ways Kani cannot introspect, so it is
recorded and two runs with different `cbmc_args` are not assumed comparable. It is recorded *verbatim
modulo UTF-8* (captured with `to_string_lossy`, so a non-UTF-8 argument is rendered with U+FFFD rather
than round-tripped byte-for-byte): sufficient as a comparability signal, but not an exact argv to replay.

**Failure scenarios.**

- The output path is not writable → surfaced as an error and non-zero exit, never silently swallowed.
  The verification *verdict* is computed independently and never rewritten by an export problem. The
  up-front marker write fails before verification starts; the terminal export runs after the harness
  verdicts but *before* the SARIF artifact and final summary line, so a terminal export failure aborts
  those remaining steps. (Whether it should suppress the SARIF write is an implementation question, not
  settled here.)
- Any `tools.*` version cannot be determined → that field is `null`, never guessed; see "Tool
  provenance" above.
- A harness times out, is OOM-killed, or CBMC crashes on it → that harness's `outcome.kind`
  (`TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED`) records it and the file is still written. Run completeness is
  *verdict-independent*: as long as every selected harness produced a result, `run_state` is `COMPLETE`
  (a suite where every harness times out is a complete run of failing harnesses). `run_state` is
  `INCOMPLETE`/`PARTIAL` only when Kani did not reach the terminal write for every selected harness.
- Kani itself crashes (a `kani-compiler` ICE, a panic in `kani-driver`, a `SIGKILL`) → no terminal
  document is written: the single terminal write happens once at the very end (`verify_project`), and any
  hard error before it unwinds past it. There is no run-level `"CRASHED"` value; the only residue is a
  stale `INCOMPLETE` marker (or, if the crash preceded it, whatever file already existed). See "How
  `run_state` and `outcome.kind` co-occur" below.
- The `--export-json` path already holds a file → it is **overwritten up front with an atomic
  `run_state: "INCOMPLETE"` marker** once harness verification begins (after the crate is built), so an
  earlier run's results cannot be misread as this run's, and a run that dies after that leaves a file that
  openly says it did not finish (replacing an earlier delete-up-front design; see the completeness
  contract and open question below).
- The parent directory does not exist → it is created (`create_dir_all`), matching `--sarif`. A path
  that is itself an existing directory fails today only when the write is attempted (the same-directory
  temp write cannot be created inside a file-shaped target); adopting this RFC moves that to the
  argument-parse-time rejection proposed under *Interaction with other flags*, so the failure arrives
  before any verification work, as for `--sarif`. The shipped difference is an accident of
  implementation order, not intentional.

**How `run_state` and `outcome.kind` co-occur.** `outcome` is meaningful only once Kani reaches a
terminal write, and at run level its `kind` has exactly one value: `COMPLETED`. The marker
(`run_state == "INCOMPLETE"`) is written *before* that and carries no `outcome`. A run-level
`outcome.kind == "CRASHED"` does not exist (see the crash scenario above); what a consumer observes
instead is a stale `INCOMPLETE` marker, or a missing file — that staleness *is* the crash signal, not a
value to look up. Every producible combination:

| `run_state` | `outcome` |
|---|---|
| `INCOMPLETE` (marker) | absent |
| `COMPLETE` | `{"kind": "COMPLETED"}` |
| `PARTIAL` | `{"kind": "COMPLETED"}` |
| `NO_HARNESSES_SELECTED` | `{"kind": "COMPLETED"}` |

Per-harness `outcome.kind == "CRASHED"` is unaffected and remains real: the run can finish, reach its
terminal write, and still report CBMC crashed on one harness — a fact about *one harness* in a document
Kani *did* finish, distinct from Kani never reaching the write.

**`NO_HARNESSES_SELECTED` versus a crate with no harnesses at all.** Both leave `harnesses[]` empty and
`harness_selection.matched_count == 0`; they are told apart by `harness_selection.requested_filters`:
non-empty means a `--harness` filter matched nothing in a crate that does define harnesses (an
`unmatched_filters` entry names which); empty means no filter was given and the crate defines no
`#[kani::proof]` (or, under `autoharness`, no eligible function).

**A vacuity hole this schema does not close on its own: cover-only vacuity.** The normative and advisory
vacuity predicates above read only `checks.*`; an `unreachable` *cover* lands in `covers.unreachable` and
trips neither. A consumer wanting the symmetric signal should additionally apply `covers.total > 0 &&
covers.unreachable.len() == covers.total` per harness, since this schema does not apply it for them.

`null` always means *not measured or not applicable*, never a guess, and is always distinguishable
from `0`, `false`, and `[]`.

**Presence matrix — top-level fields, marker versus terminal document.** The marker and a terminal
document are not the same shape; the field table above states presence in terms of "the document",
unpacked here:

| Field(s) | `INCOMPLETE` marker | Terminal document (`COMPLETE` / `PARTIAL` / `NO_HARNESSES_SELECTED`) |
|---|---|---|
| `schema_version`, `kani_commit`, `kani_commit_dirty`, `tools.*`, `machine.*`, `enabled_unstable_features`, `harness_selection.*`, `harness_timeout_s`, `configuration.*`, `target`, `started_at` | present (all knowable before verification begins) | present |
| `run_state` | present, always `"INCOMPLETE"` | present, one of `"COMPLETE"`/`"PARTIAL"`/`"NO_HARNESSES_SELECTED"` |
| `outcome`, `wall_time_s` | absent | present — neither is knowable until Kani reaches a terminal state, so the marker (written *before* that state is known) carries neither |
| `summary`, `harnesses[]` | absent, by design (stated above): a consumer that reads the marker has no verdict to read at all | present (`harnesses[]` may be empty, only under `NO_HARNESSES_SELECTED`) |

**Presence matrix — per-harness fields, by `harnesses[].outcome.kind`.** Every "—" in the harness field
table above means "on the harness object as produced for that `outcome.kind`"; spelled out for the three
kinds where it is not simply "always":

| Field(s) | `COMPLETED` | `TIMEOUT` / `OUT_OF_MEMORY` | `CRASHED` |
|---|---|---|---|
| `name`, `crate_name`, `file`, `line`, `contract`, `is_automatically_generated`, `has_loop_contracts`, `is_bounded`, `attributes.*`, `resolved_solver`, `resolved_unwind`, `generated_concrete_test` | present | present (all known before CBMC ran, or resolved as part of preparing to run it) | present |
| `resources.verification_time_s` | present, a real verification duration | present, time elapsed until the timeout/OOM fired | present, time elapsed until the crash |
| `outcome.verdict` | present | absent | absent |
| `outcome.code` / `outcome.message` | absent | absent | present |
| `failure_kind` | present | absent | absent |
| `checks`, `covers` (the objects themselves) | present | present | present |
| `checks.total`, `checks.success`, `covers.total` | integer | `null` | `null` |
| `checks.failure`, `.unreachable`, …, `covers.satisfied`, … (identity-list fields) | array of ids, may be empty | `[]` | `[]` |
| `n_properties`, `n_failed` | integer | `null` | `null` |
| `failed_properties[]`, `unsupported_constructs[]` | array, may be empty | `[]` | `[]` |
| `warnings[]`, `warnings_truncated` | array / integer | `[]` / `0` | `[]` / `0` |

### Value domains, casing, and which fields are not strings

**Enum domains.** Every field whose value is drawn from a fixed set is *closed*: each vocabulary below
is a Kani-owned Rust enum (the CBMC-derived `CheckStatus` among them, which Kani re-exports). The
`checks.*`/`covers.*` `other` field is a *bucketing* device, not an enum catch-all; see below.

| Field | Values | Closed? |
|---|---|---|
| `outcome.kind` (run level) | `COMPLETED` (single value; see below) | closed |
| `run_state` (run level) | `INCOMPLETE`, `COMPLETE`, `PARTIAL`, `NO_HARNESSES_SELECTED` | closed |
| `harnesses[].outcome.kind` | `COMPLETED`, `TIMEOUT`, `OUT_OF_MEMORY`, `CRASHED` | closed |
| `harnesses[].outcome.verdict` | `SUCCESS`, `FAILURE` (field omitted at run level) | closed |
| `harnesses[].failure_kind` | `NONE`, `PANICS_ONLY`, `OTHER`, `ERROR` | closed |
| `…[].status` (in `failed_properties`, `unsupported_constructs`, `checks.other`, `covers.other`) | `SUCCESS`, `FAILURE`, `SATISFIED`, `UNSATISFIABLE`, `UNREACHABLE`, `UNDETERMINED`, `ERROR`, `UNKNOWN`, `COVERED`, `UNCOVERED` | closed |
| `harnesses[].attributes.kind` | `"Proof"`, `"Test"`, `{"ProofForContract": {...}}` | closed (`HarnessKind`, 3 variants) |
| `harnesses[].attributes.solver` | `"Cadical"`, `"Bitwuzla"`, `"Cvc5"`, `"Kissat"`, `"Minisat"`, `"Z3"`, or `{"Binary": "<path>"}` | **explicitly open** — see below |
| `harnesses[].resolved_solver`, `tools.solvers[].name` | the lowercase spellings of the same six names, or an arbitrary binary-path string | **explicitly open** — see below |
| `tools.solvers[].source` | `"builtin"`, `"external"` | closed |

`outcome.kind` is deliberately asymmetric: `TIMEOUT`/`OUT_OF_MEMORY` describe a *harness* CBMC could not
finish and never appear at run level (a terminal document only carries `COMPLETED`; a run-level OOM
surfaces as a stale `INCOMPLETE` marker, per co-occur above). Harness-level `OUT_OF_MEMORY` is inferred
from a `137` exit (SIGKILL), not a direct measurement; `TIMEOUT` arises only under `--harness-timeout`.

`failure_kind` is scoped to `outcome.kind == "COMPLETED"`: a `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` harness
never reaches `determine_failed_properties` (no property list to classify), so the field is **omitted**
there, the same "absent, not `null`" treatment as `outcome.verdict`. On a `COMPLETED` harness it is the
*raw* classification from `determine_failed_properties` (verbatim from `FailedProperties`: `NONE`,
`PANICS_ONLY`, `OTHER`, `ERROR`), computed independently of the interpreted `outcome.verdict`. Their
relationship is a truth table (`kani-driver/src/call_cbmc.rs`'s `verification_outcome_from_properties`),
not a single rule, turning on `attributes.should_panic`:

| `attributes.should_panic` | `failure_kind` | `outcome.verdict` |
|---|---|---|
| `false` | `NONE` | `SUCCESS` |
| `false` | `PANICS_ONLY`, `OTHER`, or `ERROR` | `FAILURE` |
| `true` | `PANICS_ONLY` | `SUCCESS` |
| `true` | `NONE`, `OTHER`, or `ERROR` | `FAILURE` |

For `should_panic == false`, `failure_kind == "NONE"` ⟺ `outcome.verdict == "SUCCESS"`. For
`should_panic == true` the two are effectively *inverted* (a passing should-panic harness has
`failure_kind == "PANICS_ONLY"`; a failing one, `"NONE"`, since `determine_failed_properties` looks only
at property statuses and the missing panic is not one), so a consumer must read `attributes.should_panic`
before concluding anything from `failure_kind` alone.

The `status` values are the closed `CheckStatus` set; the `other` bucket is *not* for unknown statuses
but for known `CheckStatus` values that do not map to a named bucket in that domain (e.g. a cover-only
`SATISFIED`/`UNSATISFIABLE` among checks), keeping the partition exhaustive. A status `CheckStatus` does
not model is a parser concern resolved by adding a variant — a *major* change, since `status` is a closed
enum a consumer may match exhaustively.

**`attributes.kind` is closed; the solver names are explicitly open.** `attributes.kind` is `HarnessKind`
(3 variants: `Proof`, `ProofForContract { target_fn }`, `Test`); a fourth is a major change. The
solver-name fields (`attributes.solver`, `resolved_solver`, `tools.solvers[].name`) are the opposite:
`CbmcSolver` already has an open escape hatch (`Binary(String)`, any path), so a consumer already treats
an unrecognized name as valid-but-unfamiliar. Adding a new *named* built-in solver is therefore a
**minor** change, unlike a new `status` or `failure_kind` variant, which a consumer is *entitled* to
assume can never appear.

**Casing.** The file mixes three conventions, and one rule explains all three: *a value keeps the
serialization of the Rust type it comes from, and this schema does not fork types to re-case them.*

- **snake_case keys**, because the schema reuses `kani_metadata` and `cbmc_output_parser` types directly
  (`HarnessAttributes`, `AssignsContract`, `CheckStatus`); re-casing would fork those types, recreate the
  duplication debt of [#3541](https://github.com/model-checking/kani/issues/3541), or break existing
  `.kani-metadata.json` consumers. Those three already derive `Serialize`; but `Property`, `PropertyId`,
  and `SourceLocation` derive only `Deserialize` (never serialized out), so
  `failed_properties[]`/`unsupported_constructs[]`/`checks.other[]`/`covers.other[]` are purpose-built
  export structs rather than a direct embedding of `Property`.
- **SCREAMING_SNAKE_CASE enum values** (`outcome.kind`, `verdict`, `failure_kind`, `run_state`, every
  `status`), mirroring the reused CBMC-status types (`CheckStatus`, `FailedProperties`); the new enums
  (`Outcome`, `verdict`, `run_state`) use it too, so a consumer sees one value convention across the file.
  (Multi-word values like `NO_HARNESSES_SELECTED` need an explicit `SCREAMING_SNAKE_CASE` rename, since
  `CheckStatus`'s `UPPERCASE` coincides with it only for single-word variants.)
- **PascalCase, object-shaped**, for the two embedded `kani_metadata` attribute enums below, which keep
  their own serde derivation untouched for the same no-forking reason.

**Fields that are not always strings.** Two reused `kani_metadata` enums have non-unit variants and so
serialize as *objects*, not strings:

- `harnesses[].attributes.kind` is `"Proof"` or `"Test"`, but `{"ProofForContract": {"target_fn": "…"}}`
  for a contract-proof harness.
- `harnesses[].attributes.solver` is `"Cadical"`, `"Z3"`, etc. for a named solver, but `{"Binary": "…"}`
  for a custom solver path.

These are the *requested* attributes, carried verbatim from `.kani-metadata.json`; the resolved
counterparts (`resolved_solver`, `resolved_unwind`) are plain scalars — a string or number, or `null` —
never the object forms above.

**Reconciling the three solver spellings.** `resolved_solver` and `tools.solvers[].name` are always
spelled identically (both from the same resolution — CLI `--solver` > harness attribute > `--cbmc-args`
override > default); only `attributes.solver` differs, being the *requested* value in `CbmcSolver`'s
PascalCase. Compare `attributes.solver` against `resolved_solver` for "asked for" vs "ran".

Worth adopting from `kani list` is its versioning idiom: tool version and schema version as separate
fields. This proposal does that; see **Compatibility policy** below.
