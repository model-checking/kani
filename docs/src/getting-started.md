# Getting started

Kani is a Rust verification tool based on _model checking_. With Kani, you can
ensure that wide classes of problems are absent from your Rust code by writing
_proof harnesses_, which are broadly similar to tests (especially property
tests).

Kani is particularly useful for verifying unsafe code in Rust, where
many of the language's usual guarantees can no longer be checked by the
compiler. But it's also useful for finding panics and check user-defined
assertions in safe Rust.

## Project Status

Kani is currently under active development and has not made an official release yet.
There is support for a fair amount of the Rust language features, but not all of them.
If you encounter issues when using Kani we encourage you to [report them to us](https://github.com/model-checking/kani/issues/new/choose).

Kani usually synchronizes with the main branch of Rust every two weeks, and so
is generally up-to-date with the latest Rust language features.
