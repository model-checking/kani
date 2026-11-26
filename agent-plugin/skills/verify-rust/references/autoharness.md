# AutoHarness

AutoHarness generates harnesses with no hand-written code, and it is the main way to verify
contracts. It is unstable (`-Z autoharness`, which implies `-Z function-contracts` and
`-Z loop-contracts`).

- A function **with a contract** gets a generated `#[kani::proof_for_contract]` harness: Kani
  checks the function against its `requires`/`ensures`/`modifies` on every input the contract
  allows, loops with invariants included. This is how contracts are verified in the contract-first
  method (`references/strategy.md`).
- A function **without a contract** (all arguments `kani::Arbitrary`) gets a plain harness that
  checks panics, overflow, and UB on all inputs, within the unwind bound.

Use the `kani_autoharness` MCP tool when it is available: it bounds the run (`timeout_s`,
`memory_limit_mb`), returns the harnesses that finished if the run is cut off, splits the results
into automatic and manual harnesses, and validates patterns. The CLI:

```bash
cargo kani autoharness -Z autoharness -Z unstable-options --harness-timeout 5m \
    --include-pattern '^my_crate::math::div$' --include-pattern '^my_crate::sum_to$'
cargo kani autoharness -Z autoharness --list      # which functions get a harness, and why not
kani autoharness -Z autoharness src/lib.rs        # a single file
```

The Crate Verifiability Map's plan gives these commands per stage, with one anchored pattern per
function. The map counts the generated harnesses as evidence, per function: a verified
`proof_for_contract` harness makes the function's contract `verified`.

## Before a sweep: list first

`--list` (MCP: `list_only`) prints which functions get a harness and why the others are skipped, in
seconds. Typical skip reasons: generic functions with no candidate type, parameters like `&str` or
`Formatter` (use `--bounded-arguments`), and types without `Arbitrary` (derive it, or
`--constructor-args`). `--list` prints the tables to stderr, exits with status 2 even on success, and
writes `kani-list.json` into the current directory: do not take the exit status as a failure, and
delete the file if it was not there before. Platform or feature gated code only appears if you pass
the crate's features: without them, a harness for code that needs them can fail for that reason.

## Patterns

`--include-pattern` and `--exclude-pattern` are regular expressions matched against the function's
path as `--list` prints it: `crate::module::function`, `crate::Type::method`,
`<crate::Type as crate::Trait>::method`. They cannot contain spaces: write `.` for each
(`'<my_crate::P.as.my_crate::Flags>::set'`). Anchor them (`^...$`) to select one function. There is
no `--include-function`. In a crate whose harness helpers live in `#[cfg(kani)]` modules,
AutoHarness also generates harnesses for those helpers (raw-pointer helpers then fail on arbitrary
pointers): `--exclude-pattern '::proofs::|::verification::'`.

## Results

Add `-Z export-json --export-json <file>` and read it with
`scripts/summarize-results.py <file> --failures-only --log <kani.log>`. Automatic harnesses are
marked `automatically_generated`. A sweep produces many harnesses: split large crates by
`--include-pattern` into chunks that finish inside your time limit, run each in the background, and
keep each export (a killed run writes none).

## Things that surprise people

- **Manual harnesses run too.** The patterns only filter which *automatic* harnesses are generated.
  Every manual harness in the crate (`#[kani::proof]`) still compiles and runs, first; in one test,
  28 manual harnesses used up the whole time budget before any automatic harness ran. Budget for
  them, or sweep from a copy of the crate without them. (`--harness` can restrict the run but
  crashes dev builds of Kani, kani#4939; release builds work.)
- **A default unwind bound of 20 for the whole run** (kani#5103). It is meant for the generated
  harnesses, but it also applies to manual harnesses with no `#[kani::unwind]`: one that passes under
  plain `cargo kani` can fail here ("unwinding assertion") for a reason unrelated to the code. Re-run
  such a failure under plain `cargo kani` before treating it as a finding. For generated harnesses,
  a loop with an invariant is not unrolled, and one without is checked only up to the bound: the map
  says so.
- **Crates that use `#[kani::stub]` need `-Z stubbing`** on the autoharness command too. When it
  is missing, Kani's error does not mention it.
- **No concrete playback.** `--concrete-playback` is not supported by autoharness, so a finding has
  no witness: write a manual harness that reproduces it (`references/kani.md`), then use
  `kani_get_counterexample` and `scripts/check-witness.py` on that.
- **A killed run writes no export.** Use `timeout_s` with the MCP tool (it reads the finished
  harnesses from the console output); on the CLI keep the log (`| tee`) and use `--harness-timeout`
  so a single harness cannot eat the budget.
- **Some crates do not compile with autoharness.** Kani can crash while compiling crates that use
  `HashMap`/`HashSet` (kani#5101): that is a Kani bug; fall back to manual harnesses.
- **Trivial functions add no evidence**: an `unsafe fn new(ptr)` that only stores a pointer passes
  trivially. Prefer to spend the budget on functions with logic, and say what a sweep covered.

## Reading failures

- A failure on a **documented precondition** (a private helper whose callers always pass valid
  arguments) is a missing contract, not necessarily a bug: write the `requires`, show it to the user,
  and re-run. Check that the precondition holds at every call site.
- A failure that says **"Only a single top-level call to function ..."** comes from contract
  instrumentation, not from the code.
- An **unwinding assertion** on a generated harness is a loop with no invariant: add
  `#[kani::loop_invariant]` and `#[kani::loop_decreases]` instead of raising the bound.
- A **TIMEOUT** or **OUT_OF_MEMORY**: shrink inputs (`--string-bound`, `--slice-bound`), add loop
  invariants, replace callees by their verified contracts, and report it as inconclusive.

## Options worth knowing

- `--bounded-arguments` covers functions taking slices, `&str`, `Vec`; results then hold only up to
  `--slice-bound`, `--string-bound`, `--bounded-arbitrary-bound`: report them (the log does not).
- `--constructor-args` builds values of types without `Arbitrary` through their constructors; the
  result only covers values reachable that way.
- `--check-invariants` checks that returned values satisfy their type's mined invariants.
- Run `kani autoharness --help` for all options.
