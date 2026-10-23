# Is the proof meaningful? Vacuity and sanity checks

A Kani SUCCESS says "no check failed on any execution the harness allows". If the harness
allows no executions, or never reaches the interesting code, that statement is true and
useless. These failure modes are common, and Kani's verdict alone does not reveal them.

## Checks to run after every successful proof

1. **Vacuity probe.** End the harness with `kani::cover!(true, "harness end reached")`. Kani
   reports covers as SATISFIED (reachable) or UNSATISFIABLE. Unsatisfiable means the
   assumptions exclude every input.
2. **`scripts/check-vacuity.py <export.json>`** reads Kani's JSON export and reports:
   - `unreachable-assertion`: a user assertion that no execution reaches;
   - `all-checks-unreachable`: nothing in the harness was checked;
   - `unsatisfiable-cover` / `unreachable-cover`: a cover the assumptions exclude;
   - `no-vacuity-probe` (info): the harness has no cover; add one.
3. **`scripts/check-assumptions.py <paths> --harness <name>`** scans the source for
   contradictory bounds, `assume(false)`, pinned values (`x == 3`), assumptions that a call
   succeeds (`is_ok()`/`is_some()`), and assumptions placed after assertions.
4. **`scripts/inspect-harness.py <paths> --harness <name>`** flags harnesses with no
   `kani::any()` (a unit test, not a proof), no assertions (only default checks), or loops
   without an unwind bound.

Both checkers exit with status 2 when they report a `warning` or `error`.

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
