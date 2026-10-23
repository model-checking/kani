---
name: verify-rust
description: Formally verify Rust code with the Kani model checker. Use when the user asks to prove a property of Rust code, check unsafe code for undefined behavior, show a function never panics or overflows, write or fix a Kani proof harness or function contract, run Kani or cargo kani, explain a Kani counterexample, or generate harnesses automatically with Kani's autoharness.
---

# Verify Rust code with Kani

Kani is a bit-precise model checker for Rust. A **proof harness** (`#[kani::proof]`) runs the
code under verification on *all* values of its nondeterministic inputs (`kani::any()`), within
loop-unwinding bounds. If Kani reports SUCCESS, every assertion, panic, overflow, and
undefined-behavior check holds for every input the harness allows. That last clause is why this
skill spends as much effort on *what the harness allows* as on running Kani.

Helper scripts live in `scripts/` at the plugin root (two directories above this file). They
need only Python 3 and print compact JSON. Detailed guidance lives in `references/`; read the
relevant file when a step below points to it.

## Tool policy

1. **Prefer the Kani MCP tools** (`kani_get_version`, `kani_list_harnesses`,
   `kani_verify_harness`, `kani_get_counterexample`) when they cover the operation.
2. **Otherwise run the Kani CLI directly** (`cargo kani ...` in a Cargo project, `kani file.rs`
   for a single file). The MCP surface is intentionally small; a missing MCP tool never means
   Kani lacks the feature. AutoHarness, contracts, stubbing, coverage, solver selection, and
   every other flag are reachable through the CLI. If the MCP server is unavailable, do
   everything through the CLI and the scripts.
3. **Keep context small.** Run Kani with `-Z export-json --export-json <file>` and read the
   result with `scripts/summarize-results.py <file>` instead of the raw console output. For a
   counterexample, use concrete playback (a short unit test with concrete inputs) instead of the
   full trace. Only read raw output when the export does not exist (usually a compile error).
4. **Complementary tools.** Use rust-analyzer (via the editor/LSP) to resolve types, symbols,
   and call sites. Use Miri for fast concrete UB checks of existing tests. Neither replaces Kani;
   see `references/miri.md` and `references/rust-analyzer.md`.
5. **Never change the project toolchain.** Do not edit `rust-toolchain.toml`, `Cargo.toml`
   profiles, or global installs without asking. Kani ships its own toolchain.

## Workflow

### 1. Pin down the property

Restate what the user wants proven as a concrete, checkable claim: which function(s), under
which preconditions, and what must hold (no panic, no UB, a functional relation, an invariant).
If the property is vague ("make sure this is safe"), propose a specific one and confirm. Note
any input bounds you will need (e.g. vector length) because they limit what the proof covers.

### 2. Check Kani is available

Call `kani_get_version`, or run `scripts/check-kani-version.py`. If Kani is missing or
unsupported, tell the user and offer the official install path:
`cargo install --locked kani-verifier && cargo kani setup`. Do not install anything without
consent. Use a stable Kani release unless the user explicitly asks for a development build.

### 3. Understand the code

Read the code under verification and the types it uses. Find existing harnesses with
`kani_list_harnesses` or `cargo kani list --format json` (writes `kani-list.json`), and check
their structure with `scripts/inspect-harness.py <src dir>`. Note `cfg` features, loops (they
need unwinding bounds), heap allocation, `unsafe` blocks, and calls into code Kani cannot model
(FFI, inline assembly, OS calls), which need stubs.

### 4. Choose the approach

- **Write a harness** for a specific property: see `references/kani.md`.
- **AutoHarness** to sweep many functions for panics/UB with no hand-written harness:
  `cargo kani autoharness -Z autoharness` (CLI only); see `references/autoharness.md`.
- **Function contracts** for modular proofs or functions called from many places; see
  `references/contracts.md`.
- **Miri first** when there are existing tests and the question is concrete UB on those inputs.

### 5. Write or update the harness

Follow `references/kani.md`. In short: make inputs nondeterministic with `kani::any()`, constrain
them with `kani::assume` only as much as the code's real preconditions require, call the code,
then assert the property. Add `kani::cover!(true, "harness end reached")` at the end as a
vacuity probe. Put harnesses behind `#[cfg(kani)]` so they do not affect normal builds.

### 6. Run Kani

- MCP: `kani_verify_harness` with `path` (project dir or `.rs` file) and `harness`.
- CLI: `cargo kani --harness <name> -Z export-json --export-json /tmp/kani-results.json`, then
  `scripts/summarize-results.py /tmp/kani-results.json`.

If Kani does not produce results, it is usually a compile error: fix it and re-run. If
verification is slow or does not terminate, see "Performance" in `references/kani.md`.

### 7. On failure: get a small counterexample

Use `kani_get_counterexample`, or add `-Z concrete-playback --concrete-playback=print` to the CLI
run. Each generated unit test replays one failure with concrete input values. Explain the
failure in terms of those values, then decide with the user whether the bug is in the code or
in the harness (a missing precondition). Do not silence a real bug with a new `kani::assume`.

### 8. On success: check the proof means something

Before reporting a proof, always run both:

- `scripts/check-vacuity.py <export.json>`: unreachable assertions and unsatisfiable covers mean
  the harness, or part of it, proves nothing.
- `scripts/check-assumptions.py <files or dirs> --harness <name>`: contradictory, pinned, or
  success-assuming assumptions, and `requires(false)`.

Treat any `error` finding as a blocker: fix the harness and re-run. Review each `warning` and
either fix it or mention it in the report. See `references/vacuity.md`.

### 9. Report what was actually proven

State precisely:

- the harness(es) and the verdict;
- the property checked (user assertions, plus Kani's default checks: memory safety, arithmetic
  overflow, panics, UB);
- every assumption and bound that limits the result (input ranges, `#[kani::unwind]` /
  `--default-unwind`, `--bounded-arguments` sizes, stubs, contracts used via `stub_verified`);
- what was *not* covered (e.g. "for vectors up to length 4", "with `foo` stubbed").

Never describe a bounded or stubbed proof as unconditional.

## Compatibility

This plugin release supports the Kani versions in `compatibility.json` at the plugin root.
`--export-json`, `--concrete-playback`, and `autoharness` are unstable Kani features and need the
`-Z export-json`, `-Z concrete-playback`, and `-Z autoharness` flags shown above.
