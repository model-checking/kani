---
name: verify-rust
description: Formally verify Rust code with the Kani model checker. Use when the user asks to prove a property of Rust code, check unsafe code for undefined behavior, show a function never panics or overflows, write or fix a Kani proof harness, function contract, or loop contract, run Kani or cargo kani, explain a Kani counterexample, plan what to verify in a crate, or generate harnesses automatically with Kani's autoharness.
---

# Verify Rust code with Kani

Kani is a bit-precise model checker for Rust. If it reports SUCCESS for a harness, every assertion,
panic, overflow, and undefined-behavior check holds for every input the harness allows.

**The default method is contract-first.** Give each function a contract (`#[kani::requires]`,
`#[kani::ensures]`, `#[kani::modifies]`), each loop an invariant and a decreases clause
(`#[kani::loop_invariant]`, `#[kani::loop_modifies]`, `#[kani::loop_decreases]`), and let
**AutoHarness** verify every contract with no hand-written harness. Callers then rely on a
callee's verified contract (`#[kani::stub_verified]`) instead of re-proving its body. Loop contracts
replace unwinding bounds, so a proof covers all inputs and not only small ones, and a `decreases`
clause shows that the loop terminates (without one, a proof is partial). Hand-written harnesses and
`#[kani::unwind(N)]` are the fallback, for properties a contract cannot express and for loops where
no invariant can be found. The Crate Verifiability Map plans this for a whole crate: which
contracts to write first, which loops lack invariants or decreases, and the AutoHarness command
for each stage.

A green result is only as trustworthy as the contracts and harnesses behind it. The easiest way to
make a proof pass is to weaken it: add a precondition, an assumption, drop an assertion, stub out
code, disable a check, or change the code under verification. This skill treats every such change
as something the user must see and approve (**a `requires` is an assumption**), and checks proofs
with tools rather than by reading.

**Where things are.** The *plugin root* is the directory that contains `skills/`, `scripts/`,
`schemas/`, and `plugin.json`: from this file's directory (`skills/verify-rust/`) it is two levels
up. Scripts are in `<plugin root>/scripts/` and need only Python 3; they print compact JSON.
Guidance is in `references/` next to this file (`references/contracts.md`, ...): read the one a step
points to. The scripts and MCP tools keep their own files (work copies, evidence, exports) in the
plugin output directory, `target/kani-plugin/` of the project (`KANI_PLUGIN_OUT` changes it): use it
instead of `/tmp` for your own exports and logs too.

## Rules

1. **The agreed property is pinned.** Once the user approves a property (step 1), its contracts,
   harness assertions, covers, and allowed assumptions change only with the user's approval. With
   no user to ask (an unattended run), write the property and every precondition and assumption you
   chose into the report as *unapproved*; never call them approved.
2. **Add contracts and proofs, do not change the code under verification.** If a proof fails
   because of a bug, report it with a witness (step 7). Propose a fix separately; apply it only when
   the user approves, and confirm it with `scripts/check-witness.py`.
3. **No result without evidence.** Every claim in the report cites a Kani run recorded with
   `scripts/evidence.py record`, and states everything the result relies on.
4. **Prefer the Kani MCP tools** (`kani_get_version`, `kani_list_harnesses`, `kani_verify_harness`,
   `kani_autoharness`, `kani_get_counterexample`, and for planning `kani_map_crate`,
   `kani_plan_verification`, `kani_update_verification_map`, `kani_explain_function`) when they
   cover the operation; otherwise run the Kani CLI (`cargo kani ...`, or `kani file.rs`). Some clients
   list tools by name only and load their schemas on request: load the `kani_*` tools before calling
   them. A missing MCP tool never means Kani lacks the feature; if the server is unavailable, use the
   CLI and the scripts for everything.
5. **Keep context small.** Run Kani with `-Z export-json --export-json <file>` and read results
   with `scripts/summarize-results.py <file> --log <kani.log>`, not raw console output. For
   counterexamples, use concrete playback. Read raw output only when there is no export (a compile
   error, or a run that was killed).
6. **Never change the project toolchain** (`rust-toolchain.toml`, profiles, global installs)
   without asking. Kani ships its own toolchain. In a project with a `rust-toolchain.toml`,
   rustup installs the pinned toolchain as soon as `cargo` runs, even `cargo kani` or
   `cargo fetch`. The MCP tools and scripts prevent this themselves. For your own CLI runs,
   prefix every `cargo` command with the `cli_env` that `kani_get_version` /
   `check-kani-version.py` print (`RUSTUP_TOOLCHAIN=<Kani's toolchain> RUSTUP_AUTO_INSTALL=0`),
   and ask before running anything that needs the pinned toolchain.
