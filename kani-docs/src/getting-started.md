# Getting started with Kani

Kani is a Rust verification tool based on _model checking_.
With Kani, you can ensure that broad classes of problems are absent from your Rust code by writing _proof harnesses_, which are broadly similar to tests (especially property tests).
Kani is especially useful for verifying unsafe code in Rust, where many of the language's usual guarantees can no longer be checked by the compiler.
But Kani is also useful for finding panics in safe Rust, and it can check user-defined assertions.

## Project Status

Kani is currently in the initial development phase, and has not yet made an official release.
It is under active development, but it does not yet support all Rust language features.
(The [Book runner](./bookrunner.md) can help you understand our current progress.)
If you encounter issues when using Kani we encourage you to [report them to us](https://github.com/model-checking/kani/issues/new/choose).

Kani usually syncs with the main branch of Rust every week, and so is generally up-to-date with the latest Rust language features.

## Getting started

1. [Begin with the Kani installation guide.](./install-guide.md) Currently, this means checking out and building Kani.
2. [Understand how Kani compares to other potential tools for verifying Rust code.](./tool-comparison.md)
3. [Try following the Kani tutorial to get a feel for how Kani can be applied.](./kani-tutorial.md)
