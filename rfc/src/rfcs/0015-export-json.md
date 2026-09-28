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

This RFC is the complete normative reference. Informative-only explanations, implementation notes and rationale are collected in the accompanying PR's “Extended reference (informative)” and add no normative requirements.

## User Impact

Issue [#942](https://github.com/model-checking/kani/issues/942), cited by a `kani-driver` TODO, requests
machine-readable output. Kani's `tools/benchcomp/benchcomp/parsers/kani_perf.py` regex-parses stdout for
`Runtime Solver`, `Runtime Symex` and `Generated N VCC(s)`; other consumers grep verdicts, cover counts
and verification time. Formatting changes can silently make these matches disappear.
This helps CI dashboards detect regressions, large verification efforts identify meaningless proofs,
industrial pipelines collect cost/outcome, and Kani's own tooling stop scraping.

### The specific gap: a proof can pass while proving nothing

A contradictory `kani::assume` makes subsequent assertions unreachable, yet the harness reports
`VERIFICATION:- SUCCESSFUL` and exits 0, even for `assert!(x != x)`. Text reports unreachable checks;
exit-code-only consumers cannot distinguish a non-vacuous proof. Default reachability checking demotes
unreachable assertions from `Success` to `Unreachable`. **Shipped JSON already carries these processed
statuses and an unreachable count** through `VerificationResult.results`; this RFC defines property
identities, vacuity predicates and completeness.
**Downside.** A schema limits change; the unstable flag and explicit version allow consumer-driven corrections.

### Relationship to the shipped implementation (#4472)

[PR #4472](https://github.com/model-checking/kani/pull/4472), merged 2026-08-12, ships the flag behind
`-Z unstable-options`, with count summaries and detail arrays joined by `harness_id == pretty_name`
and Rust `Debug`-cased statuses. This RFC assumes migration to the proposed shape; superseding the
shipped shape remains open. Number `0015` is free: #4472 merged without an RFC file.

## User Experience

```
cargo kani -Z export-json --export-json results.json
```

This command uses the dedicated gate proposed by this RFC. The shipped flag uses
`-Z unstable-options`; see “Name and gate” below.

The flag is additive: existing rendered output is unchanged and the file is written in addition to
it; omitting the flag changes nothing. One combination is rejected: `--output-format=old`
bypasses CBMC's structured JSON entirely (`run_terminal_timeout` mocks a success/failure result with
zero properties and treats a timeout as success), so `--export-json` under it would produce a
well-formed file indistinguishable from the results of an actual successful verification run, even for a run that timed out. The argument
parser rejects the combination with an explicit error before verification starts.

### Interaction with other flags

- **`--only-codegen`:** rejected, like `--sarif`; no verification results exist.
- **`--jobs N`:** allowed. Sort `harnesses[]` by `(crate_name, file, line, name)`, not completion order.
  Determinism covers structure and ordering; timestamps, wall and verification times remain volatile.
- **`--output-into-files`:** independent; rendered text and the JSON file have separate destinations.
- **Multi-crate `cargo kani`:** one file per invocation covering every harness in the run, distinguished
  by `crate_name`; later crates do not overwrite earlier results.
- **Path `-`:** a literal filename. Stdout streaming is deferred because it has no atomic-rename target.
- **Existing directory as path:** reject at argument parsing, as `--sarif` does; shipped code fails at write time.

**Name and gate.** `--results-json` would identify the artifact more clearly; `--export-json` stays
provisional to match the PoC and #942. This RFC proposes dedicated `-Z export-json` gating so the
artifact can stabilize or be dropped independently. Migration must register `ExportJson` in
`kani_metadata::UnstableFeature`, change argument validation, and update tests and documentation.

### Example

This proposed-schema example is illustrative, not a measured run or verbatim PoC output.
The PoC's synthetic test supports the illustrated semantics.

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
    "cbmc": "6.10.0 (cbmc-6.10.0)"
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
      "extra_pointer_checks": false,
      "assert_contracts": true,
      "prove_safety_only": false
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

`outcome.verdict` is `SUCCESS` and the exit code is `0`; `checks.unreachable` identifies both
properties that could not be exercised.

### Reading the results

**Selection.** `name` is the fully qualified `pretty_name` (module path included; bare name at crate root).
Join on `(crate_name, name)`: names are unique only within a crate. Rerun with
`-p <package> --harness --exact <name>`; plain `--harness` matches substrings. The underscored rustc
`crate_name` needs mapping to the Cargo package name when they differ. Generated harnesses cannot be
selected by these flags: never use their name as `--harness`. Contract-proof harnesses are selectable
by their own function path, not their contract target.

**Bounded results.** Mandatory `is_bounded` is true exactly when autoharness generated the harness and
bounded at least one argument because `--autoharness-bounded-arguments` was passed and its type lacked
an unbounded `Arbitrary` strategy. Success then proves only inputs within that bound. Manual harnesses
and generated ones needing no bounded argument report false. This is independent of `is_ctor_based`;
`is_bounded == false` alone does not establish unrestricted inputs.

The predicates use abbreviated per-harness `outcome.verdict` and `checks.*`; both are inapplicable
when `configuration.checks.assertion_reach_checks` is false:

- **Vacuous pass (normative).** `verdict == "SUCCESS" && checks.total > 0 && checks.unreachable.len() ==
  checks.total`: every check in a passing harness is unreachable, so the proof examined nothing.
  (`success == 0` would be unsound — a passing `#[kani::should_panic]` harness has a `FAILURE` panic
  check; `checks.total > 0` excludes a checkless harness; the `SUCCESS` guard excludes failing ones.)
- **Vacuity-suspect (advisory).** `verdict == "SUCCESS" && !checks.unreachable.is_empty()`: a passing
  harness with *any* unreachable check, catching *partial* vacuity the normative rule misses. Advisory
  because it can flag deliberate unreachability (a deliberately dead assertion satisfies it). It reads
  only `checks.unreachable`; an unreachable *cover* is recorded in `covers.unreachable` and does not satisfy it.

With reach-checks off, contradictory assumptions can leave assertions in `checks.success`: this run
cannot report vacuity through `checks.unreachable`. For cover-only vacuity, consumers should additionally
apply `covers.total > 0 && covers.unreachable.len() == covers.total` per harness; this schema does not.
Mandatory `configuration.coverage_enabled` records whether `--coverage` generated excluded `code_coverage` properties.

**Completeness.** Proposed `run_state` replaces PoC `run_complete`; marker versus deletion remains open.
Each write puts full JSON in a temporary file in the target directory, then atomically renames it onto
the target (same-filesystem POSIX rename). A mid-write kill leaves the previous target and possibly an
orphaned temp file, never a partially written target. After building and selecting harnesses, when
verification begins, an atomic `INCOMPLETE` marker replaces any earlier file. It contains known
pre-verification fields, without `outcome`, `wall_time_s`, `summary` or `harnesses[]`.
First check `schema_version` (refuse unsupported major or, pre-1.0, minor), then `run_state`:

- Only `COMPLETE` is complete verification evidence: every selected harness has an entry, regardless of outcome.
- `PARTIAL`/`NO_HARNESSES_SELECTED` are trustworthy finished exports of missing results/empty selection, not complete evidence.
- `INCOMPLETE` or a missing file means export did not finish (write failure or abnormal termination).

Compilation errors/rejected filters precede the marker and leave existing files untouched; stale results
are invalidated only from the marker onward. Existence-only consumers fail open on a verdict-less marker;
deletion fails closed on `ENOENT`. Concurrent writers to one path are unsupported; last rename wins.
`NO_HARNESSES_SELECTED` means an unfiltered project/workspace with no selectable proof harness or eligible
autoharness function: `requested_filters == []`, `unmatched_filters == []`, `matched_count == 0`.
A wholly unmatched filter set, or any unmatched `--exact` filter, errors non-zero before export (#4743).
Nonempty `unmatched_filters` occurs only without `--exact` in `COMPLETE`/`PARTIAL` when another filter matched.

## Rationale and alternatives

### Why not extend `--sarif`?

Stable, ungated SARIF could carry this data; skipping covers/successes is Kani's writer choice.
Separation avoids coupling stable SARIF to unstable schema changes and keeps proof/cover/vacuity rows
for CI while scanners retain empty results on success. SARIF has no native vacuity representation, only property bags.
`--sarif` stays unchanged. Both writers project upstream `VerificationResult`/`Vec<Property>`, not a shared
results object. Shared path/solver/unwind/failure interpretation and cross-artifact tests should keep
them consistent; these are development practices, since shared inputs do not guarantee shared interpretation.

### Summary (`summary.*`)

| Field | Definition |
|---|---|
| `total` | `harnesses.len()`. |
| `successful` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "SUCCESS"`. |
| `failed` | Count of harnesses with `outcome.kind == "COMPLETED" && outcome.verdict == "FAILURE"`. |
| `checks_total` | Sum of `harnesses[].checks.total` over `COMPLETED` harnesses only. |
| `checks_success` | Sum of `harnesses[].checks.success` over `COMPLETED` harnesses only. |
| `covers_total` | Sum of `harnesses[].covers.total` over `COMPLETED` harnesses only. |
| `covers_satisfied` | Sum of `len(harnesses[].covers.satisfied)` over `COMPLETED` harnesses only. |

Non-`COMPLETED` harnesses count in neither `successful` nor `failed`: `successful + failed <= total`,
with the gap exactly their count. They contribute nothing to check/cover sums; null is not counted as zero.
Reported `summary.total` and pre-verification `harness_selection.matched_count` obey:

- `COMPLETE` ⇒ `total == matched_count`; `PARTIAL` ⇒ `total < matched_count`.
  Missing entries are not crash/timeout placeholders and do not establish whether harnesses ran (#4744).
- `NO_HARNESSES_SELECTED` ⇒ `matched_count == 0 == total` and `requested_filters == []`.
- `INCOMPLETE` has no `summary`; these identities do not apply.

### Compatibility policy

`schema_version` is a semantic version:

- **Minor:** add fields or explicitly open solver names (`attributes.solver`, `resolved_solver`).
  Consumers must accept unfamiliar solver names as valid; every other enum is closed.
- **Major:** rename/remove fields, change meaning/type, add closed-enum variants (`outcome.kind`,
  `verdict`, `failure_kind`, `run_state`, `status`, `attributes.kind`), or move statuses from `other`
  into named buckets. Closed-enum additions wait for a major bump; default arms remain encouraged.
- **Ignore unknown fields:** rejecting them forfeits forward compatibility within a major version.
- **Refuse unknown majors:** existing fields may have changed meaning.
- **Warnings:** presence/shape are contractual; contents, wording and size are outside these guarantees.

Consumer needs can add fields in minor versions without redesign or a competing artifact.
**Semver-zero:** while `-Z` gated, `0.x` minor/major distinctions express intent, not a promise.
Breaking changes may land in minor bumps. Assert an exact minor (e.g. `== "0.1.0"` or a small verified
allow-list); refuse other `0.x` values as unknown majors. Backward compatibility starts at `1.0`.
Check `schema_version` first.

Before leaving `-Z` (see `rfc/src/template.md`):

1. Resolve every open question, particularly processed versus raw view.
2. Obtain at least one release cycle of feedback from a real verdict-level consumer, in-tree or out-of-tree.
   `benchcomp`'s `kani_perf` parser is not that consumer; its migration is not a prerequisite. It needs
   solver/symex time, VCC counts and program-step figures, excluded here, and moves only in the
   follow-up adding structured CBMC statistics.
3. Settle whether to ship a JSON Schema document (`schemars`).

### Why a new artifact rather than extending an existing one?

`kani list --format json` is pre-verification metadata; coverage output is per-region data;
`--output-into-files` writes the same rendered text, split per harness; `.kani-metadata.json` is the
compiler-to-driver channel. The per-harness text files do carry verification outcomes, but none of
these artifacts provides the structured run-and-verdict contract proposed here.

### Why is CBMC statistics data excluded?

Symex time, VCC counts and solver time require parsing CBMC free text. `benchcomp` needs structured
CBMC statistics; moving that scraping into Kani would retain the same fragility.
`warnings` carries `WARNING`-typed `--json-ui` messages as opaque display/log text; consumers must not
pattern-match it. #4719 interprets ignored-quantifier warnings internally, but does not version their text.
Each message has a fixed implementation-defined character-boundary cap, with `truncated: bool` and
`original_chars: integer | null` (pre-truncation count, null exactly when not truncated).
The array has a separate fixed count cap; `warnings_truncated` counts omitted entries, zero if none.
These structural fields and warning presence/shape are schema-versioned; message text is not.

### What if we do nothing?

Consumers keep scraping, including Kani's own. The status quo works until an output string changes,
and then the breakage is silent: a grep that matches nothing looks exactly like a run with nothing
to report. Exit-code-only consumers still miss vacuity, although shipped JSON already exposes
unreachable statuses. Doing nothing now has a second cost: the shipped document becomes the de-facto
contract. It already has `metadata.version: "1.0"`; this RFC proposes the explicit compatibility and
completeness rules that consumers need alongside a version label.

## Open questions

- **Supersede shipped v1?** Migrate #4472's unstable writer as assumed, without a deprecation cycle,
  or redraw this RFC around its shipped shape?
- **Processed or raw?** Export processed properties/computed verdicts, agreeing with rendered results:
  reachability checks removed, some descriptions rewritten, successes demoted on fundamental failure.
  Also expose raw results, or leave them to CBMC's `--json-ui`?
- **JSON Schema document?** `schemars` would be a new workspace dependency requiring an explicit decision.
- **Marker or delete-up-front?** The proposal uses a marker; see the completeness trade-offs above.
  Both choices become irreversible once consumed.
- **Autoharness classification?** Export `chosen`/`skipped`, already in compiler-to-driver `AutoHarnessMetadata`?
- **Coverage results?** Include here or leave to `kani-cov`?
- **Dropped/transformed shipped fields?** Decide whether to restore `build_mode`, `mangled_name`,
  `end_line`, `goto_file`, per-check descriptions/locations, CBMC OS information or statistics.
- **Effective `--object-bits`?** Restore shipped resolved encoding provenance under `configuration` or per harness?
  File/update a tracking issue on adoption; resolve representation before or at stabilization.
- **Path provenance?** `file` is invocation-relative. Restore `project.workspace_root` or make paths
  workspace-relative? `output_dir` may stay dropped, but `--target-dir` changes it; do not assume a default.
- **`is_ctor_based`?** Constructor generation/mined-invariant filtering restricts success to admitted
  values independently of `is_bounded`. Representation is open; exports must state this restriction
  before stabilization. Omission cannot justify unrestricted proofs.
- **Final name?** Settle `--export-json` versus `--results-json` before leaving `-Z`.

Apply the configuration policy to every effective Kani option before stabilization, including loop-contract
synthesis and the vtable restriction override; `cbmc_args` does not record these Kani options.

## Out of scope / Future Improvements

None of these additions is proposed now.

- **Full tool/solver provenance and host `machine` metadata:** cut from v1 (#4731); file tracking
  issues on adoption.
- **Per-check timing/resources:** used by GNATprove, unavailable from CBMC structured output today.
- **Reproducible counterexamples:** expose `--concrete-playback`'s concrete value vector without source edits.
- **Finer failure classification:** distinguish unwinding/undefined-function failures to guide retries or fixes.
- **Harness triviality signal:** aggregate RFC 0003's vacuity concern.
- **Aggregate coverage:** defer to `kani-cov` and RFC 0011.
- **Contract trust chain:** `stub_verified` needs a passing contract proof. Kani enforces its existence
  at compile time; both relationships here let consumers check its run-time result. Kani could report status later.
- **Per-harness peak memory:** rejected process-wide `getrusage(RUSAGE_CHILDREN)`; accurate figures
  need per-child accounting (`wait4()` or a cgroup's `memory.peak`).

## Normative schema reference

This RFC contains the proposed field contract and design rules.

### Field reference

`null` means *not measured or not applicable*, never guessed; it differs from `0`, `false` and `[]`.
“—” means never absent/null in a terminal document, subject to explicit per-outcome conditions.
The narrower `INCOMPLETE` marker follows its presence matrix below.

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `schema_version` | string (semver) | — | This document's own version; see Compatibility policy. |
| `kani_commit` | string | nullable | Git commit Kani was built from; `null` outside a git checkout. |
| `kani_commit_dirty` | bool | nullable | Whether the build tree had uncommitted changes; `null` exactly when `kani_commit` is `null`. |
| `tools.kani` | string | — | The Kani release producing this document (`env!("CARGO_PKG_VERSION")` at build time); always known. |
| `tools.rustc` | string | nullable | rustc toolchain `kani-compiler` was built against; `null` if the probe failed. |
| `tools.cbmc` | string | nullable | CBMC's own `--version` output; `null` if it could not be probed. |
| `enabled_unstable_features` | array of strings | never null, may be empty | Sorted `-Z` flags active for this run. |
| `harness_selection.requested_filters` | array of strings | never null, may be empty | Raw `--harness` values; empty means no filter. |
| `harness_selection.exact` | bool | — | Whether `--exact` was passed. |
| `harness_selection.unmatched_filters` | array of strings | never null, may be empty | Filters matching nothing while another matched; only without `--exact`. A wholly unmatched set errors before export (#4743). |
| `harness_selection.matched_count` | integer | — | Pre-verification match count; compare against `summary.total`/`run_state`. |
| `harness_timeout_s` | number | nullable | `--harness-timeout` value in seconds; `null` when unset. |
| `configuration.checks.*` (9 bools) | bool | — each | Nine mandatory bools, defined below under Configuration. |
| `configuration.coverage_enabled` | bool | — | Mandatory; mirrors `--coverage`, default false. Generates excluded `code_coverage` properties; alongside `checks`, not inside it. |
| `configuration.cbmc_args` | array of strings | never null, may be empty | Verbatim (UTF-8-lossy) `--cbmc-args`; a comparability signal, not a replayable argv. |
| `outcome.kind` (run level) | `"COMPLETED"` | — | Always `COMPLETED` in a terminal document; absent in the marker. No run-level `CRASHED` value. |
| `run_state` | `"INCOMPLETE"` \| `"COMPLETE"` \| `"PARTIAL"` \| `"NO_HARNESSES_SELECTED"` | — | Trust is based on this field; see Completeness under Reading the results. |
| `target` | string | — | The Rust target triple Kani itself was built for. |
| `started_at` | string | — | UTC, `YYYY-MM-DDTHH:MM:SSZ` (second resolution). |
| `wall_time_s` | number | — | Seconds; volatile between runs by design (see "Interaction with other flags"). |
| `harnesses[]` | array of harness objects | never null, may be empty (e.g. under `NO_HARNESSES_SELECTED`) | Sorted by `(crate_name, file, line, name)`. |
| `harnesses[].name` | string | — | Fully qualified `pretty_name`; see Selection under Reading the results. |
| `harnesses[].crate_name` | string | — | Distinguishes same-named harnesses across crates in one workspace. |
| `harnesses[].file` | string | — | Declaring-file path relative to the invocation directory; see the path-provenance open question. |
| `harnesses[].line` | integer | — | 1-based harness-function start line; end line is not exported. |
| `harnesses[].contract` | object | nullable | `null` when the harness carries no CBMC-level `assigns` contract. |
| `harnesses[].contract.contracted_function_name` | string | — (when `contract` present) | The contract's target function. |
| `harnesses[].contract.recursion_tracker` | string | nullable | Non-null only for a `#[kani::recursive]` function. |
| `harnesses[].is_automatically_generated` | bool | — | True for an autoharness-generated harness; not selectable with `--harness`/`--exact`. |
| `harnesses[].has_loop_contracts` | bool | — | Whether the harness uses loop contracts. |
| `harnesses[].is_bounded` | bool | — | Mandatory; see Bounded results under Reading the results. |
| `harnesses[].attributes.kind` | `"Proof"` \| `"Test"` \| `{"ProofForContract": {"target_fn": string}}` | — | Requested metadata enum, including the object form for contract proofs. |
| `harnesses[].attributes.should_panic` | bool | — | Whether `#[kani::should_panic]` is set. |
| `harnesses[].attributes.solver` | string \| `{"Binary": string}` | nullable | The *requested* solver attribute; `null` when unset (default resolution applies). Compare `resolved_solver`. |
| `harnesses[].attributes.unwind_value` | integer | nullable | `#[kani::unwind(N)]` value, if set. |
| `harnesses[].attributes.stubs[]` | array of `{original, replacement}` strings | never null, may be empty | Requested stubs. |
| `harnesses[].attributes.verified_stubs[]` | array of strings | never null, may be empty | Functions stubbed by their verified contract. |
| `harnesses[].outcome.kind` | `"COMPLETED"` \| `"TIMEOUT"` \| `"OUT_OF_MEMORY"` \| `"CRASHED"` | — | |
| `harnesses[].outcome.verdict` | `"SUCCESS"` \| `"FAILURE"` | present only on `COMPLETED`, never null there | The per-harness pass/fail call. |
| `harnesses[].outcome.code` | integer | nullable, present only on `CRASHED` | Process exit code when known; per-harness crashes still reach the terminal write. |
| `harnesses[].outcome.message` | string | nullable, present only on `CRASHED` | Free-text reason for this harness crash; wording is non-contractual. |
| `harnesses[].resolved_solver` | string | nullable | The solver CBMC actually runs with; `null` when CBMC chooses for itself (bare `--smt2`). |
| `harnesses[].resolved_unwind` | integer | nullable | The effective `--unwind` bound; `null` when none applies. |
| `harnesses[].generated_concrete_test` | bool | — | Whether `--concrete-playback` produced a test for this harness. |
| `harnesses[].resources.verification_time_s` | number | — | Always present; on timeout/OOM/crash, elapsed time until that outcome, not a verification duration. |
| `harnesses[].n_properties` | integer | nullable; `null` except on `COMPLETED` | `checks.total + covers.total`; excludes `code_coverage` properties. |
| `harnesses[].n_failed` | integer | nullable; `null` except on `COMPLETED` | `failed_properties.len()`. |
| `harnesses[].failure_kind` | `"NONE"` \| `"PANICS_ONLY"` \| `"OTHER"` \| `"ERROR"` | present only on `COMPLETED`, never null there | Raw failure classification, distinct from verdict; see the truth table below. |
| `harnesses[].failed_properties[]` | array of property objects | never null, may be empty (empty on a non-`COMPLETED` harness) | Full shape below. |
| `harnesses[].unsupported_constructs[]` | array of property objects, same shape | never null, may be empty | Rust/MIR constructs Kani can't model; a reached one also appears in `failed_properties`. |
| `harnesses[].warnings[]` | array of `{message: string, truncated: bool, original_chars: integer\|null}` | never null, may be empty | Structural fields are schema-versioned; message contents are not. See warning caps under Rationale. |
| `harnesses[].warnings_truncated` | integer | — | Count of warnings dropped by the cap; zero if none. |
| `harnesses[].checks` / `.covers` | objects | — (the object itself is never omitted) | Bucket shape below; per-outcome contents follow the presence matrix. |
| `summary.*` (7 integer fields) | integer | — each | Run-level totals; see the `Summary` discussion in "Rationale". |

**`failed_properties[]` / `unsupported_constructs[]` element shape** (a full property record):

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `id` | string | — | The CBMC-style property id, in `<function>.<class>.<counter>` general form, or the shorter `<function>.<counter>` / `<class>.<counter>` forms. Recursion unwinding also uses `<function>.recursion` (historically `.recursion`), with **no numeric counter**. The export reconstructs these forms from the parsed id, rather than using Kani's display rendering. Ordinals (`<counter>`) are **not** promised stable across source changes: adding or removing an assertion upstream of one can renumber it. |
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
| `status` | closed `CheckStatus` value | — | The actual, unbucketed status; see Value domains below. |

`other[]` carries only `id` and `status`; full records are reserved for failed/unsupported properties.

**`checks{}` / `covers{}` bucket shape**, both objects:

| Field | Type | Null? | Meaning |
|---|---|---|---|
| `total` | integer | nullable; `null` when the owning harness's `outcome.kind` is not `COMPLETED` | Every property this bucket accounts for. |
| `success` (checks only) | integer | nullable; `null` under the same condition as `total` | A **count**, not an identity list. |
| `satisfied` (covers only) | array of ids | never null, may be empty (empty, not absent, on a non-`COMPLETED` harness) | An identity list (covers are user-authored and few). |
| `failure` (checks) / `unsatisfiable` (covers) | array of ids | never null, may be empty | |
| `unreachable`, `undetermined`, `error`, `unknown` | array of ids, each | never null, may be empty | |
| `other[]` | array of `{id, status}` | never null, may be empty | See above. |

On non-`COMPLETED` harnesses, no property list exists: bucket totals and `checks.success` are null,
every identity list is `[]`, and bucket arithmetic says nothing about the harness.
On `COMPLETED` harnesses, `checks.total == success + len(failure) + len(unreachable) + len(undetermined) + len(error) + len(unknown) + len(other)`;
symmetrically, `covers.total` sums all its lists, including `satisfied`. `n_properties == checks.total + covers.total`.
Partition by `property_id.class`, independently of status: `cover` goes to covers, `code_coverage` is
excluded from both buckets and `n_properties`, and every other class goes to checks. Cross-domain
statuses stay in their class's `other[]`. Successful-check identities are not exported; satisfied covers are named.
`failed_properties[]` is exactly `checks.failure` with full records: non-cover/non-code-coverage
properties with `FAILURE` status, excluding `checks.error` and all covers. Thus `n_failed == len(checks.failure)`.

### Configuration

All nine `configuration.checks.*` fields are mandatory bools; their names record effective check settings.

| Field | Flag/default | Meaning |
|---|---|---|
| `assertion_reach_checks` | `--no-assertion-reach-checks`; true unless passed | Inserts reach checks before ordinary assertions; false prevents vacuity detection through `checks.unreachable`. |
| `ignore_global_asm` | `--ignore-global-asm` | True suppresses the `global_asm!` error; behavior reachable only through it is absent, so success can be vacuous for its effects. |
| `extra_pointer_checks` | `--extra-pointer-checks` | Adds invalid-pointer relational-operation and pointer-arithmetic-overflow obligations; totals are not comparable across settings. |
| `memory_safety` | `--no-memory-safety-checks` (also `--no-default-checks`, the whole group); default true | False leaves out-of-bounds accesses and invalid-pointer dereferences unchecked. |
| `overflow` | `--no-overflow-checks`; default true | False omits CBMC's NaN and division-by-zero checks, not Rust/MIR arithmetic-overflow or integer division-by-zero assertions: `-C overflow-checks=on` remains, subject separately to `prove_safety_only`. |
| `unwinding` | `--no-unwinding-checks`; default true | False omits assertions that loop/recursion bounds cover every execution; behavior beyond bounds can be missed. |
| `undefined_function` | `--no-undefined-function-checks`; default true | False skips `assert-false-assume-false` bodies; calls remain with CBMC's default nondeterministic returns, without modeling possible side effects. |
| `assert_contracts` | `--no-assert-contracts` requires `-Z function-contracts`; true unless passed | False uses `ContractMode::Original` for ordinary calls: original body, no contract assertions or assumptions. Success does not establish contract obligations. Explicit `proof_for_contract`/`stub_verified` modes take precedence and keep their instrumentation. |
| `prove_safety_only` | `--prove-safety-only` requires `-Z unstable-options`; default false | True converts `codegen_assert_assume`'s `PropertyClass::Assertion` checks, including user assertions/panics, to assumptions; success is conditional on them. Direct `codegen_assert` checks, including `CheckHook` Assertion properties, remain; conversion depends on code path, not just class. |

**Policy for `configuration`.** A flag belongs in this block when it changes *which properties are
generated* or *what a status means*: when two runs differing only in it are not directly
comparable, or when it can make a passing result prove less than it appears to. The nine `checks` flags
and `coverage_enabled` meet that test (the partial inventory above identifies remaining gaps); future
flags are added under this rule, each a minor schema change (a new field). `cbmc_args` is the catch-all
for direct CBMC options this rule cannot name individually: anything passed straight to CBMC via
`--cbmc-args` can change results in ways Kani cannot inspect, so it is
recorded and two runs with different `cbmc_args` are not assumed comparable. It is recorded *verbatim
except for UTF-8 conversion* (captured with `to_string_lossy`, so a non-UTF-8 argument is rendered with U+FFFD rather
than round-tripped byte-for-byte): sufficient as a comparability signal, but not an exact argv to replay.
Future flags must be assessed under this policy in the PR adding them.
`--randomize-layout [seed]` changes the program's type layout; its seed is excluded here for a future subject/provenance block.
`resolved_unwind` precedence is CLI `--unwind` > harness `#[kani::unwind]` > `--default-unwind`.
Requested attributes come verbatim from `.kani-metadata.json`; resolved solver/unwind are scalars or null.
Solver precedence is last solver-selecting `--cbmc-args` override > CLI `--solver` > attribute > default;
CBMC arguments follow Kani's flags. Bare `--smt2` lets CBMC choose and yields `resolved_solver: null`.

### Failure scenarios

- Unwritable path: error and non-zero exit; never rewrite the independently computed verification verdict.
  Marker failure precedes verification; terminal export follows harness verdicts but precedes SARIF/final
  summary, so failure aborts those steps. Whether to suppress SARIF remains an implementation question.
- Undetermined tool version: null, never guessed; Kani's own version is always known.
- Harness timeout/OOM/CBMC crash: record that harness outcome and still write the file. Complete accounting
  is verdict-independent, even if every harness times out; missing entries mean `PARTIAL`, marker-only means `INCOMPLETE`.
- Kani crash (compiler ICE, driver panic, SIGKILL): no terminal document; the single final `verify_project`
  write is bypassed. An `INCOMPLETE` marker remains, or whatever file existed if the crash preceded it.
  No run-level `CRASHED` exists; per-harness `CRASHED` remains possible in a completed document.
- Existing file: atomically replace it with the `INCOMPLETE` marker when verification begins after building.
- Missing parent: create it with `create_dir_all`, as for SARIF. An existing directory target currently
  fails at write time; this proposal rejects it at argument parsing before verification.

### Presence matrices

| Field(s) | `INCOMPLETE` marker | Terminal document (`COMPLETE` / `PARTIAL` / `NO_HARNESSES_SELECTED`) |
|---|---|---|
| `schema_version`, `kani_commit`, `kani_commit_dirty`, `tools.*`, `enabled_unstable_features`, `harness_selection.*`, `harness_timeout_s`, `configuration.*`, `target`, `started_at` | present | present |
| `run_state` | present, always `"INCOMPLETE"` | present, one of `"COMPLETE"`/`"PARTIAL"`/`"NO_HARNESSES_SELECTED"` |
| `outcome`, `wall_time_s` | absent | present |
| `summary`, `harnesses[]` | absent | present (`harnesses[]` may be empty, only under `NO_HARNESSES_SELECTED`) |

| Field(s) | `COMPLETED` | `TIMEOUT` / `OUT_OF_MEMORY` | `CRASHED` |
|---|---|---|---|
| `name`, `crate_name`, `file`, `line`, `contract`, `is_automatically_generated`, `has_loop_contracts`, `is_bounded`, `attributes.*`, `resolved_solver`, `resolved_unwind`, `generated_concrete_test` | present | present | present |
| `resources.verification_time_s` | present, a verification duration | present, time elapsed until the timeout/OOM occurred | present, time elapsed until the crash |
| `outcome.verdict` | present | absent | absent |
| `outcome.code` / `outcome.message` | absent | absent | present |
| `failure_kind` | present | absent | absent |
| `checks`, `covers` (the objects themselves) | present | present | present |
| `checks.total`, `checks.success`, `covers.total` | integer | `null` | `null` |
| `checks.failure`, `.unreachable`, …, `covers.satisfied`, … (identity-list fields) | array of ids, may be empty | `[]` | `[]` |
| `n_properties`, `n_failed` | integer | `null` | `null` |
| `failed_properties[]`, `unsupported_constructs[]` | array, may be empty | `[]` | `[]` |
| `warnings[]`, `warnings_truncated` | retained warnings / number dropped by cap | retained warnings / number dropped by cap; `[]` / `0` if none available | retained warnings / number dropped by cap; `[]` / `0` if none available |

Warnings need no property array. Preserve whatever the driver retained, with the same caps for every
outcome; `[]` means none retained, not proof that CBMC emitted none.

### Value domains and casing

| Field | Values | Closed? |
|---|---|---|
| `outcome.kind` (run level) | `COMPLETED` (single value) | closed |
| `run_state` (run level) | `INCOMPLETE`, `COMPLETE`, `PARTIAL`, `NO_HARNESSES_SELECTED` | closed |
| `harnesses[].outcome.kind` | `COMPLETED`, `TIMEOUT`, `OUT_OF_MEMORY`, `CRASHED` | closed |
| `harnesses[].outcome.verdict` | `SUCCESS`, `FAILURE` (field omitted at run level) | closed |
| `harnesses[].failure_kind` | `NONE`, `PANICS_ONLY`, `OTHER`, `ERROR` | closed |
| `…[].status` (in `failed_properties`, `unsupported_constructs`, `checks.other`, `covers.other`) | `SUCCESS`, `FAILURE`, `SATISFIED`, `UNSATISFIABLE`, `UNREACHABLE`, `UNDETERMINED`, `ERROR`, `UNKNOWN`, `COVERED`, `UNCOVERED` | closed |
| `harnesses[].attributes.kind` | `"Proof"`, `"Test"`, `{"ProofForContract": {...}}` | closed (`HarnessKind`, 3 variants) |
| `harnesses[].attributes.solver` | `"Cadical"`, `"Bitwuzla"`, `"Cvc5"`, `"Kissat"`, `"Minisat"`, `"Z3"`, or `{"Binary": "<path>"}` | **explicitly open** — see below |
| `harnesses[].resolved_solver` | the lowercase spellings of the same six names, or an arbitrary binary-path string | **explicitly open** — see below |

Harness `OUT_OF_MEMORY` is inferred from CBMC-child status 137 (including SIGKILL mapped to `128 + 9`)
when no property array exists; it is not measured memory. `TIMEOUT` arises only under `--harness-timeout`.
Read `attributes.should_panic` before interpreting the computed `failure_kind` classification:

| `attributes.should_panic` | `failure_kind` | `outcome.verdict` |
|---|---|---|
| `false` | `NONE` | `SUCCESS` |
| `false` | `PANICS_ONLY`, `OTHER`, or `ERROR` | `FAILURE` |
| `true` | `PANICS_ONLY` | `SUCCESS` |
| `true` | `NONE`, `OTHER`, or `ERROR` | `FAILURE` |

Missing expected panic gives `NONE`; unexpected non-panic failures give `OTHER`; property errors give `ERROR`.
Preserve #4719's computed `ERROR`/`FAILURE` override for ignored quantifiers even without an ERROR-status property; do not recompute solely from properties.
`status` is closed `CheckStatus`. `other` holds known statuses lacking a named bucket in that domain
(e.g. `SATISFIED` among checks), not unknown statuses. An unmodeled status needs a parser variant and major bump.

Solver names are open by this export's consumer contract: accept unfamiliar strings with a string/fallback,
not solely the current closed `CbmcSolver` enum. Its `Binary(String)` variant accepts paths, not unknown
unit variants; producers may reuse its serialization. The known object form remains `{"Binary": "<path>"}`.

**Casing.** *a value keeps the serialization of the Rust type it comes from, and this schema does not fork types to re-case them.*
Keys are snake_case; outcome/verdict/failure_kind/run_state/status values are SCREAMING_SNAKE_CASE,
including new multi-word values; embedded attribute enums retain PascalCase and their object variants.
