# Getting started

Kani is an open-source verification tool that uses automated reasoning to analyze your Rust programs.
Kani is particularly useful for verifying unsafe code in Rust, where many of the Rust’s usual guarantees are no longer checked by the compiler.
Some example properties you can prove with Kani include memory safety properties (e.g., null pointer dereferences, use-after-free, etc.), the absence of certain runtime errors (i.e., panics), and the absence of some types of unexpected behavior (e.g., arithmetic overflows).
Kani can also prove custom properties provided in the form of user-specified assertions.

Kani uses proof harnesses to analyze your program. Proof harnesses are similar to test harnesses, especially property-based test harnesses.

## Project Status

Kani is currently under active development and has not made an official release yet.
There is support for a fair amount of Rust language features, but not all (e.g., concurrency).
Please see [Limitations - Rust feature support](./rust-feature-support.md) for a detailed list of supported features.

Kani usually synchronizes with the nightly release of Rust every two weeks, and so is generally up-to-date with the latest Rust language features.

If you encounter issues when using Kani we encourage you to report them to us (https://github.com/model-checking/kani/issues/new/choose).