7. **Bound every run.** Model checking can run for hours, and a run that is killed writes no
   results. Agree a time budget with the user; give every run a limit (`timeout_s`,
   `harness_timeout_s`, `memory_limit_mb` on the MCP tools; `timeout` and `-Z unstable-options
   --harness-timeout` on the CLI); run harnesses one per process (step 5); run anything longer than a
   few minutes in the background and poll it (shell tools often have a 2 to 10 minute limit that
   kills the run and its results); and report a run that hits a limit as inconclusive.

## Workflow

### 1. Pin down the property

Restate what the user wants proven as a concrete claim: which function(s), under which
preconditions, and what must hold (no panic, no UB, a functional relation, an invariant). Propose
the contracts you expect to need (preconditions, postconditions, what is modified) and any other
assumptions, and get the user's approval for the property, the contracts, and the assumptions before
writing code. Record the baseline for the code under verification: the current git commit, or, with
uncommitted work, a copy of the sources (`cp -R src target/kani-plugin/base/src`).

### 2. Check Kani is available

Call `kani_get_version`, or run `scripts/check-kani-version.py`. If Kani is missing or
unsupported, offer the official install path
(`cargo install --locked kani-verifier && cargo kani setup`); install nothing without consent.

### 3. Map the code and plan

First decide whether there is anything to verify. Kani checks functions with bodies in a library or
binary target. A crate that only defines `macro_rules!` macros (an empty list and AutoHarness
showing no items), or a proc-macro crate (Kani reports "No supported targets"), has nothing to
verify as is: say so with that evidence instead of writing harnesses for compile-time behavior. A
proc-macro's helper functions can be verified in a scratch copy with the crate type changed to `lib`
and the macro entry points removed; say that the entry points were not.

List existing harnesses with `kani_list_harnesses` (a summary by default; `filter` for names) or
`scripts/list-harnesses.py` (not plain `cargo kani list`: it writes `kani-list.json` into the
current directory, rejects `--target-dir` and `--features`, and hides compile errors), and describe
them with `scripts/inspect-harness.py <src> --summary`.

**Build the Crate Verifiability Map** (`kani_map_crate`, or `scripts/map-crate.py map`;
`references/verifiability-map.md`) for any crate with more than a few functions, and **plan from
it** (`kani_plan_verification`): the call graph, recursive components, which functions have
contracts and loop contracts and which results exist, and a staged bottom-up plan. Each stage
lists, per function, the next step (write a contract, add a loop invariant, add a decreases
clause, give a recursive component contracts and `#[kani::recursion]`), and the AutoHarness
command that verifies the stage's contracts. The map is a planning aid: read its `uncertainty`
(unresolved calls, dynamic dispatch, macros, `cfg`) before trusting an edge. For a single function,
use rust-analyzer's call hierarchy (`references/rust-analyzer.md`). `references/strategy.md` has the
strategy; `references/contracts.md` has the syntax and the pitfalls.

Code that reads the environment, spawns processes, or touches the OS (`std::env::var`,
`Command::output`, backtraces, files) hits constructs Kani does not model: it fails as unsupported or
never terminates. Stub the narrow function that does it (`#[kani::stub]`, with `-Z stubbing`), say so
in the report, and verify the logic around it; do not write harnesses that run the real call.

### 4. Write contracts, loop contracts, and (only where needed) harnesses

Work bottom-up along the plan. For each function: a contract (`references/contracts.md`), kept as
weak as the code needs: every `requires` narrows what is proved. For each loop: an invariant, a
`loop_modifies` clause if it writes through references, and a `#[kani::loop_decreases(...)]`
clause (loop attributes need `#![feature(stmt_expr_attributes, proc_macro_hygiene)]` and
`-Z loop-contracts`); an unwinding bound is the fallback. For recursion: contracts on every member
and `#[kani::recursion]`. Write a hand-written harness only for what a contract cannot express (a
property over several calls, a scenario, a stub-heavy environment); follow `references/kani.md`:
nondeterministic inputs with `kani::any()`, only the approved assumptions, a call, assertions, and a
`kani::cover!(true, "harness end reached")` at the end (except in a `#[kani::should_panic]` harness)
and in every branch: a branch the inputs never reach is checked on no execution, and only an
UNSATISFIABLE cover shows it. Prefer `assert!(a == b)` to `assert_eq!(a, b)`: Kani describes a failed
`assert_eq!` with a placeholder message. Keep harnesses under `#[cfg(kani)]`.

