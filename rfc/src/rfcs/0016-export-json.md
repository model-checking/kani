- **Feature Name:** Structured Verification Results (`export-json`)
- **Feature Request Issue:** [#942](https://github.com/model-checking/kani/issues/942)
- **RFC PR:** *(to be filled)*
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** Implemented; the schema example below is real output.

-------------------

## Summary

Add an opt-in, `-Z`-gated `--export-json <path>` flag that writes one machine-readable file
describing a verification run: per-harness outcome, failed properties, check and cover outcomes by
status, resource cost, and the provenance needed to reproduce the run.

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
details, not an interface. When they change, a consumer's grep silently matches nothing — the
failure mode is silence, not an error.

Who this helps:

- **CI gates and dashboards** — decide pass/fail, and detect regressions, without parsing prose.
- **Standard-library and large-scale verification efforts**, where hundreds of harnesses run and the
  question is not "did the build pass" but "what is proven, and did any proof stop meaning anything".
- **Commercial and industrial pipelines**, where a verifier must report cost and outcome to systems
  that were not written by the person running it. Machine-readable results are the precondition for
  using a verifier in an automated pipeline at all.
- **Kani's own tooling**, which can stop scraping its own output.

### The specific gap: a proof can pass while proving nothing

A contradictory `kani::assume` makes every subsequent assertion unreachable. The harness reports
`VERIFICATION:- SUCCESSFUL` and exits 0 — including for an assertion as obviously false as
`assert!(x != x)`. Kani's *text* output says so (`** 0 of 2 failed (2 unreachable)`), but a program
checking the exit code — which is what automated consumers do — cannot tell that proof from a real
one.

Kani already computes what is needed to say this. Reachability checks are generated per assertion and
on by default; `Property.reach` is populated; and `update_properties_with_reach_status` already
demotes a `Success` to `Unreachable` when the result cannot be trusted. **That reasoning currently
lives in the presentation layer and reaches no machine-readable output.** This RFC's core proposal is
to carry it in the results file.

**Downside.** A schema is an interface, and interfaces constrain future change. That is why this is
proposed behind `-Z export-json` with an explicit version field, so the shape can be corrected while
it is still being learned from real consumers.

### Relationship to prior work

[PR #4472](https://github.com/model-checking/kani/pull/4472) proposed a similar capability and did
substantial work; the design discussion there shaped this proposal, and I would welcome
@yimingyinqwqq's review. Reviewers on that PR asked for an RFC first, for real unstable gating, for
CBMC data to come from `--json-ui` rather than scraped log text, and for a standard schema approach.
This RFC exists to settle those questions before code merges. It takes RFC number `0016` rather than
`0015` so that #4472 keeps the number it proposed.

## User Experience

```
cargo kani -Z export-json --export-json results.json
```

The flag is additive for every output format it supports: existing rendered output is unchanged, and
the file is written in addition to it. Omitting the flag changes nothing. One combination is rejected
outright rather than silently misreporting: `--output-format=old` bypasses CBMC's structured JSON
output entirely (`run_terminal_timeout` mocks a success/failure result with zero properties either
way, and treats a timeout as success), so `--export-json` under that format would produce a
well-formed file indistinguishable from a real clean run — including for a run that actually timed
out. Kani's argument parser rejects `--export-json` combined with `--output-format=old` with an
explicit error before verification starts.

### Example

The output below is genuine: a live run of the proof-of-concept at commit
`7b125f1b47e36ca4cc50c4041abeca01912f80f9` against exactly the vacuity case this RFC's motivation
section describes -- two ordinary assertions made unreachable by one contradictory `kani::assume`, so
Kani's own text output reads `** 0 of 2 failed (2 unreachable)`, verbatim, from this same run.

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
  "kani_version": "0.67.0",
  "kani_commit": "7b125f1b47e36ca4cc50c4041abeca01912f80f9",
  "kani_commit_dirty": false,
  "cbmc_version": "6.10.0 (cbmc-6.10.0)",
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
    "cbmc_args": []
  },
  "outcome": { "kind": "COMPLETED" },
  "run_complete": true,
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

This is the vacuity case made machine-readable: `outcome.verdict` is `SUCCESS` and Kani's exit code is
`0`, exactly like a harness that proved something — but `checks.unreachable` names both properties
that could not actually be exercised, so a consumer no longer has to trust the exit code alone.
`checks` (and `covers`, symmetrically) buckets every property this schema accounts for exhaustively by
CBMC status — `success`, `failure`, `unreachable`, `undetermined`, `error`, `unknown`, and a catch-all
`other` for any status this schema does not yet know how to classify — so a consumer never has to
guess where an unrecognized result went. Scope: this partition, and `n_properties`, cover exactly what
`checks` and `covers` bucket -- under `--coverage`, `code_coverage` properties (COVERED/UNCOVERED) are
outside both buckets and outside `n_properties` too, since this schema does not export them in a
dedicated place today; `n_properties == checks.total + covers.total` always holds as a result.
`checks.success` is a bare count rather than an identity list, unlike every other bucket here:
successful-check identities are high-volume (checks are auto-generated, sometimes in the thousands)
and low-value (nothing further to investigate), whereas `covers.satisfied` above names its properties
because covers are user-authored and few, so naming which ones passed costs nothing and confirms
intent. Successful-check *identities* are deliberately not exported today — a consumer that needs to
know *what* was proven, not merely that everything passed, needs the full property list, which is
future-work territory this proposal does not promise.

**`warnings`** is empty in this run because no CBMC warning fired. A run that does trigger one — same
commit, a harness using `kani::forall!` over a symbolic range, which the SAT backend cannot discharge
— produces:

```json
{
  "warnings": [
    {
      "message": "warning: ignoring forall\n  * type: bool\n  0: tuple\n      * type: \n      0: symbol\n          * type: unsignedbv\n              * #source_location: \n                * file: <built-in- ... [truncated; 11,669 chars total, verbatim CBMC internal-IR pretty-print]"
    }
  ]
}
```

See the `warnings` discussion below for what this field does and does not promise.

**`configuration.checks.assertion_reach_checks`.** Records whether Kani inserted reachability checks
ahead of ordinary assertions (`true` unless `--no-assertion-reach-checks` was passed). This exists
because that flag changes what a vacuous harness reports, without changing anything else visible in
`configuration.checks`: with reach-checks off, an assertion made unreachable by a contradictory
`kani::assume` lands in `checks.success` instead of `checks.unreachable` — silently defeating the
vacuity signal this schema exists to carry. `configuration` records soundness-relevant toggles like
this one so two runs' results are not misread as comparable when they are not.

**`configuration.checks.ignore_global_asm` and `configuration.checks.extra_pointer_checks`.** Two
more flags recorded for the same reason. `ignore_global_asm` mirrors `--ignore-global-asm`: when
`true`, Kani did not error out on `global_asm!` in the crate, so any behavior reachable only through
that inline assembly is simply absent from the model — a proof against such a crate can pass
vacuously with respect to the assembly's effects, and a consumer needs to know this flag was set to
read the result correctly. `extra_pointer_checks` mirrors `--extra-pointer-checks`: when `true`, Kani
adds obligations for invalid pointers in relational operations and pointer-arithmetic overflow that
are otherwise not checked at all — so two runs that differ only in this flag are checking different
sets of properties, and `checks.total`/`checks.success` are not comparable between them without it.

**Failure scenarios.**

- The output path is not writable → the failure is reported, and the verification result and exit
  code are unaffected. A results-export problem must never change the verdict of the run.
- CBMC's version cannot be determined → that field is `null`. It is never guessed.
- CBMC crashes, times out, or is killed → the file is still written, with the run and harness marked
  as not completed and the reason given.

**The missing-file contract.** The write is atomic: Kani writes the full JSON to a temporary file in
the same directory as the target, then `rename`s it onto the target path (a same-filesystem POSIX
rename is atomic). So a file that *exists* at this path is always a *complete* export from *this*
run's completion path — there is no window in which the target holds a partial write, and a mid-write
kill leaves no target at all (at worst a `.tmp` file beside it, cleaned up on the next successful run
and never mistaken for the target). The target is also deleted up front, before the run starts
producing results at all: that is not needed for "exists implies complete" — the rename already gives
that — but it stops a run that dies *before* ever reaching the write/rename step from leaving an
*earlier* run's file sitting at the target, wrongly read as this run's output. A *missing* file means
this run did not complete its export. That covers more than "verification never began" (a compilation
error, or a rejected `--harness` filter): it also covers an export failure (the bulleted case above:
an unwritable path, a full disk) and abnormal termination (an OOM-kill, Ctrl-C) — none of these leave
a file behind, and absence alone cannot distinguish between them. An export failure is reported like
the other failures above, but never changes the run's verdict.

`null` always means *not measured or not applicable*, never a guess, and is always distinguishable
from `0`, `false`, and `[]`.

## Rationale and alternatives

### Why not extend `--sarif`?

Kani already emits SARIF, and SARIF is a good fit for reporting defects to code-scanning tools. It is
a poor fit for reporting *proofs*, for reasons of shape rather than quality:

- Kani's SARIF writer skips cover properties and emits nothing for successful ones, so **a fully
  green run produces an empty `results` array**. For a prover, the all-green run is the most common
  and most important case.
- SARIF's model is a finding anchored to a location. Proof, cover and vacuity semantics have no
  native home in it; they would live in `properties` bags — a private schema wearing a standard
  schema's clothes — and every code-scanning consumer of that same file would then see results it
  does not want.
- `--sarif` must remain valid SARIF. This artifact must be free to change shape while it is `-Z`
  gated. One file cannot be both.

Both artifacts are worth having, and this proposal does not change `--sarif`.

### Why snake_case, when `kani list --format json` uses kebab-case?

Because this schema **reuses `kani_metadata` and `cbmc_output_parser` types directly** rather than
defining parallel copies of `HarnessAttributes`, `AssignsContract` and `CheckStatus`. Re-casing the
keys would mean either forking those types — recreating the duplication debt tracked in
[#3541](https://github.com/model-checking/kani/issues/3541) — or changing their serialization, which
would break existing `.kani-metadata.json` consumers. The machine-consumed artifacts Kani already
produces are snake_case.

What is worth adopting from `kani list` is the *substance* of its versioning idiom — a tool version
and a schema version as separate fields — and this proposal does that.

### Why a new artifact rather than extending an existing one?

`kani list --format json` is pre-verification metadata; coverage output is per-region data;
`--output-into-files` writes the same rendered text, split per harness; `.kani-metadata.json` is the
compiler-to-driver channel. None carries verification outcomes.

### Why is CBMC statistics data excluded?

Symex time, VCC counts and solver time exist only inside CBMC's free-text messages. Extracting them
would mean pattern-matching human-readable output — which is the fragility this RFC exists to remove,
so doing it inside the fix would be self-defeating. `benchcomp` doing exactly this today is evidence
the need is real; the right fix is structured data from CBMC, not more scraping. A `warnings` field
does carry CBMC's messages verbatim, explicitly as non-contractual free text: CBMC's `--json-ui`
stream tags each message with a `messageType` (for example `"WARNING"`), and this schema surfaces
exactly the `WARNING`-typed messages already present in that stream — for instance the SAT backend's
`warning: ignoring forall`, emitted when a `kani::forall!`/`kani::exists!` over a symbolic bound
cannot be discharged by the default solver. [PR #4719](https://github.com/model-checking/kani/pull/4719),
opened independently by a CBMC maintainer, surfaces this same class of dropped-quantifier warning
prominently — corroboration that this is a real gap, not a hypothetical one. These strings are CBMC's
internal-IR pretty-printer output verbatim, can run to several kilobytes with no promised structure,
and are not parsed or classified by Kani; a consumer must treat each entry as an opaque string to
display or log, not a machine-readable field to pattern-match on — the same caution this RFC applies
to CBMC's rendered text everywhere else.

### What if we do nothing?

Consumers keep scraping, including Kani's own. The status quo works until an output string changes,
and then the breakage is silent — a grep that matches nothing looks exactly like a run with nothing
to report. The vacuity problem in particular stays invisible to automation.

## Open questions

- **Processed or raw view?** The property list Kani renders is already post-processed: reachability
  checks removed, some descriptions rewritten, successful checks demoted on a fundamental failure.
  This proposal exports the **processed** view, so the file agrees with Kani's exit code and text
  output. Should the raw CBMC view also be available, or is that CBMC's `--json-ui` to provide?
- Should a JSON Schema document ship alongside? That likely means a `schemars` dependency, which is
  not currently in the workspace — a real dependency decision rather than something to add quietly.
- Should this cover the `autoharness` subcommand's results, whose `chosen`/`skipped` classification
  currently has no machine-readable form?
- Coverage results — include here, or leave with `kani-cov`?
- **Which other flags belong in `configuration`?** `assertion_reach_checks`, `ignore_global_asm` and
  `extra_pointer_checks` are recorded because each changes what a run's results mean without changing
  anything else visible in the schema — that is now done, not merely proposed. What *other* options
  merit the same treatment, and should this be an explicit policy rather than a field added each time
  one is identified?

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
  `--jobs`, where the counter is shared across concurrently-running siblings. An honest per-harness
  figure needs per-child accounting instead — e.g. `wait4()`-based rusage per child process, or a
  per-child cgroup with its own `memory.peak` — neither of which this schema attempts today; CI
  consumers infer OOM today from exit code 137 (already visible via `outcome.kind == "OUT_OF_MEMORY"`).
