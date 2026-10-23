# Writing and running Kani proof harnesses

## Anatomy of a harness

```rust
#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    #[kani::unwind(5)] // only if the code has loops; see "Loops" below
    fn check_parse_never_panics() {
        // 1. Nondeterministic inputs: Kani considers every possible value.
        let len: usize = kani::any();
        kani::assume(len <= 4); // 2. Only the bounds the code really needs.
        let bytes: Vec<u8> = kani::vec::exact_vec::<u8, 4>()[..len].to_vec();

        // 3. Call the code under verification.
        let result = parse(&bytes);

        // 4. Assert the property. Panics, overflow, and UB are checked automatically.
        if let Ok(v) = result {
            assert!(v.len() <= bytes.len());
        }

        // 5. Vacuity probe: must be SATISFIED, or the assumptions exclude everything.
        kani::cover!(true, "harness end reached");
    }
}
```

## Nondeterministic values

| API | Use |
|-----|-----|
| `kani::any::<T>()` | any value of a type implementing `kani::Arbitrary` (primitives, arrays, tuples, `Option`, `Result`, ...) |
| `kani::any_where(\|x: &T\| cond)` | any value satisfying `cond` |
| `#[derive(kani::Arbitrary)]` | derive `Arbitrary` for your structs/enums (under `#[cfg_attr(kani, derive(kani::Arbitrary))]`) |
| `kani::vec::any_vec::<T, N>()` | a `Vec<T>` of any length up to `N` |
| `kani::vec::exact_vec::<T, N>()` | a `Vec<T>` of length exactly `N` |
| `kani::slice::any_slice_of_array(&arr)` | a slice of any length into `arr` |
| `kani::bounded_any::<T, N>()` | types implementing `BoundedArbitrary` (`Vec`, `String`, ...) up to size `N` |

Bounded collections make the proof hold **only up to that size**. Say so in the report.

## Assumptions

`kani::assume(cond)` discards every execution where `cond` is false. Use it to encode the code's
real preconditions, nothing more. Put assumptions before the call under test. An assumption
after the call restricts which results the later assertions see, and can hide bugs.
Run `scripts/check-assumptions.py` after editing a harness.

## Running

```bash
cargo kani                                   # all harnesses in the crate
cargo kani --harness check_parse             # harnesses whose name contains the string
cargo kani --harness mod::check_parse --exact
kani src/lib.rs                              # single file, no Cargo project
cargo kani list --format json                # inventory -> kani-list.json
```

Structured results (preferred over reading console output):

```bash
cargo kani -Z export-json --export-json /tmp/kani.json --harness check_parse
scripts/summarize-results.py /tmp/kani.json --failures-only
```

Console output is long. If you must read it, `--output-format terse` shortens it.

## Counterexamples

```bash
cargo kani --harness check_parse -Z concrete-playback --concrete-playback=print
```

prints a `#[test]` that replays the failure with concrete values. `--concrete-playback=inplace`
adds it to the source file; `cargo kani playback -Z concrete-playback -- <test name>` runs it.
Explain failures using these concrete values, not the raw trace.

## Loops

Kani unrolls loops. Set a bound per harness with `#[kani::unwind(N)]` or globally with
`--default-unwind N`; `N` must exceed the maximum iteration count (iterations + 1). An
"unwinding assertion" failure means the bound is too small. For unbounded loops, consider loop
contracts (`references/contracts.md`) instead of very large bounds.

## Stubbing

Replace functions Kani cannot or should not analyze (FFI, I/O, expensive crypto):

```rust
#[kani::proof]
#[kani::stub(std::fs::read, fake_read)]
fn check_with_stub() { ... }
```

Needs `-Z stubbing`. A stub is an assumption about the real function's behavior: list every stub
in the report.

## Common checks and flags

- Default checks: memory safety, arithmetic overflow, panics (`unwrap`, indexing, `assert!`),
  and several kinds of UB. `--no-overflow-checks` and similar flags turn them off; do not use
  them to make a proof pass.
- `-Z uninit-checks`, `-Z valid-value-checks`: extra UB checks, useful for `unsafe` code.
- `#[kani::should_panic]`: the harness passes only if it panics.
- `#[kani::solver(kissat)]` or `--solver kissat`: try another SAT solver when a proof is slow.
- `-Z unstable-options --harness-timeout 10m`, `--jobs N`: bound time per harness and parallelism.
- Run `cargo kani --help` for the full list; this file covers only the common options.

## Performance

When a proof is slow: shrink input bounds and unwinding first, stub expensive callees, prove
callees separately with contracts and use `stub_verified`, try `--solver kissat`/`cadical`, and
split one large harness into several smaller ones. See the Kani book's "Debugging slow proofs".
