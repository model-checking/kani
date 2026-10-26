---
name: verify-rust
description: Formally verify Rust code with the Kani model checker. Use when the user asks to prove a property of Rust code, check unsafe code for undefined behavior, show a function never panics or overflows, write or fix a Kani proof harness or function contract, run Kani or cargo kani, explain a Kani counterexample, or generate harnesses automatically with Kani's autoharness.
---

# Verify Rust code with Kani

Kani is a bit-precise model checker for Rust. A **proof harness** (`#[kani::proof]`) runs the
code under verification on *all* values of its nondeterministic inputs (`kani::any()`), within
loop-unwinding bounds. If Kani reports SUCCESS, every assertion, panic, overflow, and
undefined-behavior check holds for every input the harness allows.

A green result is only as trustworthy as the harness and contracts it checks. The easiest way
to make a proof pass is to weaken it: add an assumption, drop an assertion, stub out code,
disable a check, or change the code under verification. This skill treats every such change as
something the user must see and approve, and checks proofs with tools rather than by reading.

Helper scripts live in `scripts/` at the plugin root (two directories above this file). They
need only Python 3 and print compact JSON. Detailed guidance lives in `references/`; read the
relevant file when a step below points to it.

## Rules

1. **The agreed property is pinned.** Once the user approves a property (step 1), its harness
   assertions, covers, and allowed assumptions change only with the user's approval. This also
   holds for function contracts the user asked for.
2. **Add proofs, do not change the code under verification.** If a proof fails because of a
   bug, report it with a witness (step 7). Propose a fix separately; apply it only when the user
   approves, and confirm it with `scripts/check-witness.py`.
3. **No result without evidence.** Every claim in the report cites a Kani run recorded with
   `scripts/evidence.py record`, and states everything the result relies on.
4. **Prefer the Kani MCP tools** (`kani_get_version`, `kani_list_harnesses`,
   `kani_verify_harness`, `kani_get_counterexample`) when they cover the operation; otherwise run
   the Kani CLI (`cargo kani ...`, or `kani file.rs`). A missing MCP tool never means Kani lacks
   the feature. If the MCP server is unavailable, use the CLI and the scripts for everything.
5. **Keep context small.** Run Kani with `-Z export-json --export-json <file>` and read results
   with `scripts/summarize-results.py <file>`, not raw console output. For counterexamples, use
   concrete playback. Read raw output only when there is no export (usually a compile error).
6. **Never change the project toolchain** (`rust-toolchain.toml`, profiles, global installs)
   without asking. Kani ships its own toolchain. In a project with a `rust-toolchain.toml`,
   rustup installs the pinned toolchain as soon as `cargo` runs, even `cargo kani` or
   `cargo fetch`. The MCP tools and scripts prevent this themselves. For your own CLI runs,
   prefix every `cargo` command with the `cli_env` that `kani_get_version` /
   `check-kani-version.py` print (`RUSTUP_TOOLCHAIN=<Kani's toolchain> RUSTUP_AUTO_INSTALL=0`),
   and ask before running anything that needs the pinned toolchain.
7. **Bound every run.** Model checking can run for hours. Give each Kani run a time budget
   agreed with the user (`timeout_s` and `harness_timeout_s` on the MCP tools; `timeout` and
   `-Z unstable-options --harness-timeout` on the CLI), verify one harness or a small group
   at a time, and report a run that hits its budget as inconclusive.

## Workflow

### 1. Pin down the property

Restate what the user wants proven as a concrete claim: which function(s), under which
preconditions, and what must hold (no panic, no UB, a functional relation, an invariant).
Propose the input bounds and assumptions you expect to need, and get the user's approval for the
property and its assumptions before writing code. Record the baseline for the code under
verification: the current git commit, or, with uncommitted work, a copy of the sources
(`cp -R src "$TMPDIR/kani-base/src"`).

### 2. Check Kani is available

Call `kani_get_version`, or run `scripts/check-kani-version.py`. If Kani is missing or
unsupported, offer the official install path
(`cargo install --locked kani-verifier && cargo kani setup`); install nothing without consent.

### 3. Understand the code and pick a strategy

Read the code and its types. List existing harnesses with `kani_list_harnesses` (a summary by
default; `filter` for names) or `cargo kani list --format json`, and describe them with
`scripts/inspect-harness.py <src> --summary` (then `--flagged-only` or `--harness`; pass
`--list-json kani-list.json` to find macro-generated harnesses the scan cannot see).
Map the call graph of the code under verification with rust-analyzer's call hierarchy
(`references/rust-analyzer.md`), then choose a strategy with `references/strategy.md`: one
harness for a small target; contracts proved bottom-up for larger ones. Use AutoHarness
(`references/autoharness.md`, CLI only) to sweep many functions for panics and UB.

### 4. Write or update the harness

Follow `references/kani.md`: nondeterministic inputs with `kani::any()`, only the approved
assumptions, a call to the code, then assertions of the property. End the harness with
`kani::cover!(true, "harness end reached")`, and put a `kani::cover!` in every branch of the
harness (each `if`/`match` arm, `unwrap_err`, error path): a branch the inputs never reach is
checked on no execution, and only an UNSATISFIABLE cover shows it. Keep harnesses under
`#[cfg(kani)]`.