Show the contracts and harnesses to the user. **Once they approve them, snapshot a new baseline**
(a commit, or `cp -R src target/kani-plugin/approved/src`): this pins the property. From then on,
`check-proof-diff.py` against this snapshot flags any weakening; against the starting commit, a
contract or harness written later only shows up as new.

### 5. Run Kani, with AutoHarness first

- **AutoHarness verifies the contracts**: `kani_autoharness` with `include_patterns` (the plan gives
  them per stage; `list_only` first, because it also runs every manual harness, with a different
  default unwind bound), or `cargo kani autoharness -Z autoharness -Z loop-contracts -Z
  unstable-options --harness-timeout 5m --include-pattern '^crate::path$'`. For each function with
  a contract it generates a `proof_for_contract` harness that checks the function against the
  contract on every input the contract allows; for the rest it checks panics and UB. Details and
  pitfalls: `references/autoharness.md`.
- **Hand-written harnesses**: MCP `kani_verify_harness` with `path`, `harness`, `timeout_s`, and
  `memory_limit_mb`. For many harnesses set `per_harness`: each runs in its own process, so a
  timeout or memory kill loses only that harness. The tool reports `run.export_path` (the export,
  under the plugin output directory) and takes `log_path`: keep both for `evidence.py` and
  `check-vacuity.py`. With `exact` the name must be module-qualified (`proofs::check_x`, as the list
  shows it). On the CLI, `scripts/run-harnesses.py` runs one process per harness with a timeout and
  a memory cap and keeps the merged results as they finish (run it in the background); for one
  harness: `cargo kani --harness <name> -Z export-json --export-json target/kani-plugin/kani.json |
  tee target/kani-plugin/kani.log`, then `scripts/summarize-results.py ... --log ...`.
- Flags that trip people up: `--harness-timeout` needs `-Z unstable-options`; `#[kani::stub]` needs
  `-Z stubbing`; contracts need `-Z function-contracts` and loop contracts `-Z loop-contracts` (the
  MCP tools and scripts add the ones they detect in the sources); `--jobs N` needs
  `--output-format terse`, which drops the per-check descriptions `check-vacuity.py --log` uses;
  `cargo kani` has no `--target-dir` (set `CARGO_TARGET_DIR`), no `--test NAME`, and `list` takes
  neither `--features` nor `--target-dir`. In `zsh`, do not build `--harness` lists in a string
  variable. An unfiltered `cargo kani` compiles every harness, and one compiler crash (kani#5101:
  `HashMap`/`HashSet`) then stops all of them: run harnesses by name.

If there is no export, the run was killed or did not compile: check the log (`summarize-results.py
--log` shows the verdicts in it), fix the compile error, or re-run bounded. If the summary lists
`weakened_checks` (disabled checks or raw CBMC arguments) or a harness lists `trusted` stubs, the
result depends on them: avoid them, or get the user's approval and report them. A TIMEOUT or
OUT_OF_MEMORY verdict is not a failure and not a proof: read its `diagnosis` (an unwinding bound that
is too small and unbounded recursion look like out-of-memory), shrink symbolic inputs first
(`references/kani.md`, Performance), replace a callee by its verified contract, add a loop invariant,
and say the result is inconclusive.

### 6. On failure: get a witness

Use `kani_get_counterexample` (its `failure_witnesses` replay failed checks; its `cover_witnesses`
only show inputs that reach a cover), or the CLI with `-Z concrete-playback
--concrete-playback=print`. AutoHarness does not support concrete playback: write a manual harness
that reproduces the failure first. Explain the failure with the concrete values. Playback runs the real
code, not stubs or contracts: if the harness uses `#[kani::stub]` or `#[kani::stub_verified]`, the
failure may come from a stub or a callee contract whose `ensures` is too weak, and the witness may
pass on the real code (`check-witness.py` reports `spurious-failure-likely`). Then decide with the
user whether the bug is in the code or in the contract or harness (a missing, approved
precondition). A failing contract proof is either a bug or a contract that is too strong or too
weak: never add a precondition to make a failure disappear without the user's approval.

### 7. Report a defect, or propose a fix

