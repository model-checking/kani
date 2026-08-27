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
interpreting that outcome correctly. This is evidence about the run, not everything needed to
reproduce it byte-for-byte: `cbmc_args` is recorded verbatim but is not a replayable argv (see
its own section below).
The schema in this RFC is the proposed contract for that file; whether it supersedes the shipped
v1 shape, and how, is the first open question.

## User Impact

This RFC exists because issue #942, *"Design Machine Readable Output - RFC"*, asks for exactly this
document, and because `kani-driver` carries a TODO pointing at it:

```rust
// TODO: Record processed items and dump them into a JSON file
// <https://github.com/model-checking/kani/issues/942>
```

**Kani already has a first-party consumer that needs this, and it is fragile by construction.**
`tools/benchcomp/benchcomp/parsers/kani_perf.py` regex-scrapes Kani's own stdout for
`Runtime Solver`, `Runtime Symex` and `Generated N VCC(s)`, and carries this comment:

> `# CBMC prints out some metrics more than once, e.g. "Solver" and "decision procedure". Add those values together`

Every other consumer is in the same position: greps for `VERIFICATION:- `, for
`** N of M cover properties satisfied`, for `Verification Time:`. Those strings are printing
details, not an interface. When they change, a consumer's grep silently matches nothing. The
failure mode is silence, not an error.

Who this helps:

- **CI gates and dashboards:** decide pass/fail, and detect regressions, without parsing prose.
- **Standard-library and large-scale verification efforts**, where hundreds of harnesses run and the
  question is not "did the build pass" but "what is proven, and did any proof stop meaning anything".
- **Commercial and industrial pipelines**, where a verifier must report cost and outcome to systems
  that were not written by the person running it. Machine-readable results are the precondition for
  using a verifier in an automated pipeline at all.
- **Kani's own tooling**, which can stop scraping its own output.

### The specific gap: a proof can pass while proving nothing

A contradictory `kani::assume` makes every subsequent assertion unreachable. The harness reports
`VERIFICATION:- SUCCESSFUL` and exits 0, including for an assertion as obviously false as
`assert!(x != x)`. Kani's *text* output says so (`** 0 of 2 failed (2 unreachable)`), but a program
checking the exit code, which is what automated consumers do, cannot tell that proof from a real
one.

Kani already computes what is needed to say this. Reachability checks are generated per assertion and
on by default; `Property.reach` is populated; and `update_properties_with_reach_status` already
demotes a `Success` to `Unreachable` when the result cannot be trusted. **That reasoning currently
lives in the presentation layer and reaches no machine-readable output.** This RFC's core proposal is
to carry it in the results file.

**Downside.** A schema is an interface, and interfaces constrain future change. That is why the
schema stays behind an unstable gate with an explicit version field, so the shape can be corrected
while it is still being learned from real consumers.

### Relationship to the shipped implementation (#4472)

