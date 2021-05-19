# Rust Model Checker (RMC)
The Rust Model Checker (RMC) aims to be a bit-precise model-checker for Rust.

## Project Status
RMC is currently in the initial development phase.
It **does not yet support all rust language features**.
We are working to extend our support of language features.
If you encounter issues when using RMC we encourage you to 
[report them to us](https://github.com/model-checking/rmc/issues/new/choose).

## Quickstart

1. Install Rust using [rustup](https://www.rust-lang.org/tools/install).

1. Install all dependencies required for upstream-rust, as per the 
   [README](UPSTREAM-README.md#building-on-a-unix-like-system). You do not need to do the rest of the build instructions.

1. Install CBMC.
   CBMC has prebuilt releases 
   [available for major platforms](https://github.com/diffblue/cbmc/releases).
   RMC currently works with CBMC versions 5.26 or greater.
   If you want to build CBMC from source, follow 
   [the cmake instructions from the CBMC repo](https://github.com/diffblue/cbmc/blob/develop/COMPILING.md#working-with-cmake).
   We recommend using `ninja` as the CBMC build system.

1. Configure RMC. 
   We recommend using the following options:
   ```
   ./configure \
      --debuginfo-level-rustc=2 \
      --enable-debug \
      --set=llvm.download-ci-llvm=true \
      --set=rust.debug-assertions-std=false \
      --set=rust.deny-warnings=false \
      --set=rust.incremental=true
   ```

1. Build RMC
   ```
   ./x.py build -i --stage 1 library/std
   ```

1. Run the RMC test-suite
   ```
   ./scripts/rmc-regression.sh
   ```

## Running RMC
RMC currently supports command-line invocation on single files.
We are actively working to integrate RMC into `cargo`.
Until then, the easiest way to use RMC is as follows


1. Add `rmc/scripts` to your path
1. Go to a folder that contains a rust file you would like to verify with RMC.
   For example, `cd rmc/rust-tests/cbmc-reg/Parenths`.
   By default, `rmc` uses `main()` as the entry point.
1. Execute RMC on the file
   ```
   rmc main.rs
   ```
   You should see output that looks like the following
   ```
      ** Results:
   main.rs function main
   [main.assertion.1] line 7 attempt to compute `move _6 + const 1_i32`, which would overflow: SUCCESS
   [main.assertion.2] line 7 attempt to compute `move _4 * move _5`, which would overflow: SUCCESS
   [main.assertion.3] line 8 assertion failed: c == 88: SUCCESS
   [main.assertion.4] line 11 attempt to compute `move _16 * move _17`, which would overflow: SUCCESS
   [main.assertion.5] line 11 attempt to compute `move _15 + const 1_i32`, which would overflow: SUCCESS
   [main.assertion.6] line 11 attempt to compute `move _14 * move _20`, which would overflow: SUCCESS
   [main.assertion.7] line 12 assertion failed: e == 10 * (500 + 5): SUCCESS
   ```
1. Write your own test file, add your own assertions, and try it out!

### Advanced flags
RMC supports a set of advanced flags that give you control over the verification process.
For example, consider the `CopyIntrinsics` regression test:
1. `cd rmc/rust-tests/cbmc-reg/CopyIntrinsics`
1. Execute RMC on the file
   `rmc main.rs`
1. Note that this will unwind forever
   ```
   Unwinding loop memcmp.0 iteration 1 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 2 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 3 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 4 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 5 file <builtin-library-memcmp> line 25 function memcmp thread 0
   ...
   ```
1. You can pass additional arguments to the CBMC backend using the syntax:
   ```
   rmc filename.rs -- <additional CBMC arguments>
   ```
   To see which arguments CBMC supports, run `cbmc --help`.
   In this case, we want the `--unwind` argument to limit the unwinding.
   We also use the `--unwinding-assertions` argument to ensure that our unwind bounds are sufficient.
   Note that:
   ```
   rmc main.rs -- --unwind 1 --unwinding-assertions
   ```
   produces an unwinding failure, while
   ```
   rmc main.rs -- --unwind 17 --unwinding-assertions
   ```
   leads to all assertions passing.
1. You can check for undefined behaviour using builtin checks from CBMC.
   Try using `--pointer-check`, or `--unsigned-overflow-check`.
   You can see the full list of available checks by running `cbmc --help`.

### Looking under the hood
1. To see "under the hood" of what RMC is doing, try passing the `--gen-c` flag to RMC
   ```
   rmc --gen-c main.rs <other-args>
   ```
   This generates a file `main.c` which contains a "C" like formatting of the CBMC IR.
1. You can also view the raw CBMC internal representation using the `--keep-temps` option.

## Security
See [SECURITY](https://github.com/model-checking/rmc/security/policy) for more information.

## Developer guide
See [DEVELOPER-GUIDE.md](DEVELOPER-GUIDE.md).

## License
### Rust compiler
RMC contains code from the Rust compiler.
The rust compiler is primarily distributed under the terms of both the MIT license 
   and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[UPSTREAM-COPYRIGHT](UPSTREAM-COPYRIGHT) for details.

### RMC additions
RMC is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
