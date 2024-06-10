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

 * `--concrete-playback=[print|inplace]`: _Experimental_, `--enable-unstable` feature that generates a Rust unit test case
 that plays back a failing proof harness using a concrete counterexample.
 If used with `print`, Kani will only print the unit test to stdout.
 If used with `inplace`, Kani will automatically add the unit test to the user's source code, next to the proof harness. For more detailed instructions, see the [debugging verification failures](./debugging-verification-failures.md) section.

 * `--visualize`: _Experimental_, `--enable-unstable` feature that generates an HTML report providing traces (i.e., counterexamples) for each failure found by Kani.

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

Starting with Rust 1.80 (or nightly-2024-05-05), every reachable #[cfg] will be automatically checked that they match the expected config names and values.
To avoid warnings on `cfg(kani)`, we recommend adding the `check-cfg` lint config in your crate's `Cargo.toml` as follows:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(kani)'] }
```

For more information please consult this [blog post](https://blog.rust-lang.org/2024/05/06/check-cfg.html).

## The build process

When Kani builds your code, it does three important things:

1. It sets `cfg(kani)` for target crate compilation (including dependencies).
2. It injects the `kani` crate.
3. It sets `cfg(kani_host)` for host build targets such as any build script and procedural macro crates.

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

This conditional compilation with `cfg(kani)` (as seen above) is still required for Kani proofs placed under `tests/`.
When this code is built by `cargo test`, the `kani` crate is not available, and so it would otherwise cause build failures.
(Whereas the use of `dev-dependencies` under `tests/` does not need to be gated with `cfg(test)` since that code is already only built when testing.)
