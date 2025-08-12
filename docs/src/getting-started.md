# Getting started

Kani is an open-source verification tool that uses [model checking](./tool-comparison.md) to analyze Rust programs.
Kani is useful for checking both safety and correctness of Rust code.
- *Safety*: Kani automatically checks for many kinds of [undefined behavior](./undefined-behaviour.md).
This makes it particularly useful for verifying unsafe code blocks in Rust, where the "[unsafe superpowers](https://doc.rust-lang.org/stable/book/ch19-01-unsafe-rust.html#unsafe-superpowers)" are unchecked by the compiler.
- *Correctness*: Kani automatically checks for certain behaviors that are likely incorrect (namely, panics and arithmetic overflows), although these checks can be disabled if desired. Kani also supports custom correctness properties, either in the form of assertions (`assert!(...)`) or [function contracts](./reference/experimental/contracts.md).

Since Kani uses model checking, Kani will either prove the property, disprove the property (with a counterexample), or may run out of resources.

Kani uses proof harnesses to analyze programs.
Proof harnesses are similar to test harnesses, especially property-based test harnesses.

## Project Status

Kani is currently under active development.
Releases are published [here](https://github.com/model-checking/kani/releases).
Major changes to Kani are documented in the [RFC Book](https://model-checking.github.io/kani/rfc).
We also publish updates on Kani use cases and features on our [blog](https://model-checking.github.io/kani-verifier-blog/).

There is support for a fair amount of Rust language features, but not all (e.g., concurrency).
Please see [Limitations](./limitations.md) for a detailed list of supported features.

Kani releases every month.
As part of every release, Kani will synchronize with a recent nightly release of Rust, and so is generally up-to-date with the latest Rust language features.

If you encounter issues when using Kani, we encourage you to [report them to us](https://github.com/model-checking/kani/issues/new/choose).