To report a defect, add the witness test (`--concrete-playback=inplace`, or paste the `print`
output into the harness's module) and run `scripts/check-witness.py <crate> --test <name>`: only
`defect-confirmed` confirms it (the replay panics with the failure Kani reported;
`different-failure` means another panic). Kani's inplace mode can add the same test twice, misplace
tests, or fail in a workspace member: run `scripts/cleanup-playback.py <file>` to drop duplicates, and
run it from the workspace root with `-p <package>` for a member. The script copes with a `no_std`
crate and with test targets that do not build under `cfg(kani)`; for a hand-written test pass
`--expect "<failure text>"`. Pass crate options with `--cargo-arg`. A defect that native execution
does not trap (out-of-bounds pointer arithmetic, invalid pointers) replays as passing: confirm it with
Miri (`references/miri.md`) and report it as Kani-only evidence.

To fix it, propose the change to the user first. Once it is approved and applied,
`scripts/check-witness.py <crate> --test <name> --base <baseline>` must report `fix-confirmed` (the
witness fails before the fix and passes after it); commit the witness and its harness before the fix
so the baseline has them. Then re-run the proofs, and pass the fixed functions to
`check-proof-diff.py --allow-code-change`.

### 8. On success: check that the proof means something

Run all of these and fix every `error` before reporting:

- `scripts/check-vacuity.py <export.json> --log <kani.log>`: unreachable user assertions and
  unsatisfiable covers. Without `--log` it cannot tell user assertions from built-in overflow
  checks the solver folded away, so it reports them as warnings. Info findings are only counted.
- `scripts/check-specs.py <src> --probe-harnesses --harness <name>`: Kani itself checks that
  the end of each harness is reachable, that contract preconditions (on free functions and
  methods) are satisfiable (a contradictory `requires` makes a contract proof vacuous), and which
  `requires` clauses constrain nothing. It re-runs the harnesses (give it `--harness-timeout` and the
  crate's `--features`), so select harnesses and skip those that did not finish.
- `scripts/check-proof-diff.py <src> --base-dir <approved snapshot>` (or `--base <commit>`):
  flags every change since the approved contracts and harnesses that weakens the property (removed
  assertions or covers, new or strengthened preconditions, removed postconditions, new assumptions,
  including in helper functions and stub bodies, stubs, `should_panic`, lower unwind bounds), and
  every change to executable code, including in macro invocations, since the starting commit. Each
  `warning` needs the user's explicit approval; each `error` must be reverted unless the user
  approves it. When a precondition changed, run `scripts/check-specs.py <src> --compare-base
  <baseline>` to classify it with Kani.
- `scripts/summarize-coverage.py --log <kani.log>` after a run with
  `-Z source-coverage --coverage`: which regions of the code no harness exercises. Classify each
  uncovered region (dead code, a gap in the proof, a dead branch in the harness) and report it
  (`references/coverage.md`). Its percentages are over the functions the harnesses reach, not over
  the crate.
- `scripts/check-assumptions.py <src> --harness <name>`: a quick text scan for suspicious
  assumptions; treat its findings as hints to check, not verdicts.

Harness filters take a name, a module-qualified name, or a module, and fail loudly when nothing
matches. See `references/vacuity.md`.

### 9. Record the evidence and report what was proven

If you built a map, `kani_update_verification_map` (or `scripts/map-crate.py update`) it with the
final exports and re-plan: the map counts AutoHarness results as evidence, tracks which contracts
are verified and which loops still lack invariants or decreases clauses, and replans.

Take `scripts/evidence.py snapshot --out target/kani-plugin/pre.json` **before** the run, then record
it afterwards: `scripts/evidence.py record --export <export.json> --log <kani.log> --snapshot
target/kani-plugin/pre.json --command "<the exact command>"`, and cite the bundle it prints.
(Without a snapshot, any file touched during the run, even restored to the same content as in a
mutation test, triggers a warning, and then you must re-run.) For an MCP run, use `run.export_path`,
`run.command`, and `run.cwd` from the tool's result (`--cwd`). Then state precisely:

- the functions and harnesses, the verdicts, and the evidence bundle;
- the property checked (the contracts, user assertions, plus Kani's default checks: memory safety,
  arithmetic overflow, panics, UB);
- everything the result relies on: preconditions (`requires`) and assumptions, input bounds, unwind
  bounds, `--bounded-arguments` sizes, stubs and `stub_verified` contracts (and whether each
  contract is verified), and any `weakened_checks`;
- termination: which loops have `decreases` clauses; a proof with invariants but no decreases clause
  is partial, and a proof of a loop without an invariant holds only within its unwind bound;
- what is not covered (e.g. "for vectors up to length 4", "with `foo` stubbed"), including the
  regions coverage shows no harness exercises;
- every change since the baseline that the user approved.

Never describe a bounded, stubbed, or partial proof as unconditional.

## Compatibility

This plugin release supports the Kani versions in `compatibility.json` at the plugin root.
`--export-json`, concrete playback, AutoHarness, function contracts, and loop contracts are unstable
Kani features; they need `-Z export-json`, `-Z concrete-playback`, `-Z autoharness`,
`-Z function-contracts`, and `-Z loop-contracts`.