[PR #4472](https://github.com/model-checking/kani/pull/4472) merged on 2026-08-12 and ships
`--export-json` on `main`, behind `-Z unstable-options`. It is the as-built v1, and the design
discussion there shaped this proposal. The capability therefore exists today. What this RFC settles
is the contract for it.

The shipped document differs from the schema specified here in shape and vocabulary: it exports
count summaries plus per-harness detail arrays correlated by `harness_id == pretty_name`, it
serializes status values in their Rust `Debug` casing, and it gates on `-Z unstable-options` rather
than a dedicated feature ident. Whether this schema supersedes the shipped shape while the flag is
unstable, and what migrates, is the first open question below. This RFC is numbered `0015`, per
review; #4472 merged without an RFC file, so the number is free.

## User Experience

```
cargo kani -Z export-json --export-json results.json
```

(This shows the dedicated gate this RFC proposes. The shipped flag gates on
`-Z unstable-options` today; see "On the `-Z` gate" below.)

The flag is additive for every output format it supports: existing rendered output is unchanged, and
the file is written in addition to it. Omitting the flag changes nothing. One combination is rejected
outright rather than silently misreporting: `--output-format=old` bypasses CBMC's structured JSON
output entirely (`run_terminal_timeout` mocks a success/failure result with zero properties either
way, and treats a timeout as success), so `--export-json` under that format would produce a
well-formed file indistinguishable from a real clean run, including for a run that actually timed
out. Kani's argument parser rejects `--export-json` combined with `--output-format=old` with an
explicit error before verification starts.

### Interaction with other flags

`--export-json` has a defined answer for every flag `--sarif` guards, and for the flags that change
*what* or *how many* results exist:

- **`--only-codegen`:** rejected, like `--sarif`: there are no verification results to export.
- **`--jobs N` / concurrent harnesses:** allowed. The `harnesses[]` array is written in a deterministic
  order, sorted by `(crate_name, file, line, name)`, not completion order, so the harness order is
  stable across runs and a CI job can diff exports across commits without spurious reordering. The
  inherently volatile fields (timestamps, wall and verification times) still differ between runs, as they
  must; determinism is a promise about *structure and ordering*, not byte-identity.
- **`--output-into-files`:** additive and independent: that flag controls where the *rendered text*
  goes; `--export-json` writes its own file regardless, and the two do not interact.
- **`cargo kani` over a multi-crate workspace:** one file per Kani invocation, covering every harness in
  the run; each harness carries its `crate_name`, so same-named harnesses in different crates stay
  distinct. It is not written once per crate, and a later crate does not overwrite an earlier one's
  results.
- **A path of `-`:** *not* special-cased to mean stdout; it is treated as a literal filename. Streaming
  the export to stdout is a reasonable CI want, but it interacts with the atomic-write contract below
  (there is no target to `rename` onto), so it is deferred as an explicit future option rather than
  half-specified now.
- **A path that already exists as a directory**: this RFC proposes rejecting it at argument-parse
  time, as `--sarif` does. The shipped implementation instead fails later, when the write is
  attempted; see the write-behaviour notes below.

**On the flag's name.** `--export-json` is a verb where every other artifact-producing flag is a noun
(`--sarif`, `--output-into-files`), and it does not say *what* is exported. `--results-json` would read
better beside `--sarif` and age better if Kani ever emits a second JSON artifact; the spelling is not
yet load-bearing. This RFC uses `--export-json` to match the proof-of-concept and the issue #942
discussion, and treats the final name as an open decision rather than a design commitment.

**On the `-Z` gate.** As shipped, the flag gates on the generic `-Z unstable-options`. This RFC
proposes a dedicated `-Z export-json` identifier instead, matching the per-feature gating pattern
(`--coverage` gates on `SourceCoverage`), so this artifact can stabilize or be dropped
independently of unrelated unstable options. The switch is a one-line gate change on an unstable
surface; it belongs to the migration discussed in the first open question.

### Example

The output below is **the proposed 0015 document**, shown for the vacuity case this RFC's
motivation section describes: two ordinary assertions made unreachable by one contradictory
`kani::assume`, so Kani's own text output reads `** 0 of 2 failed (2 unreachable)`. That text
output is verbatim from a real run of the proof-of-concept at commit
`7b125f1b47e36ca4cc50c4041abeca01912f80f9`, and the harness and its properties below are exactly
that run's. **The JSON is not**, however, a verbatim dump of that commit's own `--export-json`
output: the PoC at `7b125f1b` emits flat `kani_version`/`cbmc_version` fields, not the grouped
`tools` object shown below, and reports completeness as a boolean `run_complete`, not the
`run_state` this RFC proposes (see "The completeness contract" below). These two are only the most
visible of this document's proposed additions over the PoC's own shape — `is_bounded`,
`configuration.coverage_enabled`, `tools.solvers[].source`, and the `summary` section are further
examples, all covered in their own sections below; treat the JSON as an instance of the *proposed*
schema against the PoC's real vacuity run, not as a capture of what the PoC prints today.

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

This is the vacuity case made machine-readable: the harness's `outcome.verdict` is `SUCCESS` and Kani's
exit code is `0`, exactly like a harness that proved something. But `checks.unreachable` names both
properties that could not actually be exercised, so a consumer no longer has to trust the exit code alone.

### Field reference

Every field the example shows, normatively. `null` always means *not measured or not applicable*
(see "Value domains" below); "—" in the Null? column means the field is never absent and never `null`
**in a terminal document** (`COMPLETE` / `PARTIAL` / `NO_HARNESSES_SELECTED`). The `INCOMPLETE` marker is
a separate, narrower shape governed by its own presence matrix below, not by this column: several fields
marked "—" here (`outcome`, `wall_time_s`, `summary`, `harnesses[]`) are absent from the marker entirely.

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

These `other[]` elements deliberately carry only `id` and `status`, **not** `description`/`class`/`file`/
`line`/`trace_available` — the fuller shape is reserved for `failed_properties[]`/
`unsupported_constructs[]`, where a consumer actually needs to act on the property; `other` exists so
the partition stays exhaustive, not to duplicate the full record for a rare, already-anomalous status.

**`checks{}` / `covers{}` bucket shape**, both objects:

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `total` | integer | nullable; `null` when the owning harness's `outcome.kind` is not `COMPLETED` | Every property this bucket accounts for. |
| `success` (checks only) | integer | nullable; `null` under the same condition as `total` | A **count**, not an identity list (see "Rationale" for why). |
| `satisfied` (covers only) | array of ids | never null, may be empty (empty, not absent, on a non-`COMPLETED` harness) | An identity list (covers are user-authored and few). |
| `failure` (checks) / `unsatisfiable` (covers) | array of ids | never null, may be empty | |
| `unreachable`, `undetermined`, `error`, `unknown` | array of ids, each | never null, may be empty | |
| `other[]` | array of `{id, status}` | never null, may be empty | See above. |

On a `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` harness, CBMC produced no property list to bucket at all: `total`
(and `success`, for `checks`) is `null` — never `0`, which would misread as "measured, and none passed" —
and every identity-list field is `[]`, since there is nothing to name, not nothing that was found. The
bucket-arithmetic invariant below is trivially still consistent (`null` is not a number to sum against),
but does not become a meaningful claim about that harness until it reaches `COMPLETED`.

The bucket-arithmetic invariant holds unconditionally for both objects **on a `COMPLETED` harness**:
`checks.total == success +
len(failure) + len(unreachable) + len(undetermined) + len(error) + len(unknown) + len(other)`, and
symmetrically for `covers` with `satisfied` in place of `success`. Combined with the exclusion of
`code_coverage` properties from both buckets, this is also why `n_properties == checks.total +
covers.total` holds as a result, not as a separate promise (see "Rationale" below).

**Tool provenance (`tools`).** A single object carries the version of every tool whose behaviour can
change a result: `kani`, `rustc`, `cbmc`, `goto_cc`, `goto_instrument`, and a `solvers` array of
`{name, version, source}`. This is the machine-readable form of the versions Kani already prints, closing the
gap in [#2572](https://github.com/model-checking/kani/issues/2572): a consumer comparing two runs, or
bisecting a regression to a toolchain bump, no longer scrapes stdout for them.

`goto_synthesizer` is a seventh, *conditional* key: it is present only when the run requested
loop-contract synthesis (`--synthesize-loop-contracts`), and absent otherwise, since that tool has no
reason to be probed at all for a run that never asks for it. Every key that is present follows one null
rule, generalizing beyond `cbmc` alone: **a version that cannot be determined is `null`, never guessed**,
whether the probe binary is missing, refuses `--version`, or prints nothing parseable. A missing key
(only possible for `goto_synthesizer`) means the tool was not *requested* this time — presence tracks the
`--synthesize-loop-contracts` request, not whether synthesis subsequently succeeded, per the field
reference above; a present `null` means it was requested but its version could not be pinned down.

`solvers` is the deduplicated set of solvers actually resolved across every harness in `harnesses[]`
(not merely requested): a run whose harnesses all resolve to the same solver reports one entry, and a
run with no named-solver harness (every harness leaves the choice to CBMC, e.g. bare `--smt2`) reports
an empty array. Entries are sorted by `name`, so two runs that resolve the same solver set produce the
same document. Each `name` is spelled exactly as `harnesses[].resolved_solver` spells it: one of the
closed lowercase names (`bitwuzla`, `cadical`, `cvc5`, `kissat`, `minisat`, `z3`) or, for a custom
`--external-sat-solver`/`Binary` override, the literal binary path string, open-ended and not part of
the closed set. `version` is `null` for a solver CBMC has built in (`cadical`, `minisat` report their
own version as CBMC's, so naming a version for them would be misleading) and otherwise the probed
`--version` string, following the same null-means-undetermined rule as every other `tools.*` field.

A bare `null` here is overloaded, though, and this proposal adds a sibling `source: "builtin" |
"external"` to disambiguate it: `null` for a built-in solver means "this solver has no version of its own
to report, by design," while `null` for a solver run as a separate binary means "a version probe was
attempted and failed." Critically, `source` cannot be derived from the solver *name* string, because the
name alone does not disambiguate these two cases: `--sat-solver cadical` (CBMC's own built-in CaDiCaL) and
a probe-failed `--external-sat-solver cadical` (a same-named binary on `PATH`, run as a separate process)
both resolve to `name == "cadical"` with a `null` version, for entirely different reasons, and a
name-keyed rule cannot tell them apart. `source` instead reflects the actual resolution path, matching
`effective_solver`/`CbmcSolver` in the code: it is `"builtin"` exactly when CBMC resolved the solver
without a separate binary to probe — the default resolution to `CbmcSolver::Cadical`/`CbmcSolver::Minisat`,
or a `--sat-solver <name>` override, both of which select a solver CBMC already has built in and never
name an external binary — and `"external"` in every other case: a resolved `CbmcSolver::Binary` (a custom
`--external-sat-solver` path) or one of `Bitwuzla`/`Cvc5`/`Kissat`/`Z3`, all of which CBMC runs as a
separate process Kani can, and does, attempt to probe. This is what correctly separates `--sat-solver
cadical` (`source: "builtin"`, no probe ever attempted) from `--external-sat-solver cadical` (`source:
"external"`, a probe was attempted and, here, failed) even though both spell the resolved name identically.
A consumer that wants to tell "this solver's version is unknowable in principle" from "this solver's
version probe failed on this machine" reads `source`, not `version` alone.

**The harness name (`name`) is already the re-runnable selector.** `name` is the harness's
`pretty_name`: the fully qualified name the user gave the function, module path included (e.g.
`sequence::proofs::no_over_read`; for a crate-root harness it equals the bare function name, since
there is no module path to add). This is exactly the string `--harness --exact` accepts to re-run this
one harness again — `name` is not a bare, local identifier that needs a separate "selector" field
alongside it; it *is* the selector. Three things a consumer must still get right:

- **`name` is unique within a crate, not across a workspace.** Two different crates in one
  `cargo kani` workspace run can each define a harness with the same fully qualified `name` (a
  `sequence::proofs::no_over_read` in `crate_a` and another in `crate_b`). Within a multi-crate
  workspace, pair `name` with `crate_name` as the actual join key, and re-run with
  `-p <crate_name> --harness --exact <name>`.
- **`crate_name` is not always what `-p` wants.** `crate_name` (`kani_metadata/src/lib.rs:29`) is
  `krate().name`, the *rustc* crate identifier: always underscored, since Rust identifiers cannot
  contain a hyphen. `cargo -p` takes the *Cargo package* name instead, which is whatever the package's
  `Cargo.toml` names it and commonly contains hyphens (a package named `my-crate` compiles to a crate
  named `my_crate`; Cargo does this substitution automatically and silently). This schema does not
  export the Cargo package name today, so `-p <crate_name>` round-trips only when the two happen to
  coincide (a package name with no hyphens). When they differ, a consumer must map `crate_name` back to
  its owning package's `Cargo.toml`-declared name itself before re-running with `-p`; this schema gives
  it the rustc name to look up, not the Cargo name to pass directly.
- **Plain `--harness` is substring-matching; only `--exact` makes `name` round-trip.** Without
  `--exact`, `--harness <name>` can match more than the one harness `name` names (see
  `harness_selection.exact` above). A consumer that needs to reproduce *exactly* this result must pass
  `--exact` alongside `--harness`.

**Harnesses that are not `--harness`-selectable.** An `is_automatically_generated` harness (from
`kani autoharness`) is not something `--harness`/`--exact` can select at all: `find_proof_harnesses`
skips automatically generated harnesses regardless of filter, by design. Its `name` is still reported
(it is a real, unique identifier for that harness in this run), but a consumer must not treat it as a
usable `--harness` argument when `is_automatically_generated` is `true`. A `#[kani::proof_for_contract]`
harness, by contrast, *is* selectable, and its `name` is the harness function's own path — not the
target function it proves a contract for (that target is `attributes.kind.ProofForContract.target_fn`,
a separate field, see "Fields that are not always strings" below).

**Why not `mangled_name`.** `HarnessMetadata` also carries a `mangled_name` — the name of the harness in
the CBMC symbol table — but this schema does not export it, and it would not serve as a selector even
if it did: it identifies the harness to CBMC's internal machinery, not to Kani's own CLI, and is not a
string `--harness` accepts. `name` (`pretty_name`) is the chosen key precisely because it is the one
string that is simultaneously human-readable, already module-qualified, and directly re-runnable.

**The bounded-result flag (`is_bounded`).** `is_bounded` is mandatory on every harness, not merely
restored from the shipped writer: a bounded result read as an unrestricted one is exactly the
over-claim class this schema exists to prevent, the same reasoning that makes the vacuity predicates
below normative rather than advisory. It is `true` exactly when `kani autoharness` generated this
harness *and* bound at least one of the function's arguments (for example a slice reference) to a
*bounded* nondeterministic value, because `--autoharness-bounded-arguments` was passed and the
argument's type had no unbounded `Arbitrary` strategy Kani could use instead; in that case the harness's
`outcome.verdict == "SUCCESS"` proves the target only for inputs representable within that bound, not for
every input of that type. A manually written harness (`is_automatically_generated == false`) always
reports `false` — this field is never omitted, on any harness, so a consumer never has to branch on
`is_automatically_generated` before reading it — and so does an autoharness-generated harness that
needed no bounded argument at all. Compare `is_ctor_based` (still open, see "Shipped fields dropped or
transformed" below): the two flags are orthogonal caveats on an autoharness result (one about bounding
a value's range, the other about which values a constructor can reach), and only `is_bounded` changes
whether the result can be read as an unrestricted proof.

The consumer applies two named predicates (all fields are per-harness: `harnesses[].outcome.verdict`,
`harnesses[].checks.*`, abbreviated below), both guarded on `verdict == "SUCCESS"` and both
*inapplicable* when `configuration.checks.assertion_reach_checks` is `false`:

- **Vacuous pass (normative).** `verdict == "SUCCESS" && checks.total > 0 && checks.unreachable.len() ==
  checks.total`: every check in a passing harness is unreachable, so the proof examined nothing. Stated as
  `unreachable.len() == total` rather than `success == 0` deliberately: `success == 0` is *necessary but
  not sufficient*, because a passing harness can hold an *expected* failure. A `#[kani::should_panic]`
  harness's panic check is a `FAILURE` under a `SUCCESS` verdict, so `success == 0` alone would fire on a
  legitimate `should_panic` proof; requiring *every* check to be unreachable excludes that case (the
  expected failure keeps `unreachable.len() < total`). Here `checks.total > 0` *is* load-bearing: it
  excludes a harness with no checks at all, on which `unreachable.len() == total == 0` would otherwise hold
  vacuously. The `SUCCESS` guard is load-bearing too: a *failing* harness can also have every check
  unreachable, so it is what makes this a statement about a *pass*.
- **Vacuity-suspect (advisory).** `verdict == "SUCCESS" && !checks.unreachable.is_empty()`: a passing
  harness with *any* unreachable check, catching the *partial* vacuity the normative rule misses (some
  checks proved something, others were dead). This is noisier (a deliberately dead assertion trips it
  without meaning the proof is worthless), hence advisory: it flags candidates for a human to inspect, not
  a condition for a machine to gate on. (It reads only `checks.unreachable`; an unreachable *cover* lands
  in `covers.unreachable` and does not trip it.)

Both rules are sound only when `configuration.checks.assertion_reach_checks` is `true`: with reach-checks
off, an assertion made unreachable by a contradictory `kani::assume` reports as `SUCCESS` and inflates
`checks.success` instead of landing in `checks.unreachable`, so a consumer must read that flag and treat
`assertion_reach_checks == false` as "this run cannot surface vacuity through `checks.unreachable`". The
`configuration.checks.assertion_reach_checks` note below explains the mechanism; stating the predicates
and the flag together is deliberate, so the schema is self-documenting on its own central claim.

`checks` (and `covers`, symmetrically) buckets every property this schema accounts for exhaustively by
CBMC status: `success`/`satisfied`, `failure`/`unsatisfiable`, `unreachable`, `undetermined`, `error`, `unknown`, and a catch-all
`other` for any status this schema does not yet know how to classify. So a consumer never has to
guess where an unrecognized result went. Scope: this partition, and `n_properties`, cover exactly what
`checks` and `covers` bucket. Under `--coverage`, `code_coverage` properties (COVERED/UNCOVERED) are
outside both buckets and outside `n_properties` too, since this schema does not export them in a
dedicated place today; `n_properties == checks.total + covers.total` always holds as a result.
`checks.success` is a bare count rather than an identity list, unlike every other bucket here:
successful-check identities are high-volume (checks are auto-generated, sometimes in the thousands)
and low-value (nothing further to investigate), whereas `covers.satisfied` above names its properties
because covers are user-authored and few, so naming which ones passed costs nothing and confirms
intent. Successful-check *identities* are deliberately not exported today: a consumer that needs to
know *what* was proven, not merely that everything passed, needs the full property list, which is
future-work territory this proposal does not promise.

**The `checks` / `covers` partition criterion.** A property goes into `covers` exactly when
`property_id.class == "cover"` (`Property::is_cover_property`,
`kani-driver/src/cbmc_output_parser.rs`); every other property (including `class == "assertion"`, the
class that carries ordinary `assert!`/panic checks) goes into `checks`. This is a syntactic test on the
property's own CBMC-assigned class, independent of its `status`; a cover-class property that happens to
carry a status normally seen among checks (or vice versa) still partitions by `class`, and lands in the
other bucket's `other[]` array rather than crossing over (see "the `other` bucket" in Value domains
below). `code_coverage`-class properties are the one class excluded from both buckets entirely, per the
scope note above.

**`failed_properties[]` membership.** `failed_properties[]` names exactly the properties in
`checks.failure` — that is, `class != "cover"` properties whose `status == "FAILURE"` — with the full
per-property shape (`id`, `description`, `class`, `file`, `line`, `trace_available`, `status`) rather
than `checks.failure`'s bare id list. It does **not** include `checks.error` (an `ERROR` status is a
solver-level "could not determine," not a failure the harness's code caused, and is bucketed and
counted separately: see `failure_kind == "ERROR"` above) or any cover-class property (`covers.unsatisfiable`
is that bucket's own, separately-named list, and is never merged into `failed_properties[]`, keeping
`failed_properties[]` scoped to `checks` alone). This is exactly why the invariant `n_failed ==
len(checks.failure)` holds unconditionally on a `COMPLETED` harness: `n_failed` is defined as
`failed_properties.len()`, and `failed_properties[]`'s membership is defined to be `checks.failure`'s,
so the two counts are the same set counted twice, not two independent numbers that happen to agree.

**`warnings`** is empty in this run because no CBMC warning fired. A run that does trigger one (a
harness using `kani::forall!` over a symbolic range, which the SAT backend cannot discharge) produces
a warning message like the one below, taken from that same class of run; the `warnings_truncated` field,
and the `truncated`/`original_chars` pair on each entry, are this proposal's addition and are not
emitted by the proof-of-concept, which has no truncation logic today:

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

`truncated`/`original_chars` replace encoding the same information as a `[truncated; N chars total]`
suffix inside `message` itself: a structural field a consumer can check without pattern-matching
free text stays true to this schema's own reason for existing, whereas the suffix was exactly the kind
of "parse the string to learn a fact" this RFC otherwise argues against. `original_chars` is `null`
when `truncated` is `false` (nothing was cut, so there is no original length distinct from `message`'s
own length to report). See the `warnings` discussion below for what this field does and does not
promise.

**`configuration.checks.assertion_reach_checks`.** Records whether Kani inserted reachability checks
ahead of ordinary assertions (`true` unless `--no-assertion-reach-checks` was passed). This exists
because that flag changes what a vacuous harness reports, without changing anything else visible in
`configuration.checks`: with reach-checks off, an assertion made unreachable by a contradictory
`kani::assume` lands in `checks.success` instead of `checks.unreachable`, silently defeating the
vacuity signal this schema exists to carry. `configuration` records the toggles that change how a
result must be read, like this one, so two runs are not misread as comparable when they are not.

**`configuration.checks.ignore_global_asm` and `configuration.checks.extra_pointer_checks`.** Two
more flags recorded for the same reason. `ignore_global_asm` mirrors `--ignore-global-asm`: when
`true`, Kani did not error out on `global_asm!` in the crate, so any behavior reachable only through
that inline assembly is simply absent from the model. A proof against such a crate can pass
vacuously with respect to the assembly's effects, and a consumer needs to know this flag was set to
read the result correctly. `extra_pointer_checks` mirrors `--extra-pointer-checks`: when `true`, Kani
adds obligations for invalid pointers in relational operations and pointer-arithmetic overflow that
are otherwise not checked at all, so two runs that differ only in this flag are checking different
sets of properties, and `checks.total`/`checks.success` are not comparable between them without it.

**`configuration.checks.memory_safety`, `.overflow`, `.unwinding`, and `.undefined_function`.** The
remaining four `checks` bools, each mirroring one of the four `--no-*-checks` flags (all default
`true`; the field name records the check being *on*, not the flag that turns it off):

- `memory_safety` mirrors `--no-memory-safety-checks` (and `--no-default-checks`, which turns off
  every check in this group at once). `false` means Kani passed `--no-bounds-check` and
  `--no-pointer-check` to CBMC, so out-of-bounds accesses and invalid-pointer dereferences are not
  checked at all; a run with this `false` can pass while containing exactly the memory-safety bugs
  Kani exists to find.
- `overflow` mirrors `--no-overflow-checks`. `true` adds CBMC's `--nan-check` on top of Kani's own
  `-C overflow-checks=on` instrumentation; `false` also disables the division-by-zero check
  (`--no-div-by-zero-check`), so with this `false`, arithmetic overflow, NaN production, and division
  by zero are all unchecked.
- `unwinding` mirrors `--no-unwinding-checks`. `false` passes `--no-unwinding-assertions`, so CBMC no
  longer asserts that a loop or recursion bound was sufficient to cover every execution; a bounded
  proof under this flag can silently miss behavior past the bound instead of failing on it.
- `undefined_function` mirrors `--no-undefined-function-checks`. `true` links Kani's model of the
  standard library and asserts on any remaining call to a function with no body; `false` drops such
  calls instead of flagging them, so behavior reachable only through an unmodeled function is simply
  absent from the proof.

Each meets the same "changes which properties are generated" test as `assertion_reach_checks`,
`ignore_global_asm`, and `extra_pointer_checks` above, which is why all seven live in this block
rather than being treated as ordinary CLI trivia.

**`configuration.coverage_enabled`.** Mirrors `--coverage` (`session.args.coverage`, mandatory bool,
default `false`). This flag is precisely what makes `code_coverage` (`COVERED`/`UNCOVERED`) properties
exist at all — the properties this schema explicitly carves out of `checks`, `covers`, and
`n_properties` (see "Rationale" below) — so it meets the `configuration` policy stated next as directly
as `assertion_reach_checks` does, and dropping it would leave a consumer no way to tell "this run has no
coverage properties because none exist in this schema yet" from "this run has no coverage properties
because `--coverage` was never passed." It sits alongside `checks.*` rather than nested under it, since
it is not itself a `checks` toggle.

**Policy for `configuration`.** A flag belongs in this block when it changes *which properties are
generated* or *what a status means*, that is, when two runs differing only in that flag are not
apples-to-apples comparable, or when the flag can make a passing result mean less than it appears to.
The seven `checks` flags and `coverage_enabled` above meet that test; future flags are added under this
rule rather than case by case, and adding one is a minor schema change (a new field). `cbmc_args` is the
explicit catch-all for everything this rule cannot name individually: anything passed straight to CBMC
via `--cbmc-args` can change results in ways Kani cannot introspect, so it is recorded and two runs with
different `cbmc_args` are not assumed comparable. It is recorded *verbatim modulo UTF-8*: arguments are
captured with `to_string_lossy`, so a non-UTF-8 argument is rendered with the U+FFFD replacement
character rather than round-tripped byte-for-byte; this is sufficient for the comparability signal, but a
consumer must not treat the list as an exact argv to replay.

**Failure scenarios.**

- The output path is not writable → the failure is surfaced as an error and the process exits
  non-zero; it is never silently swallowed. The verification *verdict* is computed independently and is
  never rewritten by an export problem, but the export failure itself is real and reflected in the exit
  status. The up-front marker write fails before verification starts; the terminal export runs after the
  harness verdicts have been computed and the normal per-harness output (when enabled) has been rendered,
  but *before* the SARIF artifact and the final summary line, so a terminal export failure both exits
  non-zero and aborts those remaining reporting steps. (Whether an export failure should suppress the
  SARIF write is an implementation question flagged for the code review, not settled here.)
- Any `tools.*` version cannot be determined (the probe binary is missing, refuses `--version`, or
  prints nothing parseable) → that field is `null`. It is never guessed; see "Tool provenance" above.
- A harness times out, is OOM-killed, or CBMC crashes on it → that harness's `outcome.kind`
  (`TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED`) records it and the file is still written. Run completeness is
  *verdict-independent*: as long as every selected harness produced a result, the run's `run_state` is
  still `COMPLETE` (a suite where every harness times out is a complete run of failing harnesses, not an
  incomplete run). `run_state` is only `INCOMPLETE`/`PARTIAL` when Kani itself did not reach the terminal
  write for every selected harness; see the completeness contract below.
- Kani itself crashes (a `kani-compiler` ICE, a panic in `kani-driver`, a `SIGKILL`) → no terminal
  document is ever written for this run, because the single terminal write happens once, at the very
  end, after every harness result is already known (`verify_project` in `kani-driver/src/main.rs`); a
  hard error anywhere before that point — including one raised for a single harness, such as a failed
  CBMC spawn (`run_cbmc_piped`'s `.map_err(...)?` in `kani-driver/src/call_cbmc.rs`), or one propagated
  through `run_until_abort` in `kani-driver/src/harness_runner.rs` — unwinds straight out of
  `verify_project` and skips it. There is therefore no code path that produces a run-level `"CRASHED"`
  document; the only observable residue is whatever the `INCOMPLETE` marker last wrote (left stale,
  never overwritten), or, if the crash preceded even that write, whatever file already existed at the
  path (untouched). See "How `run_state` and `outcome.kind` co-occur" below.
- The `--export-json` path already holds a file → it is **overwritten up front with an atomic
  `run_state: "INCOMPLETE"` marker** once harness verification begins (after the crate is built), so an
  *earlier* run's results cannot then sit at the path to be misread as this run's, and a run that dies
  after that point leaves a file that openly says it did not finish (the mechanism is the completeness
  contract below). This replaces an earlier design that *deleted* the path up front; the trade-off, and
  the residual window before the marker is written, are discussed there.
- The parent directory of the path does not exist → it is created (`create_dir_all`), matching
  `--sarif`. A path that is itself an existing directory fails today only when the write is attempted,
  because the same-directory temp write cannot be created inside a file-shaped target. Adopting this
  RFC includes moving that to the argument-parse-time rejection proposed under *Interaction with other
  flags*, so the failure arrives before any verification work is done, as it does for `--sarif`. The
  difference in the shipped code is an accident of implementation order, not an intentional design.

**The completeness contract.** This proposal replaces the boolean `run_complete` the proof-of-concept
writes today with a richer **`run_state`** field, shown in the example above, and anchors trust on it
rather than on file existence. `run_state` is proposed, not yet implemented; choosing between this
marker design and delete-up-front is one of the open questions below. The write is atomic: Kani writes
the full JSON to a temporary file in the same directory as the target, then `rename`s it onto the target
path (a same-filesystem POSIX rename is atomic), so the target never holds a partial write and a mid-write
kill leaves the previous file in place (at worst an orphaned temp file beside it, never mistaken for the
target; note that a `SIGKILL`-orphaned temp file is *not* swept by a later run, since each export uses a
fresh random name).

Once harness verification begins, Kani writes an atomic **marker**: a document carrying the
pre-verification fields it already knows (schema/tool versions, `machine`, `configuration`,
`harness_selection`) plus `run_state: "INCOMPLETE"`, and *not* the verification `summary` or per-harness
`harnesses[]` results, so a consumer that reads the marker sees no verdict to trust. Writing it
invalidates any stale earlier file at the path and records that a run started here. On normal completion
the terminal write sets `run_state` to one of three **terminal, successfully-published** values:
`COMPLETE` (every selected harness produced a result), `PARTIAL` (some did not, e.g. a `--fail-fast`
run aborted after the first failure), or `NO_HARNESSES_SELECTED` (no harness was selected for
verification at all — see "`NO_HARNESSES_SELECTED` versus a crate with no harnesses at all" below for the
two ways that can happen).

A consumer reads the document in that order, not either one alone: first confirm `schema_version` is one
it supports (see Compatibility policy below — on an unrecognized major, or, pre-1.0, an unrecognized
minor, refuse to parse), *then* read `run_state`, which is not optional for a consumer that means to read
the rest of the file correctly:

- **`run_state == "COMPLETE"`** is the only value a consumer may treat as *complete verification evidence*.
  It is strictly stronger than "a file exists": a `--fail-fast` run that skipped harnesses, or a zero-match
  filter that publishes a well-formed `successful: 0, failed: 0` document, each leaves an existing,
  parseable, clean-*looking* file (an incomplete- or empty-run hazard), yet neither is `COMPLETE`.
- **`PARTIAL` / `NO_HARNESSES_SELECTED`** are *finished* exports and can be trusted to describe accurately
  what they say (a partial run, or an empty selection); they simply must not be read as a complete run.
- **`INCOMPLETE`, or a missing file,** means the export did not finish: an export failure (an unwritable
  path, a full disk) or abnormal termination (an OOM-kill, Ctrl-C) after the marker was written.

Two limits are stated rather than left to be discovered. First, the marker is written only *once
verification begins*, after the crate is built, so a failure *before* that point (a compilation error, a
rejected `--exact --harness` filter) does not write the marker and leaves any pre-existing file at the
path untouched; existence-based staleness is only defeated from the marker onward. Second, the cost of
anchoring on the marker rather than on delete-then-existence: a legacy consumer that checks only the
file's *presence* and never reads `run_state` now finds the marker and fails *open* on a file that carries
no verdict, where the older delete-up-front design failed *closed* on `ENOENT`. `run_state` (the trust
anchor) is distinct from the run-level `outcome.kind`, which is diagnostic only and, as established next,
carries strictly less information than `run_state` at the run level: a consumer gates on `run_state`, not
on `outcome.kind`. The contract assumes a single writer per path: two concurrent runs aiming
`--export-json` at the same file are unsupported, and the last rename wins.

**How `run_state` and `outcome.kind` co-occur.** `outcome` is meaningful only once Kani reaches a
terminal write, and at run level its `kind` has exactly one possible value: `COMPLETED`. The marker
(`run_state == "INCOMPLETE"`) is written *before* that point and does not carry `outcome` at all (see the
presence matrix above) — the field is absent there, not set to some other value. A run-level
`outcome.kind == "CRASHED"` does not exist in this schema: nothing in the writer's design, current or
proposed, is positioned to ever produce it. The terminal write is a single step, at the very end of
`verify_project`, after every harness result and the `summary` are already known; a hard error anywhere
before that point — a `kani-compiler` crash, a panic in `kani-driver`, or even a failure raised while
processing one harness (e.g. a failed CBMC spawn) — unwinds past the point where `outcome` would be set
and the terminal write never happens. What a consumer observes instead is the `INCOMPLETE` marker, left
stale because nothing overwrote it (or, if the crash preceded the marker's own write, whatever pre-existing
file was at the path, untouched — see the two limits below). That staleness — an `INCOMPLETE` marker, or
a missing file, that never turns into a terminal document — *is* the crash signal a consumer reads; it is
not a value it looks up. Every `run_state`/`outcome` combination this schema can actually produce is:

| `run_state` | `outcome` |
|---|---|
| `INCOMPLETE` (marker) | absent |
| `COMPLETE` | `{"kind": "COMPLETED"}` |
| `PARTIAL` | `{"kind": "COMPLETED"}` |
| `NO_HARNESSES_SELECTED` | `{"kind": "COMPLETED"}` |

(`PARTIAL` means some *harnesses* were skipped, e.g. by `--fail-fast`, not that Kani crashed;
`NO_HARNESSES_SELECTED` means Kani ran to completion and simply had nothing selected to verify — both are
finished exports, hence `outcome.kind == "COMPLETED"` in both, per "The completeness contract" above.)
Per-harness `outcome.kind == "CRASHED"` is unaffected by any of this and remains a real, producible value
(see the per-harness presence matrix below): the run itself can finish normally, reach its terminal write,
and still report that CBMC crashed while checking one specific harness — that is a fact about *one
harness*, recorded in a document Kani *did* finish writing, which is a different claim entirely from Kani
itself never reaching the write at all.

**`NO_HARNESSES_SELECTED` versus a crate with no harnesses at all.** Both leave `harnesses[]` empty and
`harness_selection.matched_count == 0`, but they are told apart by `harness_selection.requested_filters`:
non-empty means a `--harness` filter matched nothing in a crate that does define harnesses (an
`unmatched_filters` entry names which); empty means no filter was given and the crate itself defines no
`#[kani::proof]` (or, under `autoharness`, no eligible function) at all. A consumer that wants to
distinguish "my filter is wrong" from "there is nothing here to verify" reads `requested_filters`, not
just `matched_count`.

**A vacuity hole this schema does not close on its own: cover-only vacuity.** The normative and advisory
vacuity predicates above read only `checks.*`; an `unreachable` *cover* statement lands in
`covers.unreachable` and trips neither one. A consumer that wants the symmetric signal for cover
properties should additionally apply
`covers.total > 0 && covers.unreachable.len() == covers.total` per harness, mirroring the normative
`checks` predicate, since this schema does not apply that check on the consumer's behalf.

`null` always means *not measured or not applicable*, never a guess, and is always distinguishable
from `0`, `false`, and `[]`.

**Presence matrix — top-level fields, marker versus terminal document.** The marker and a terminal
document are not the same shape, and the field table above states each field's presence in terms of
"the document", which needs unpacking into these two cases:

| Field(s) | `INCOMPLETE` marker | Terminal document (`COMPLETE` / `PARTIAL` / `NO_HARNESSES_SELECTED`) |
|---|---|---|
| `schema_version`, `kani_commit`, `kani_commit_dirty`, `tools.*`, `machine.*`, `enabled_unstable_features`, `harness_selection.*`, `harness_timeout_s`, `configuration.*`, `target`, `started_at` | present (all knowable before verification begins) | present |
| `run_state` | present, always `"INCOMPLETE"` | present, one of `"COMPLETE"`/`"PARTIAL"`/`"NO_HARNESSES_SELECTED"` |
| `outcome`, `wall_time_s` | absent | present — neither is knowable until Kani reaches a terminal state, so the marker (written *before* that state is known) carries neither |
| `summary`, `harnesses[]` | absent, by design (stated above): a consumer that reads the marker has no verdict to read at all | present (`harnesses[]` may be empty, only under `NO_HARNESSES_SELECTED`) |

A consumer that has already confirmed `schema_version` is one it supports, and then checks `run_state`
before touching anything else, never needs this table — `"INCOMPLETE"` already says "there is no verdict
here, stop." It exists for a consumer inspecting the raw JSON, or writing a parser that must not panic on
a field it expected unconditionally.

**Presence matrix — per-harness fields, by `harnesses[].outcome.kind`.** Every "—" in the harness field
table above implicitly means "on the harness object as CBMC/Kani produced it for that `outcome.kind`";
the following spells that out for the three kinds where it is not simply "always":

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

Nothing in the `TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED` columns is a `0` standing in for "measured, and
nothing found": every unmeasured *count* is `null`, and every unmeasured *identity list* is empty
because there is nothing to name, which is the same "unmeasured is `null`, never `0`" rule stated for
`tools.*` versions, applied consistently here.

## Rationale and alternatives

### Why not extend `--sarif`?

Kani already emits SARIF; it is stable and *not* `-Z` gated, so the bar for a second
machine-readable artifact is higher than it would have been a release ago, and the case for one has to
rest on consumer contracts, not on expressive power. SARIF *could* carry this data: SARIF 2.1.0 has
`result.kind ∈ {pass, fail, informational, notApplicable, review, open}` and arbitrary property bags,
and Kani's SARIF writer skipping covers (`kani-driver/src/sarif.rs:143`) and emitting nothing for
successes (`sarif_level` returns `None` for everything but `Failure`/`Undetermined`/`Unknown`,
`sarif.rs:233`) are choices in *our writer*, not limits of the format. The reasons to keep this a
separate artifact are about who consumes each file and how free each is to change:

- **`--sarif` is stable and must stay valid SARIF; this artifact must be free to break shape while it
  is `-Z` gated.** This is the load-bearing reason. A single file *could* physically carry both, but it
  would then have to be at once a frozen surface conformant to an external standard *and* an unstable one
  we reshape as real consumers teach us what it needs, and we are unwilling to bind the stable artifact
  to the unstable one's churn, which is exactly what merging them would do.
- **The two files serve consumers with opposite needs.** `--sarif` is read by code-scanning tools that
  specifically do *not* want proof, cover and vacuity rows: to them a fully green run *should* be an
  empty `results` array. This artifact is read by CI gates and dashboards for which the all-green run
  is the most common and most important case and those rows are the entire point. Adding them to the
  SARIF file degrades it for its consumers to serve a different one.
- **Vacuity has no native SARIF representation**, only a property bag, a private schema wearing a
  standard schema's clothes. Expressing this schema's central claim in SARIF means smuggling a bespoke
  shape through a standard one, serving neither file's consumers.

Both artifacts are worth having, and this proposal does not change `--sarif`. **How the two avoid
drifting apart:** they are not two renderings of one shared results object. Each is an independent
renderer over the *same upstream* per-harness `VerificationResult` (its parsed `Vec<Property>` from
`cbmc_output_parser`). The source of truth is shared; only the projection differs, and the projections
are deliberately different. That is acceptable because forcing both through one intermediate object
would recreate exactly the stable-to-unstable coupling the first bullet rejects. This shifts the burden
onto the implementation: any *interpretation* of the shared properties (path relativization, solver and
unwind resolution, failed-property classification) should be shared leaf logic called by both writers
rather than re-derived per file, and a cross-artifact conformance test should assert the two agree on the
facts they both carry. This is a development discipline, not a design-level guarantee: the design
shares the *inputs*, and keeping the *interpretation* shared is an ongoing responsibility rather than a
property the shape enforces on its own.

### Summary (`summary.*`)

The field reference above points here for `summary.*`'s meaning, since seven bare integers next to each
other say nothing about how they relate. All seven are run-level totals over `harnesses[]`, always
present in a terminal document, always integers, never `null` there — `summary` is absent entirely in the
`INCOMPLETE` marker (see the presence matrix above), so "always present" is scoped to the shape that has
a `summary` to begin with:

| Field | Definition |
|---|---|
| `total` | `harnesses.len()`. |
| `successful` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "SUCCESS"`. |
| `failed` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "FAILURE"`. |
| `checks_total` | Sum of `harnesses[].checks.total` over `COMPLETED` harnesses only. |
| `checks_success` | Sum of `harnesses[].checks.success` over `COMPLETED` harnesses only. |
| `covers_total` | Sum of `harnesses[].covers.total` over `COMPLETED` harnesses only. |
| `covers_satisfied` | Sum of `len(harnesses[].covers.satisfied)` over `COMPLETED` harnesses only. |

**`total == harnesses.len()`** always, in any document where `summary` is present — that is, any terminal
document, regardless of which of the three terminal `run_state` values it carries: `summary` describes
exactly the harnesses this document actually reports on, not the harnesses that were selected (that
comparison is `harness_selection.matched_count` versus `summary.total`, covered next). The `INCOMPLETE`
marker has no `summary` to apply this identity to, so the identity is vacuous there, not violated.

**How a non-`COMPLETED` harness counts.** `successful` and `failed` both require `outcome.kind ==
"COMPLETED"`, so a `TIMEOUT`, `OUT_OF_MEMORY`, or `CRASHED` harness is deliberately counted in neither:
it has no `outcome.verdict` to be `SUCCESS` or `FAILURE` (see the presence matrix above), and counting it
as a failure would conflate "CBMC found a bug" with "CBMC never got to look." The identity holding is
therefore `successful + failed <= total`, not `==`; the gap `total - successful - failed` is exactly the
count of non-`COMPLETED` harnesses in this run. Equality holds iff every harness in `harnesses[]` reached
`COMPLETED`. The four `checks_*`/`covers_*` sums follow the same rule for the same reason: a
non-`COMPLETED` harness's `checks.total`/`covers.total` is `null` (see the presence matrix above), so it
contributes nothing to these sums rather than contributing `0` as if it had been measured and found empty
— the distinction that matters is "not counted" versus "counted as zero."

**The `matched_count` / `total` / `run_state` invariant.** `harness_selection.matched_count` is the
*pre-verification* match count; `summary.total` is the *post-verification* harness count; `run_state`
says how the two relate:

- `run_state == "COMPLETE"` ⟹ `summary.total == harness_selection.matched_count`: every matched harness
  produced an entry in `harnesses[]` (with whatever `outcome.kind` it reached — `COMPLETE` does not mean
  every harness passed, only that every one was accounted for).
- `run_state == "PARTIAL"` ⟹ `summary.total < harness_selection.matched_count`: some matched harnesses
  have no entry in `harnesses[]` at all, because Kani stopped before running them (e.g. `--fail-fast`
  after the first failure) — they are missing from the array, not present with a `TIMEOUT`/`CRASHED`
  placeholder, since Kani never attempted them.
- `run_state == "NO_HARNESSES_SELECTED"` ⟹ `harness_selection.matched_count == 0 == summary.total`.
- `run_state == "INCOMPLETE"` (the marker): `summary` is absent entirely (see the presence matrix
  above), so this invariant does not apply.

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

`outcome.kind` is deliberately asymmetric: `TIMEOUT` and `OUT_OF_MEMORY` describe a *harness* CBMC could
not finish and never appear at run level, where the only value a terminal document can ever carry is
`COMPLETED` (Kani finished the run); `OUT_OF_MEMORY` is a harness-level-only outcome, and has no run-level
counterpart. A run-level out-of-memory — Kani itself killed — is not representable as a run-level
`outcome.kind` value at all, since Kani never reaches the terminal write that would set `outcome`; it
surfaces instead as a stale `INCOMPLETE` marker (see "How `run_state` and `outcome.kind` co-occur" above).
`OUT_OF_MEMORY` at the harness level is inferred from a `137` exit (SIGKILL, the usual OOM-killer signal),
not a direct memory measurement; `TIMEOUT` arises only under `--harness-timeout`.

`failure_kind` is scoped to `outcome.kind == "COMPLETED"`: a `TIMEOUT`, `OUT_OF_MEMORY`, or `CRASHED`
harness never reaches `determine_failed_properties` at all (CBMC produced no property list to classify),
so the field is **omitted** on those harnesses, the same "absent, not `null`" treatment as
`outcome.verdict` above (see the presence matrix in "The completeness contract" below for the full
picture). On a `COMPLETED` harness, `failure_kind` is the *raw* failure classification from
`determine_failed_properties` (verbatim from `FailedProperties`: `NONE`, `PANICS_ONLY`, `OTHER`, or
`ERROR`), computed independently of the interpreted `outcome.verdict`. The relationship between the two
is a small truth table (`kani-driver/src/call_cbmc.rs`'s `verification_outcome_from_properties`), not a
single rule, and it turns on `attributes.should_panic`:

| `attributes.should_panic` | `failure_kind` | `outcome.verdict` |
|---|---|---|
| `false` | `NONE` | `SUCCESS` |
| `false` | `PANICS_ONLY`, `OTHER`, or `ERROR` | `FAILURE` |
| `true` | `PANICS_ONLY` | `SUCCESS` |
| `true` | `NONE`, `OTHER`, or `ERROR` | `FAILURE` |

For `should_panic == false`, `failure_kind == "NONE"` and `outcome.verdict == "SUCCESS"` are equivalent
(`NONE ⟺ SUCCESS`): there is nothing to reconcile. For `should_panic == true`, the two vocabularies
disagree in **two** places, not one, and disagree in *both directions*:

- A harness that panics as expected has `outcome.verdict == "SUCCESS"` (a `#[kani::should_panic]`
  proof doing exactly what it should) yet `failure_kind == "PANICS_ONLY"`, not `NONE`: its panic check is
  a `FAILURE`-status property that the verdict interprets as expected, per the vacuity predicates below.
- A harness that was expected to panic but did not has `outcome.verdict == "FAILURE"` yet
  `failure_kind == "NONE"`: every property the harness actually contains held, but the missing panic
  itself is what the should-panic check fails on, so `determine_failed_properties` — which only ever
  looks at property statuses — finds nothing to classify as a failure while the interpreted verdict is
  `FAILURE` regardless.

So `should_panic == true` is exactly the condition under which `failure_kind` and `outcome.verdict` are
*inverted* relative to the `should_panic == false` case (`SUCCESS ⟺ PANICS_ONLY` takes the place of
`SUCCESS ⟺ NONE`); neither value is uniformly "stronger" than the other across both cases, and a
consumer must read `attributes.should_panic` before drawing any conclusion from `failure_kind` alone.
The `status` values are the closed `CheckStatus` set; the `checks.*`/`covers.*` `other` bucket is *not* a
home for unknown statuses
but for known `CheckStatus` values that do not map to a named bucket in that domain, for instance a
cover-only status (`SATISFIED`/`UNSATISFIABLE`) surfacing among checks, which keeps the partition
exhaustive without implying the status vocabulary is open. Should CBMC ever emit a status `CheckStatus`
does not model, that is a parser-level concern resolved by adding a variant. That is a *major* schema change
under the compatibility policy below, since `status` is a closed enum a consumer may match exhaustively,
not something the `other` bucket silently absorbs.

**`attributes.kind` is closed; the solver names are explicitly open.** `attributes.kind` is
`HarnessKind`, a fixed 3-variant Rust enum (`Proof`, `ProofForContract { target_fn }`, `Test`); Kani
adding a fourth kind of harness is exactly the closed-enum case above, and is a major change under the
compatibility policy below. The solver-name fields (`attributes.solver`, `resolved_solver`,
`tools.solvers[].name`) are the opposite case, and are the vocabulary the compatibility policy's minor-change
allowance for a new enum value actually describes: `CbmcSolver` already ships with an open escape hatch
(`Binary(String)`, any path a consumer has never seen), so a consumer of these fields is *already*
required to treat an unrecognized name as "some solver, spelled this way" rather than to exhaustively
match a closed set — there is no way to write an exhaustive match against a field that already accepts
an arbitrary path string today. Kani adding a new *named* built-in solver (a new `CbmcSolver` variant
alongside `Bitwuzla`/`Cadical`/`Cvc5`/`Kissat`/`Minisat`/`Z3`) is therefore a **minor** change: a
consumer built against this schema already has to handle a solver name it doesn't recognize, so a new
name changes nothing about how such a consumer must be written, unlike a new `status` or `failure_kind`
variant, which a consumer is *entitled* to assume can never appear.

**Casing.** The file mixes three conventions, and one rule explains all three: *a value keeps the
serialization of the Rust type it comes from, and this schema does not fork types to re-case them.*

- **snake_case keys**, because the schema reuses `kani_metadata` and `cbmc_output_parser` types directly
  (`HarnessAttributes`, `AssignsContract`, `CheckStatus`) rather than parallel copies. Re-casing keys
  would mean forking those types, recreating the duplication debt tracked in
  [#3541](https://github.com/model-checking/kani/issues/3541), or changing their serialization and
  breaking existing `.kani-metadata.json` consumers. This "reuse verbatim" story is free for the three
  types named above, which already derive both `Serialize` and `Deserialize` today (they cross the
  compiler-to-driver `.kani-metadata.json` boundary in both directions already). It is not free for
  every candidate type: `cbmc_output_parser::Property`, `PropertyId`, and `SourceLocation` currently
  derive only `Deserialize` (they parse CBMC's `--json-ui` stream but nothing has ever serialized them
  back out), so reusing *those* directly would require adding a `Serialize` derive as part of
  implementing this RFC, not merely embedding an already-`Serialize` type. This is exactly why
  `failed_properties[]`/`unsupported_constructs[]`/`checks.other[]`/`covers.other[]` are instead their
  own purpose-built export structs (see "Field reference" above) rather than a direct embedding of
  `Property`.
- **SCREAMING_SNAKE_CASE enum values** (`outcome.kind`, `verdict`, `failure_kind`, `run_state`, every
  `status`), mirroring the reused CBMC-status types (`CheckStatus`, `FailedProperties`). (Precisely:
  `CheckStatus` is `#[serde(rename_all = "UPPERCASE")]`, which coincides with SCREAMING_SNAKE only because
  its variants are single words; the newly-introduced multi-word values such as `NO_HARNESSES_SELECTED`
  must therefore use an explicit `SCREAMING_SNAKE_CASE` rename, not `UPPERCASE`, to match.) The
  newly-introduced enums (`Outcome`, `verdict`, `run_state`) were given this casing on purpose, so a
  consumer sees one value convention across the whole file rather than a seam between reused and new types.
  (`run_state` is proposed in this RFC's completeness contract and does not exist in the shipped
  writer; any implementation of it uses this casing, and any future value set introduced into the
  file follows this same single rule rather than carrying its Rust type's default casing.)
- **PascalCase, object-shaped**, for the two embedded `kani_metadata` attribute enums below, which keep
  their own serde derivation untouched for the same no-forking reason.

**Fields that are not always strings.** Two reused `kani_metadata` enums have non-unit variants and so
serialize as *objects*, not strings; a consumer must not assume otherwise:

- `harnesses[].attributes.kind` is `"Proof"` or `"Test"`, but `{"ProofForContract": {"target_fn": "…"}}`
  for a contract-proof harness.
- `harnesses[].attributes.solver` is `"Cadical"`, `"Z3"`, etc. for a named solver, but `{"Binary": "…"}`
  for a custom solver path.

These are the *requested* attributes, carried verbatim from `.kani-metadata.json`; the resolved
counterparts (`resolved_solver`, `resolved_unwind`) are plain scalars: a string or number, or `null`
when not applicable (as the example's `resolved_unwind` shows), never the object forms above, and sit
at the top level of each harness for a consumer that only wants what actually ran.

**Reconciling the three solver spellings.** `attributes.solver` (the requested attribute, PascalCase:
`"Cadical"`, `"Z3"`, or `{"Binary": "…"}`), `resolved_solver` (the effective solver, lowercase:
`"cadical"`, `"z3"`, or the literal binary path string), and `tools.solvers[].name` are not three
independent vocabularies: `resolved_solver` and `tools.solvers[].name` are always spelled identically
(both come from the same resolution — CLI `--solver` > harness attribute > `--cbmc-args` override >
default — see "Tool provenance" above), and only `attributes.solver` differs, because it is the
*requested*, not resolved, value and keeps `CbmcSolver`'s own PascalCase enum casing rather than being
re-cased to match. A consumer comparing "what was asked for" against "what ran" compares
`attributes.solver` against `resolved_solver`, not against `tools.solvers[].name`, which exists to
answer a different question ("what solvers did this run use, in total").

What is worth adopting from `kani list` is the *substance* of its versioning idiom: a tool version and
a schema version as separate fields. This proposal does that; see **Compatibility policy** below.

### Compatibility policy

`schema_version` is a semantic version. It exists so the shape can be corrected while it is still being
learned from real consumers, and it makes the rules of that correction explicit rather than implied:

- **Minor (backward-compatible) changes** bump the minor version: adding a field, or adding a new value
  to one of the **explicitly open** vocabularies named in "Value domains" above (today, just the solver
  names: `attributes.solver`, `resolved_solver`, `tools.solvers[].name`) — a consumer of an open
  vocabulary already treats an unrecognized value as "valid, but unfamiliar" rather than exhaustively
  matching it, per the forward-compatibility rule below, so such a consumer keeps working across a minor
  bump without change. Every other enum in this schema is closed (see the next bullet); this allowance
  does not apply to them.
- **Major (breaking) changes** bump the major version: renaming or removing a field, changing the meaning
  or type of an existing field, adding a variant to a *closed* enum (`outcome.kind`, `verdict`,
  `failure_kind`, `run_state`, `status`, `attributes.kind`) since a consumer may exhaustively match those,
  or moving a status out of the
  `checks.*`/`covers.*` `other` bucket into a named bucket. Adding a variant to a closed enum is a
  deliberately high bar (it forces, say, a new `failure_kind` to wait for a major bump), chosen so these
  core vocabularies do not change casually; consumers are nonetheless encouraged to keep a default arm
  even against the closed sets.
- **Forward-compatibility rule for consumers: ignore unknown fields.** This is the entire mechanism by
  which the schema grows within a major version; a consumer that rejects unknown fields forfeits it.
- **On an unknown major version, refuse to parse** rather than guess: a major bump means a field the
  consumer relies on may have changed meaning, and a silent misread is exactly the failure mode this
  schema exists to remove.
- **`warnings` is outside these guarantees** (see its own section): its presence and shape are
  contractual; its contents, wording and size are not, and are not covered by `schema_version`.

**This schema is deliberately open to growing richer, not a closed shape frozen at v1.** Today's fields
are the honest core this RFC could verify against the shipped writer, not a ceiling on what the format
will ever carry. The additive minor-version rule above — new fields land without a major bump, and a
conformant consumer already ignores fields it does not recognize — is the mechanism by which that growth
happens: as real downstream evidence consumers (CI gates, dashboards, external verification-tracking or
evidence formats that want to ingest a Kani run as one input among several) show up with a concrete need
this v1 does not yet meet — finer per-harness provenance, additional resource or environment data,
tighter linkage between a property and the evidence that discharged it — the schema is expected to grow
to carry it, one additive minor version at a time, rather than being redesigned from scratch or left to a
second, competing artifact. Nothing about this shape should be read as asserting that today's fields are
all a downstream consumer will ever need from Kani.

**Semver-zero, while it is `-Z` gated.** Until stabilization the schema is `0.x`, and the minor/major
*distinction above describes intent* rather than a promise: a `0.x` consumer must match an exact minor
(or an explicitly supported range) instead of relying on forward-compatibility, because a breaking change
may land on a `0.x` minor bump. The backward-compatibility guarantee for minor changes takes effect at
`1.0`. In practice, "match an exact minor" means a consumer or test asserts on `schema_version` itself
(e.g. `schema_version == "0.1.0"`, or a small explicit allow-list of minors it has verified against)
before trusting anything else in the document, and treats any other `0.x` value the same as an unknown
major: refuse to parse rather than guess. This is a slightly stronger rule than the `1.0`-and-later
policy above, and is why `schema_version` is checked first, not last.

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

Symex time, VCC counts and solver time exist only inside CBMC's free-text messages. Extracting them
would mean pattern-matching human-readable output, which is the fragility this RFC exists to remove,
so doing it inside the fix would be self-defeating. `benchcomp` doing exactly this today is evidence
the need is real; the right fix is structured data from CBMC, not more scraping. A `warnings` field
does carry CBMC's messages verbatim, explicitly as non-contractual free text: CBMC's `--json-ui`
stream tags each message with a `messageType` (for example `"WARNING"`), and this schema surfaces
exactly the `WARNING`-typed messages already present in that stream, for instance the SAT backend's
`warning: ignoring forall`, emitted when a `kani::forall!`/`kani::exists!` over a symbolic bound
cannot be discharged by the default solver. [PR #4719](https://github.com/model-checking/kani/pull/4719),
opened independently by a CBMC maintainer, surfaces this same class of dropped-quantifier warning
prominently, corroborating that this is a real gap, not a hypothetical one. These strings are CBMC's
internal-IR pretty-printer output verbatim, can run to several kilobytes with no promised structure,
and are not parsed or classified by Kani; a consumer must treat each entry as an opaque string to
display or log, not a machine-readable field to pattern-match on, the same caution this RFC applies
to CBMC's rendered text everywhere else. Two consequences the schema states rather than leaves to be
discovered:

- **`warnings` is outside the `schema_version` compatibility guarantees.** The field's presence and its
  `[{ "message": …, "truncated": …, "original_chars": … }]` shape are contractual; the *contents* of
  `message` (wording, structure) are not, and must not be pattern-matched or assumed stable across
  versions.
- **`warnings` is bounded, with explicit, structural truncation markers.** One CBMC warning already runs
  to ~11.6 KB (the example above), and the standard-library-scale runs this RFC targets can emit many;
  left unbounded the field could dwarf everything else in the file and defeat the CI consumers it exists
  to serve. So each `message` is capped at a fixed, implementation-defined length and truncated on a
  character boundary; the fact and extent of that truncation are carried as sibling fields on the same
  entry, `truncated: bool` and `original_chars: integer | null` (the pre-truncation character count,
  `null` exactly when `truncated` is `false`), rather than as a suffix appended inside `message` itself
  — a consumer that wants to know whether a warning was cut, or by how much, reads a field, not a
  string. The per-harness `warnings` array is separately capped at a fixed count, with a sibling
  `warnings_truncated` integer on each harness (`0` when nothing was dropped, shown in the example)
  recording how many *entries* (not characters) were omitted. A consumer can then always tell "no more
  warnings" from "we stopped recording", and never infers completeness from absence. The structural
  fields here — `truncated`, `original_chars`, and `warnings_truncated` — are schema-versioned; the
  warning text `message` itself carries is not.

### What if we do nothing?

Consumers keep scraping, including Kani's own. The status quo works until an output string changes,
and then the breakage is silent: a grep that matches nothing looks exactly like a run with nothing
to report. The vacuity problem in particular stays invisible to automation. Doing nothing now has a
second cost: the shipped v1 document becomes the de-facto contract, unversioned and unspecified.

## Open questions

- **Does this schema supersede the shipped v1 shape?** #4472's document (count summaries and
  per-harness detail arrays joined on `pretty_name`, `Debug`-cased status values, gated
  `-Z unstable-options`) is on `main` today, and this RFC's schema is not that shape. The flag is
  unstable, so the shape can change without a deprecation cycle. The question is whether to migrate
  the shipped writer to this schema now, or to redraw this RFC around the shipped shape. The
  sections above are written for the former.
- **Processed or raw view?** The property list Kani renders is already post-processed: reachability
  checks removed, some descriptions rewritten, successful checks demoted on a fundamental failure.
  This proposal exports the **processed** view, so the file agrees with Kani's exit code and text
  output. Should the raw CBMC view also be available, or is that CBMC's `--json-ui` to provide?
- Should a JSON Schema document ship alongside? That likely means a `schemars` dependency, which is
  not currently in the workspace, a real dependency decision rather than something to add quietly.
- **Completeness: `run_state` marker, or delete the target up front?** This proposal writes a marker.
  Deleting up front destroys a user-named path before verification starts, and fails *closed* on
  `ENOENT` for a consumer that only checks whether the file exists; the marker fails *open* for that
  same consumer. The window where this matters is narrower than it first sounds: a stale `COMPLETE`
  from a previous run can only be read during the pre-marker **build** window (before the crate is
  built and harness verification begins) — once verification begins, the marker write invalidates the
  stale file immediately, replacing it with `INCOMPLETE`, not leaving it in place "until the new one
  finishes." Both become one-way doors as soon as anything consumes the file, which is why this is
  asked rather than decided.
- Should this cover the `autoharness` subcommand's results, whose `chosen`/`skipped` classification
  currently has no machine-readable form?
- Coverage results: include here, or leave with `kani-cov`?
- **Shipped fields dropped or transformed in this shape — keep, drop, or make mandatory?** #4472's
  document (`create_metadata_json`, `create_harness_metadata_json`, `process_cbmc_results` in
  `kani-driver/src/frontend/schema_utils.rs`) carries a number of fields this schema does not restore
  as-is. Grouped by disposition:
  - **Dropped, no successor field today:** `build_mode` (`"debug"`/`"release"`, from
    `create_metadata_json`); `harnesses[].mangled_name` (see "Why not `mangled_name`" above); the
    source range's `end_line` (only `original_start_line` survives, as `harnesses[].line`); `goto_file`
    (the generated modeling file path); per-check `description`/`location` for every property, not just
    the failed and `other`-bucketed ones (the shipped shape records full detail for every check via
    `PropertyCounts`/`create_verification_result_json`; this schema records identities only for
    `success`/`unreachable`/`undetermined`/`error`/`unknown`, and full records only for
    `failed_properties`/`unsupported_constructs`/`other[]` — see "Field reference" above); CBMC's OS
    banner string (`cbmc_metadata.os_info`, distinct from the new run-level `machine.os`, which is
    Kani's own host OS, not CBMC's reported one); and CBMC execution statistics (`cbmc_stats`: symex
    time, VCC counts, solver time — already addressed by "Why is CBMC statistics data excluded?" below
    and by the "Per-check timing and resource data" future-work item, restated here only so this bullet
    is a complete inventory).
  - **Dropped, and soundness-relevant — now `harnesses[].is_bounded`, a *mandatory* field:** resolved
    above, in "The bounded-result flag (`is_bounded`)"; no longer an open question.
  - **The effective `--object-bits` value stays an open question.** `effective_object_bits`
    (`effective_object_bits` in the shipped code, computed per run in
    `kani-driver/src/frontend/schema_utils.rs`) is soundness-adjacent in the same direction as
    `is_bounded` — an object-bits-limited pointer-encoding scheme narrows what a `SUCCESS` verdict
    covers — but it is not promoted alongside it, for a reason specific to this field rather than to the
    soundness argument: `effective_object_bits` is presently a per-run CBMC argument
    (`VerificationArgs::cbmc_object_bits`, reconciled against any `--object-bits` inside `--cbmc-args`),
    not a per-harness *attribute*, so unlike `is_bounded` it has no obvious per-harness home in this
    schema today, and different harnesses in one run cannot currently be told apart by their own
    effective bound. On triage of the shipped issue tracking this gap
    (model-checking/kani#4731), a Kani maintainer (feliperodri) classed `object_bits` as a niche
    concern relative to the harness-level `is_bounded` fix, which is the basis for treating it
    differently here rather than as a parallel promotion. It remains this RFC's position that leaving it
    dropped is a real gap, not an endorsement of the status quo: adopting this RFC should include filing
    (or updating) a tracking issue for a per-harness `object_bits` home, to be resolved before, or as
    part of, this schema's stabilization, per the "What must be true before this leaves `-Z`" list above.
  - **`coverage.enabled` is resolved — now `configuration.coverage_enabled`, a *mandatory* field:** see
    the `configuration.coverage_enabled` entry above; no longer an open question. (Restoring it under the
    stated `configuration` policy, rather than dropping the one flag that explains why `code_coverage`
    properties are invisible to this schema while keeping three other flags for the identical reason, is
    why this stopped being optional.)
  - **The Cargo provenance `project.workspace_root` / `output_dir`, and `harnesses[].file`'s base
    directory.** These two are linked: `harnesses[].file` is written relative to the directory Kani was
    invoked from (`relativize_path`, via `std::env::current_dir()`), not to any field this document
    itself carries. For a `cargo kani` run from the workspace root, invocation directory and
    `workspace_root` coincide and the distinction is invisible; for a standalone `kani-driver` invocation,
    or a script that changes directory before invoking Kani, they need not. Restoring `workspace_root` (or
    equivalently, defining `file` as workspace-relative rather than invocation-relative and computing it
    that way) is what makes `file` machine-resolvable by a consumer that does not itself know, or trust,
    the directory the run happened to be launched from. `output_dir` (the compiler's `target/<triple>/...`
    output directory) has no comparable soundness argument and can stay dropped.
  - **`is_ctor_based`** (the autoharness constructor-args flag, orthogonal to the `chosen`/`skipped`
    question above) is confirmed present in the shipped writer and in `HarnessMetadata` today
    (`kani_metadata::HarnessMetadata::is_ctor_based`); the removal claim above is accurate, not
    speculative. Unlike `is_bounded`, it stays dropped-but-open rather than promoted: cheap to restore,
    called out as a decision rather than a silent regression, but not soundness-mandatory in the same
    direct way `is_bounded` is (see "The bounded-result flag" above) or `object_bits` is argued to be
    just above, since a constructor-based generation strategy narrows which *values* are covered rather
    than silently mislabeling an unrestricted result as one.
- **Final flag name.** `--export-json` versus `--results-json` (see the flag-name note under User
  Experience). The CLI spelling is provisional and this is a stabilization-blocking decision: the name
  must be settled before the flag leaves `-Z`.

(The earlier open question "which other flags belong in `configuration`?" is now resolved as the stated
**Policy for `configuration`** above: a flag is recorded when it changes which properties are generated
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
- **Per-harness peak memory.** A `getrusage(RUSAGE_CHILDREN)`-based approach was prototyped and
  rejected: `ru_maxrss` is a process-wide running *maximum*, not a per-child figure, so the result is
  order-dependent (only a harness that out-peaks every predecessor gets a value; a later, lighter
  harness typically reads `null` even under real memory pressure) and is not attempted at all under
  `--jobs`, where the counter is shared across concurrently-running siblings. An accurate per-harness
  figure needs per-child accounting instead: e.g. `wait4()`-based rusage per child process, or a
  per-child cgroup with its own `memory.peak`, neither of which this schema attempts today; CI
  consumers infer OOM today from exit code 137 (already visible via `outcome.kind == "OUT_OF_MEMORY"`).
