# Miri as a complement to Kani

[Miri](https://github.com/rust-lang/miri) interprets Rust code and detects undefined behavior on
the concrete inputs it runs. Kani checks all inputs within bounds but has its own modeling
limits. Use them together:

| Question | Tool |
|----------|------|
| Does this `unsafe` code have UB for *any* input (within bounds)? | Kani |
| Do the existing unit tests trigger UB (aliasing, provenance, data races)? | Miri |
| Does a Kani counterexample really reproduce at runtime? | Concrete playback test, then `cargo miri test <test>` |
| Stacked/Tree Borrows aliasing violations | Miri (Kani does not check Rust's aliasing model) |

Running Miri:

```bash
rustup +nightly component add miri    # ask the user first; this changes their toolchain setup
cargo +nightly miri test
cargo +nightly miri test <test name>
```

Rules:

- Do not change `rust-toolchain.toml` to install Miri. Use `+nightly` explicitly, and ask before
  adding components.
- Miri finding no UB in tests is not a proof. Kani SUCCESS is not a guarantee about aliasing
  rules. Report which tool established which claim.
- A concrete playback test from Kani can be run under Miri to confirm a UB counterexample.