Show the harness (and any contract) to the user. **Once they approve it, snapshot a new
baseline** (a commit, or `cp -R src "$TMPDIR/kani-approved/src"`): this pins the property.
From then on, `check-proof-diff.py` against this snapshot flags any weakening; against the
starting commit, a harness written later only shows up as new.

### 5. Run Kani

- MCP: `kani_verify_harness` with `path`, `harness`, and a `timeout_s`. Set `export_path` and
  `log_path` to keep the export and console log for `evidence.py` and `check-vacuity.py`.
  `jobs` verifies harnesses in parallel (each job can take several GB of memory).
- CLI: `cargo kani --harness <name> -Z export-json --export-json /tmp/kani.json | tee
  /tmp/kani.log`, then `scripts/summarize-results.py /tmp/kani.json`. `--jobs N` requires
  `--output-format terse`, which drops the per-check descriptions `check-vacuity.py --log` uses.

Crates that use function contracts need `-Z function-contracts` on every Kani command
(including `list`; pass it in `extra_args` to the MCP tools). If there is no export, fix the
compile error and re-run. If the summary lists
`weakened_checks` (disabled checks or raw CBMC arguments) or a harness lists `trusted` stubs,
the result depends on them: avoid them, or get the user's approval and report them.

### 6. On failure: get a witness

Use `kani_get_counterexample` (its `failure_witnesses` replay failed checks; its
`cover_witnesses` only show inputs that reach a cover), or the CLI with
`-Z concrete-playback --concrete-playback=print`. Explain the failure with those values.
Playback runs the real code, not stubs: if the harness uses `#[kani::stub]` or
`#[kani::stub_verified]`, the failure may come from a stub or a callee contract whose `ensures`
is too weak, and the witness may pass on the real code (`check-witness.py` reports
`spurious-failure-likely`). Then decide with the user whether
the bug is in the code or in the harness (a missing, approved precondition). Never add an
assumption to make a failure disappear without the user's approval.

### 7. Report a defect, or propose a fix

To report a defect, add the witness test with `--concrete-playback=inplace` and run
`scripts/check-witness.py <crate> --test <name>`: only `defect-confirmed` confirms it (the
replay panics with the failure Kani reported; `different-failure` means another panic). Pass
crate options with `--cargo-arg` (e.g. `--cargo-arg --features --cargo-arg std`). If the test
build fails under `cfg(kani)`, the script retries with `--lib`, then in a copy without the
crate's other tests. In a workspace member, `--concrete-playback=inplace` may fail: run it from
the workspace root with `-p <package>`. To fix
it, propose the change to the user first. Once it is approved and applied,
`scripts/check-witness.py <crate> --test <name> --base <baseline>` must report
`fix-confirmed` (the witness fails before the fix and passes after it). Then re-run the proofs.

### 8. On success: check that the proof means something

Run all of these and fix every `error` before reporting:

- `scripts/check-vacuity.py <export.json> --log <kani.log>`: unreachable user assertions and
  unsatisfiable covers. Without `--log` it cannot tell user assertions from built-in overflow
  checks the solver folded away, so it reports them as warnings.
- `scripts/check-specs.py <src> --probe-harnesses --harness <name>`: Kani itself checks that
  the end of each harness is reachable, that contract preconditions (on free functions and
  methods) are satisfiable, and which `requires` clauses constrain nothing. It re-runs the
  harnesses, so give it the same time budget; skip it for harnesses that did not finish.
- `scripts/check-proof-diff.py <src> --base-dir <approved snapshot>` (or `--base <commit>`):
  flags every change since the approved harness that weakens the property (removed assertions
  or covers, new assumptions, including in helper functions the harness calls, stubs,
  `should_panic`, lower unwind bounds, changed contracts), and every change to executable code
  since the starting commit. Each `warning` needs the user's explicit approval; each `error`
  must be reverted unless the user approves it. When a precondition changed, run
  `scripts/check-specs.py <src> --compare-base <baseline>` to classify it with Kani.
- `scripts/check-assumptions.py <src> --harness <name>`: a quick text scan for suspicious
  assumptions; treat its findings as hints to check, not verdicts.

See `references/vacuity.md`.

### 9. Record the evidence and report what was proven

Record the final run with
`scripts/evidence.py record --export <export.json> --command "<the exact command>"` and cite the
bundle it prints. For an MCP run, use the `export_path` you passed and the `run.command` and
`run.cwd` the tool returned (`--cwd`). Then state precisely:

- the harness(es), the verdict, and the evidence bundle;
- the property checked (user assertions, plus Kani's default checks: memory safety, arithmetic
  overflow, panics, UB);
- everything the result relies on: assumptions and input bounds, unwind bounds, `--bounded-arguments`
  sizes, stubs and `stub_verified` contracts, and any `weakened_checks`;
- what is not covered (e.g. "for vectors up to length 4", "with `foo` stubbed");
- every change since the baseline that the user approved.

Never describe a bounded or stubbed proof as unconditional.

## Compatibility

This plugin release supports the Kani versions in `compatibility.json` at the plugin root.
`--export-json`, concrete playback, AutoHarness, and function contracts are unstable Kani
features; they need `-Z export-json`, `-Z concrete-playback`, `-Z autoharness`, and
`-Z function-contracts`.
