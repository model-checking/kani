# Is the proof meaningful? Vacuity and sanity checks

A Kani SUCCESS says "no check failed on any execution the harness allows". If the harness
allows no executions, or never reaches the interesting code, that statement is true and
useless. These failure modes are common, and Kani's verdict alone does not reveal them.

## Checks to run after every successful proof

1. **Vacuity probe.** End the harness with `kani::cover!(true, "harness end reached")`. Kani
   reports covers as SATISFIED (reachable) or UNSATISFIABLE. Unsatisfiable means the
   assumptions exclude every input.
2. **`scripts/check-vacuity.py <export.json> --log <kani.log>`** reads Kani's JSON export (and
   the console output of the same run, to tell user assertions from built-in checks) and reports:
   - `unreachable-assertion`: a user assertion that no execution reaches (an `error` with
     `--log`; a `warning` without it, since Kani's built-in overflow and division checks share
     the `assertion` class and are often unreachable because the solver proved they cannot fail);
   - `unreachable-builtin-check` (info, with `--log`): a built-in check that cannot fail;
   - `all-checks-unreachable`: nothing in the harness was checked;
   - `unsatisfiable-cover` / `unreachable-cover`: a cover the assumptions exclude;
   - `no-vacuity-probe` (info): the harness has no cover; add one.
3. **`scripts/check-specs.py <paths> --probe-harnesses --harness <name>`** asks Kani itself,
   in a copy of the crate, whether the end of each harness is reachable
   (`harness-end-unreachable` means the proof is vacuous), whether each contract's preconditions
   are satisfiable together (`requires-unsatisfiable`), and which precondition clauses hold for
   every input (`requires-always-true`). A check that cannot run is reported as `unchecked`,
   never as passing.
4. **`scripts/check-proof-diff.py <paths> --base <baseline>`** compares the harnesses,
   contracts, and code with the baseline the user agreed to, and flags every change that makes
   the proof cover less (removed assertions or covers, new assumptions, stubs, `should_panic`,
   lower unwind bounds, changed contracts), and every change to executable code.
5. **`scripts/check-assumptions.py <paths> --harness <name>`** is a quick text scan of the source for
   contradictory bounds, `assume(false)`, pinned values (`x == 3`), assumptions that a call
   succeeds (`is_ok()`/`is_some()`), and assumptions placed after assertions.
6. **`scripts/inspect-harness.py <paths> --harness <name>`** flags harnesses with no
   `kani::any()` (a unit test, not a proof), no assertions (only default checks), or loops
   without an unwind bound.

Each checker exits with status 2 when it reports a `warning` or `error`. The text-based
scans (`check-assumptions.py`, `inspect-harness.py`) give hints; the Kani-backed checks decide.

## What the result relies on

`scripts/summarize-results.py` lists, for the run, `weakened_checks`: options that disable checks
(`--no-overflow-checks`, `--no-unwinding-checks`, `--prove-safety-only`, ...) or pass raw CBMC
arguments; and, per harness, `trusted`: stubs and `stub_verified` contracts. A SUCCESS holds only
under them. Avoid them, or get the user's approval and list them in the report.

## Interpreting results

- **Reachable but weak**: the probe is SATISFIED, but the assumptions still exclude important
  inputs (for example, `len <= 2` when real inputs have length 1000). This is not vacuous, but
  the report must state the bound.
- **Covers as targeted probes**: `kani::cover!(result.is_err())` shows the error path is
  reachable; `kani::cover!(x > 1000)` shows large values are exercised. Add them for every
  branch the property depends on.
- **Fixing vacuity**: remove or relax the over-constraining assumption, fix the input
  construction (e.g. `exact_vec` with a length that the code rejects early), or raise the
  unwinding bound if a loop exit is unreachable. Never "fix" it by deleting the cover.
- **Mutation sanity check (optional)**: temporarily break the code (flip a comparison) and
  confirm the harness fails. If it still passes, the harness does not check the property.
  Revert the change afterwards.
