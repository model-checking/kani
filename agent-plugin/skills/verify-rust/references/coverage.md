# Coverage: what do the harnesses actually exercise?

A SUCCESS verdict covers only the code the harness reaches. A branch no input can take, a
function the harness never calls, an error path hidden behind an assumption: all are checked on
no execution, and nothing in the verdict says so. Source coverage shows these gaps, and it is
the quickest way to find dead code and proof gaps in a set of existing harnesses.

## Run it

```bash
cargo kani -Z source-coverage --coverage [--harness NAME]... | tee kani.log
```

(Add `-Z function-contracts` for crates that use contracts. With the MCP tools, pass
`["-Z", "source-coverage", "--coverage"]` in `extra_args` and set `log_path`.) Kani prints the
covered regions per harness and writes one `*_kaniraw.json` file per harness into
`target/kani/<target>/kanicov_<timestamp>/`.

Run all the harnesses that matter for the code you are checking in one go (or in a few runs:
pass each run's directory below): a region is covered if *any* harness covers it.

## Read it

```bash
scripts/summarize-coverage.py --log kani.log --file src/ethernet.rs
```

It merges the harnesses and prints, per file, the covered and total regions, and each uncovered
region with its line and function. Dependencies (files outside the workspace) are left out unless
you pass `--all-files`; `--src` (default `./src`) lists the source files no harness touched at
all (`files_without_regions`), which the percentages cannot show. A region is covered if any
harness covers it, and generic instances of the same source span are merged, so a closure covered
by one harness is not reported as a gap because another harness's instance never reaches it. `--max-uncovered N` limits the list; `--crate PATH` uses the
latest run under a crate; `--compare DIR...` shows what changed between two sets of runs (for
example, before and after adding a harness). Do not use `kani-cov summary` on projects with
several files: it can panic (kani#5102).

## Interpret it

For each uncovered region, decide which of these it is, and say so in the report:

- **Dead code.** No input reaches it: for example, a branch that needs a value the type cannot
  have, or a fallback after a check that cannot fail. Explain why (the arithmetic, the invariant).
  This is a finding about the code (remove it, or assert it unreachable), not a gap in the proof.
- **A gap in the proof.** Inputs reach it, but the harness does not: an assumption excludes them,
  a bound is too small, or no harness calls the code. Add a harness or widen the inputs, then
  re-run and confirm the region is now covered.
- **A dead branch in the harness itself.** For instance an error path (`unwrap_err`, a short
  input case) that the harness's inputs can never trigger, as when a harness documented as
  "up to N bytes" always uses exactly N. Cover every branch with `kani::cover!`
  (`references/vacuity.md`) and fix the inputs.
- **Not meant to be covered**, such as code only needed outside Kani. Name it.

The percentages are over the regions Kani instrumented, which are the functions the harnesses
reach: a crate with one harness can show 100% while most of its functions were never visited.
Quote the scope (`files_without_regions`, the number of functions reached) next to any percentage.

Coverage is of regions in source, not of behaviors: a fully covered function can still be
checked under assumptions that exclude important inputs. It complements `check-vacuity.py` and
`check-specs.py`; it does not replace them.
