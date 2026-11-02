# AutoHarness

`autoharness` generates a harness for each eligible function (all arguments implement
`kani::Arbitrary`, or have a contract) and verifies them, with no hand-written code. It is the
fastest way to sweep a crate for panics, overflow, and UB. It is unstable.

Use the `kani_autoharness` MCP tool when it is available: it bounds the run with `timeout_s`,
returns the harnesses that finished if the run is cut off, splits the results into automatic and
manual harnesses, and validates patterns. Otherwise use the CLI:

```bash
cargo kani autoharness -Z autoharness                       # whole crate
kani autoharness -Z autoharness src/lib.rs                  # single file
cargo kani autoharness -Z autoharness --include-pattern 'parser::' --exclude-pattern 'tests::'
cargo kani autoharness -Z autoharness --list                # only list what would be generated
```

## Before a sweep: list first

Run `--list` (MCP: `kani_autoharness` with `list_only`) before any sweep. It prints which
functions get a harness and why the others are skipped, in seconds, and sizes the sweep. Typical
skip reasons: generic functions with no candidate type, parameters like `&str` or `Formatter`
(fixable with `--bounded-arguments`), and types without `Arbitrary` (fixable with a derive, or
`--constructor-args`). Note that `--list` writes `kani-list.json` into the working directory:
delete it if it was not there before.

## Read the results

Add `-Z export-json --export-json /tmp/auto.json` and read the result with
`scripts/summarize-results.py /tmp/auto.json --failures-only`; a sweep produces many harnesses, and
the raw output is very long. Automatic harnesses are marked `automatically_generated`.

## Things that surprise people

- **Manual harnesses run too.** `--include-pattern` and `--exclude-pattern` only filter which
  *automatic* harnesses are generated. Every manual harness in the crate (`#[kani::proof]`) still
  compiles and runs, first, whatever the patterns say; in one test, 28 manual harnesses used up the
  whole time budget before any automatic harness ran. Budget for them, or sweep from a copy of the
  crate without them. (`--harness` can restrict the run but crashes dev builds of Kani,
  kani#4939; release builds work.)
- **A default unwind bound of 20 for the whole run** (kani#5103). It is meant for the generated
  harnesses, but it also applies to manual harnesses that have no `#[kani::unwind]`: one that
  passes under plain `cargo kani` can fail here ("unwinding assertion") for a reason unrelated to
  the code. Re-run such a failure under plain `cargo kani` before treating it as a finding.
- **Crates that use `#[kani::stub]` need `-Z stubbing`** on the autoharness command too. When it
  is missing, Kani's error does not mention it.
- **Patterns cannot contain spaces.** Write `.` instead: `'Mul<Foo>.for.Bar'`.
- **No concrete playback.** `--concrete-playback` is not supported by autoharness, so a finding
  has no witness. Write a manual harness that reproduces it (`references/kani.md`), then use
  `kani_get_counterexample` and `scripts/check-witness.py` on that.
- **A killed run writes no export.** If the run hits its time limit or is killed (for example
  by the memory watchdog), Kani writes no `--export-json`. Use `timeout_s` with the MCP tool, which
  reads the finished harnesses from the console output; on the CLI, keep the log (`| tee`) and
  use `--harness-timeout` so single harnesses cannot eat the budget.
- **Some crates do not compile with autoharness.** Kani can crash while compiling crates that
  use `HashMap`/`HashSet` (kani#5101). That is a Kani bug, not a problem with the crate: fall back to
  manual harnesses.

## Notes

- `-Z autoharness` implies `-Z function-contracts` and `-Z loop-contracts`. Functions with a
  contract are verified against it.
- `--bounded-arguments` also covers functions taking slices/`&str`/`Vec`; results then hold
  only up to `--slice-bound`, `--string-bound`, and `--bounded-arbitrary-bound`. Such harnesses
  are marked "bounded" in the summary; report the bounds.
- `--constructor-args` builds values of types without `Arbitrary` through their constructors;
  results only cover values reachable that way.
- An autoharness checks that the function does not panic or trigger UB on *any* input. A
  failure may be a precondition the function documents but does not check, typically in a
  private helper whose callers always pass valid arguments; in that case write a contract
  (`references/contracts.md`) or a manual harness with the right assumptions. A failure that
  needs a documented precondition to be false is not a bug in the code, but check that the
  precondition really holds at every call site.
- Failures of the form "Only a single top-level call to function ..." come from contract
  instrumentation, not from the code.
- A sweep shows what the code does for any input; coverage shows which code the sweep
  exercised (`references/coverage.md`).
- Run `kani autoharness --help` for all options.
