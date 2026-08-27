- **Feature Name:** Structured Verification Results (`export-json`)
- **Feature Request Issue:** [#942](https://github.com/model-checking/kani/issues/942)
- **RFC PR:** [#4727](https://github.com/model-checking/kani/pull/4727)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** Implemented; the schema example below is real output.

-------------------

## Summary

Specify the machine-readable file that Kani's `--export-json <path>` flag writes. The flag shipped
in [#4472](https://github.com/model-checking/kani/pull/4472) and is unstable. The file describes one
verification run: per-harness outcome, failed properties, check and cover outcomes by status,
resource cost, and the provenance needed to reproduce the run. The schema in this RFC is the
proposed contract for that file; whether it supersedes the shipped v1 shape, and how, is the first
open question.

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

The output below is genuine: a live run of the proof-of-concept at commit
`7b125f1b47e36ca4cc50c4041abeca01912f80f9` against exactly the vacuity case this RFC's motivation
section describes: two ordinary assertions made unreachable by one contradictory `kani::assume`, so
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
  "kani_commit": "7b125f1b47e36ca4cc50c4041abeca01912f80f9",
  "kani_commit_dirty": false,
  "tools": {
    "kani": "0.67.0",
    "rustc": "1.98.0-nightly (14210df0e 2026-05-31)",
    "cbmc": "6.10.0 (cbmc-6.10.0)",
    "goto_cc": "6.10.0 (cbmc-6.10.0)",
    "goto_instrument": "6.10.0 (cbmc-6.10.0)",
    "solvers": [{ "name": "cadical", "version": null }]
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
      "selector": "check_contradictory_assume",
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

**Tool provenance (`tools`).** A single object carries the version of every tool whose behaviour can
change a result: `kani`, `rustc`, `cbmc`, `goto_cc`, `goto_instrument`, and a `solvers` array of
`{name, version}` (a solver CBMC selects itself contributes `version: null`). This is the machine-readable
form of the versions Kani already prints, closing the gap in
[#2572](https://github.com/model-checking/kani/issues/2572): a consumer comparing two runs, or bisecting a
regression to a toolchain bump, no longer scrapes stdout for them.

**Stable harness selector (`selector`).** Besides its bare `name` and `crate_name`, each harness carries
the exact string `--harness` accepts to run it again — the module-qualified path
(`sequence::proofs::no_over_read`; for a crate-root harness it equals `name`). `name` alone is not
re-runnable (two crates may share one), and `(crate_name, file, line, name)` identifies a harness without
being a *selector*. An out-of-tree consumer that records which harness backs a downstream result — a CI
gate re-running one proof, an external tool tracking a proof across commits — needs one stable,
re-runnable key, and reconstructing it from the file path is fragile. `selector` is that key; Kani owns
the string rather than making a consumer rebuild it.

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

**`warnings`** is empty in this run because no CBMC warning fired. A run that does trigger one (same
commit, a harness using `kani::forall!` over a symbolic range, which the SAT backend cannot discharge)
produces:

```json
{
  "warnings": [
    {
      "message": "warning: ignoring forall\n  * type: bool\n  0: tuple\n      * type: \n      0: symbol\n          * type: unsignedbv\n              * #source_location: \n                * file: <built-in- ... [truncated; 11,669 chars total]"
    }
  ]
}
```

See the `warnings` discussion below for what this field does and does not promise.

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

**Policy for `configuration`.** A flag belongs in this block when it changes *which properties are
generated* or *what a status means*, that is, when two runs differing only in that flag are not
apples-to-apples comparable, or when the flag can make a passing result mean less than it appears to.
The three `checks` flags above meet that test; future flags are added under this rule rather than case
by case, and adding one is a minor schema change (a new field). `cbmc_args` is the explicit catch-all for
everything this rule cannot name individually: anything passed straight to CBMC via `--cbmc-args` can
change results in ways Kani cannot introspect, so it is recorded and two runs with different `cbmc_args`
are not assumed comparable. It is recorded *verbatim modulo UTF-8*: arguments are captured with
`to_string_lossy`, so a non-UTF-8 argument is rendered with the U+FFFD replacement character rather than
round-tripped byte-for-byte; this is sufficient for the comparability signal, but a consumer must not treat the
list as an exact argv to replay.

**Failure scenarios.**

- The output path is not writable → the failure is surfaced as an error and the process exits
  non-zero; it is never silently swallowed. The verification *verdict* is computed independently and is
  never rewritten by an export problem, but the export failure itself is real and reflected in the exit
  status. The up-front marker write fails before verification starts; the terminal export runs after the
  harness verdicts have been computed and the normal per-harness output (when enabled) has been rendered,
  but *before* the SARIF artifact and the final summary line, so a terminal export failure both exits
  non-zero and aborts those remaining reporting steps. (Whether an export failure should suppress the
  SARIF write is an implementation question flagged for the code review, not settled here.)
- CBMC's version cannot be determined → that field is `null`. It is never guessed.
- A harness times out, is OOM-killed, or CBMC crashes on it → that harness's `outcome.kind`
  (`TIMEOUT`/`OUT_OF_MEMORY`/`CRASHED`) records it and the file is still written. Run completeness is
  *verdict-independent*: as long as every selected harness produced a result, the run's `run_state` is
  still `COMPLETE` (a suite where every harness times out is a complete run of failing harnesses, not an
  incomplete run). `run_state` is only `INCOMPLETE`/`PARTIAL` when Kani itself did not reach the terminal
  write for every selected harness; see the completeness contract below.
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

**The completeness contract.** This proposal replaces the boolean `run_complete` shown in the example
above with a richer **`run_state`** field, and anchors trust on it rather than on file existence. (The
example still shows `run_complete` because it is genuine output of the pre-marker proof-of-concept; the
`run_state` marker described here is the proposed change, and choosing between it and
delete-up-front is one of the open questions below.) The write is atomic: Kani writes
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
run aborted after the first failure), or `NO_HARNESSES_SELECTED` (a non-`--exact` filter matched nothing).

Reading `run_state` is therefore not optional for a consumer that means to read the file correctly:

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
anchor) is distinct from the run-level `outcome.kind` (`COMPLETED`/`CRASHED`), which is diagnostic only:
a consumer gates on `run_state`, not on `outcome.kind`. The contract assumes a single writer per path: two
concurrent runs aiming `--export-json` at the same file are unsupported, and the last rename wins.

`null` always means *not measured or not applicable*, never a guess, and is always distinguishable
from `0`, `false`, and `[]`.

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

### Value domains, casing, and which fields are not strings

**Enum domains.** Every field whose value is drawn from a fixed set is *closed*: each vocabulary below
is a Kani-owned Rust enum (the CBMC-derived `CheckStatus` among them, which Kani re-exports). The
`checks.*`/`covers.*` `other` field is a *bucketing* device, not an enum catch-all; see below.

| Field | Values | Closed? |
|---|---|---|
| `outcome.kind` (run level) | `COMPLETED`, `CRASHED` | closed |
| `run_state` (run level) | `INCOMPLETE`, `COMPLETE`, `PARTIAL`, `NO_HARNESSES_SELECTED` | closed |
| `harnesses[].outcome.kind` | `COMPLETED`, `TIMEOUT`, `OUT_OF_MEMORY`, `CRASHED` | closed |
| `harnesses[].outcome.verdict` | `SUCCESS`, `FAILURE` (field omitted at run level) | closed |
| `harnesses[].failure_kind` | `NONE`, `PANICS_ONLY`, `OTHER`, `ERROR` | closed |
| `…[].status` (in `failed_properties`, `unsupported_constructs`, `checks.other`, `covers.other`) | `SUCCESS`, `FAILURE`, `SATISFIED`, `UNSATISFIABLE`, `UNREACHABLE`, `UNDETERMINED`, `ERROR`, `UNKNOWN`, `COVERED`, `UNCOVERED` | closed |

`outcome.kind` is deliberately asymmetric: `TIMEOUT` and `OUT_OF_MEMORY` describe a *harness* CBMC could
not finish and never appear at run level, where the only two values are `COMPLETED` (Kani finished the
run) and `CRASHED` (Kani itself aborted). `OUT_OF_MEMORY` is inferred from a `137` exit (SIGKILL, the
usual OOM-killer signal), not a direct memory measurement; `TIMEOUT` arises only under
`--harness-timeout`. `failure_kind` is present on every completed harness: it is `NONE` exactly when
`outcome.verdict` is `SUCCESS`, and names the class of failure otherwise. The `status` values are the
closed `CheckStatus` set; the `checks.*`/`covers.*` `other` bucket is *not* a home for unknown statuses
but for known `CheckStatus` values that do not map to a named bucket in that domain, for instance a
cover-only status (`SATISFIED`/`UNSATISFIABLE`) surfacing among checks, which keeps the partition
exhaustive without implying the status vocabulary is open. Should CBMC ever emit a status `CheckStatus`
does not model, that is a parser-level concern resolved by adding a variant. That is a *major* schema change
under the compatibility policy below, since `status` is a closed enum a consumer may match exhaustively,
not something the `other` bucket silently absorbs.

**Casing.** The file mixes three conventions, and one rule explains all three: *a value keeps the
serialization of the Rust type it comes from, and this schema does not fork types to re-case them.*

- **snake_case keys**, because the schema reuses `kani_metadata` and `cbmc_output_parser` types directly
  (`HarnessAttributes`, `AssignsContract`, `CheckStatus`) rather than parallel copies. Re-casing keys
  would mean forking those types, recreating the duplication debt tracked in
  [#3541](https://github.com/model-checking/kani/issues/3541), or changing their serialization and
  breaking existing `.kani-metadata.json` consumers.
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

What is worth adopting from `kani list` is the *substance* of its versioning idiom: a tool version and
a schema version as separate fields. This proposal does that; see **Compatibility policy** below.

### Compatibility policy

`schema_version` is a semantic version. It exists so the shape can be corrected while it is still being
learned from real consumers, and it makes the rules of that correction explicit rather than implied:

- **Minor (backward-compatible) changes** bump the minor version: adding a field, or adding an enum
  variant that a consumer following the forward-compatibility rule below (a default/`_` arm) already
  tolerates. Such a consumer keeps working across a minor bump.
- **Major (breaking) changes** bump the major version: renaming or removing a field, changing the meaning
  or type of an existing field, adding a variant to a *closed* enum (`outcome.kind`, `verdict`,
  `failure_kind`, `run_state`, `status`) since a consumer may exhaustively match those, or moving a status
  out of the
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

**Semver-zero, while it is `-Z` gated.** Until stabilization the schema is `0.x`, and the minor/major
*distinction above describes intent* rather than a promise: a `0.x` consumer must match an exact minor
(or an explicitly supported range) instead of relying on forward-compatibility, because a breaking change
may land on a `0.x` minor bump. The backward-compatibility guarantee for minor changes takes effect at
`1.0`.

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
  `[{ "message": … }]` shape are contractual; the *contents* (wording, structure, count) are not, and
  must not be pattern-matched or assumed stable across versions.
- **`warnings` is bounded, with explicit truncation markers.** One CBMC warning already runs to
  ~11.6 KB (the example above), and the standard-library-scale runs this RFC targets can emit many;
  left unbounded the field could dwarf everything else in the file and defeat the CI consumers it exists
  to serve. So each `message` is capped at a fixed, implementation-defined length, truncated on a
  character boundary with the marker `[truncated; N chars total]` appended (as shown above), and the
  per-harness `warnings` array is capped at a fixed count with a sibling `warnings_truncated` integer on
  each harness (`0` when nothing was dropped, shown in the example) recording how many entries were
  omitted. A consumer can then always tell "no more warnings" from "we stopped recording", and never
  infers completeness from absence. The structural fields here, `warnings_truncated` and the presence of
  a marker, are schema-versioned; the warning text they bound is not.

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
  same consumer, which reads a stale `COMPLETE` from a previous run until the new one finishes. Both
  become one-way doors as soon as anything consumes the file, which is why this is asked rather than
  decided.
- Should this cover the `autoharness` subcommand's results, whose `chosen`/`skipped` classification
  currently has no machine-readable form?
- Coverage results: include here, or leave with `kani-cov`?
- **Shipped fields dropped in this shape — keep or drop?** #4472's document carried three fields this
  schema does not: the Cargo provenance `project.workspace_root` / `output_dir`; the autoharness
  `is_bounded` / `is_ctor_based` per-harness flags (orthogonal to the `autoharness` question above, which
  is about `chosen`/`skipped`); and the `coverage.enabled` marker. Each is cheap to restore if a consumer
  wants it, and the default here is to leave them out until one does — but the removal is called out as a
  decision rather than a silent regression, since a consumer of the shipped v1 shape may rely on them.
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
