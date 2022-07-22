# Using Kani

At present, Kani can used in two ways:

 * [On a single crate](#usage-on-a-single-crate) with the `kani` command.
 * [On a Cargo package](#usage-on-a-package) with the `cargo kani` command.

If you plan to integrate Kani in your projects, the recommended approach is to use `cargo kani`.
If you're already using cargo, this will handle dependencies automatically, and it can be configured (if needed) in `Cargo.toml`.
But `kani` is useful for small examples/tests.

## Usage on a package

Kani is integrated with `cargo` and can be invoked from a package as follows:

```bash
cargo kani [OPTIONS]
```

This works like `cargo test` except that it will analyze all proof harnesses instead of running all test harnesses.

## Common command line flags

Common to both `kani` and `cargo kani` are many command-line flags:

 * `--visualize`: Generates an HTML report showing coverage information and providing traces (i.e., counterexamples) for each failure found by Kani.

 * `--tests`: Build in "[test mode](https://doc.rust-lang.org/rustc/tests/index.html)", i.e. with `cfg(test)` set and `dev-dependencies` available (when using `cargo kani`).

 * `--harness <name>`: By default, Kani checks all proof harnesses it finds.
   You can switch to checking a single harness using this flag.

 * `--default-unwind <n>`: Set a default global upper [loop unwinding](./tutorial-loop-unwinding.md) bound for proof harnesses.
   This can force termination when CBMC tries to unwind loops indefinitely.

Run `cargo kani --help` to see a complete list of arguments.

## Usage on a single crate

For small examples or initial learning, it's very common to run Kani on just one source file.
The command line format for invoking Kani directly is the following:

```
kani filename.rs [OPTIONS]
```

This will build `filename.rs` and run all proof harnesses found within.

## Configuration in `Cargo.toml`

Users can add a default configuration to the `Cargo.toml` file for running harnesses in a package.
Kani will extract any arguments from these sections:

 * `[workspace.metadata.kani.flags]`
 * `[package.metadata.kani.flags]`

For example, if you want to set a default loop unwinding bound (when it's not otherwise specified), you can achieve this by adding the following lines to the package's `Cargo.toml`:

```toml
[package.metadata.kani.flags]
default-unwind = 1
```

The options here are the same as on the command line (`cargo kani --help`), and flags (that is, command line arguments that don't take a value) are enabled by setting them to `true`.

## The build process

When Kani builds your code, it does two important things:

1. It sets `cfg(kani)`.
2. It injects the `kani` crate.

A proof harness (which you can [learn more about in the tutorial](./kani-tutorial.md)), is a function annotated with `#[kani::proof]` much like a test is annotated with `#[test]`.
But you may experience a similar problem using Kani as you would with `dev-dependencies`: if you try writing `#[kani::proof]` directly in your code, `cargo build` will fail because it doesn't know what the `kani` crate is.

This is why we recommend the same conventions as are used when writing tests in Rust: wrap your proof harnesses in `cfg(kani)` conditional compilation:

```rust
#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    pub fn check_something() {
        // ....
    }
}
```

This will ensure that a normal build of your code will be completely unaffected by anything Kani-related.

This conditional compilation with `cfg(kani)` is still required for code under `tests/`.
(Unlike normal test code, which can unconditionally make use of `dev-depenencies` under `tests/`.)
When this code is built by `cargo test`, the `kani` crate is not available, and so it would otherwise cause build failures.
