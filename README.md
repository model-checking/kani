# Rust Model Checker (RMC)
The Rust Model Checker (RMC) aims to be a bit-precise model-checker for Rust.

## Project Status
RMC is currently in the initial development phase.
It **does not support all rust language features**.
We are working to extend our support of language features.
If you encounter issues when using RMC then we encourage you to report them on this repository.

## Quickstart

1. Install all dependencies required for upstream-rust, as per the 
   [README](UPSTREAM-README.md#building-on-a-unix-like-system).

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
      --set=llvm.download-ci-llvm=true\
      --set=rust.debug-assertions-std=false \
      --set=rust.deny-warnings=false \
      --set=rust.incremental=true
   ```

1. Build RMC
   ```
   ./x.py build -i --stage 1 library/std --keep-stage 1
   ```

5. Run the RMC test-suite
   ```
   ./scripts/rmc-regression.sh
   ```

## Running RMC
1. Add `rmc/scripts` to your path
2. Go to a folder that contains a rust file you would like to verify with RMC.
   For example, `cd rmc/rust-tests/cbmc-reg/CopyIntrinsics`.
   By default, `rmc` uses `main()` as the entry point.
3. Execute RMC on the file
   `rmc main.rs`
4. Note that this will unwind forever
   ```
   Unwinding loop memcmp.0 iteration 1 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 2 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 3 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 4 file <builtin-library-memcmp> line 25 function memcmp thread 0
   Unwinding loop memcmp.0 iteration 5 file <builtin-library-memcmp> line 25 function memcmp thread 0
   ...
   ```
5. You can pass additional arguments to the CBMC backend using the syntax:
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
6. To see "under the hood" of what RMC is doing, try passing the `--gen-c` flag to RMC
   ```
   rmc --gen-c main.rs -- --unwind 17 --unwinding-assertions
   ```
   This generates a file `main.rs` which contains a "C" like formatting of the CBMC IR.
   You can also view the raw CBMC internal representation using the `--keep-temps` option.
7. Write your own test file, add your own assertions, and try it out!

## Security
See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## Architecture
TODO

## Developer guide
TODO

## Branch structure
RMC is implemented as an additional codegen backend for the 
[Rust compiler](https://github.com/rust-lang/rust).
The `master` branch from the upstream compiler is mirrored as `upstream-master`.
The RMC code itself is maintained as a set of rebased patches, on branches named `main-<upstream-version>-yyyy-mm-dd`.
The most recent of these branches is set as the `default` branch for the repository.
This is the branch that you should build and use, and this is the branch that you should make PRs against.

### Patch structure
The `main-<upstream-version>-yyyy-mm-dd` branches have the following git structure:

* A set of commits representing RMC feature code.
   These patches only affect RMC files.
   Any API changes are contained in a single commit, described below.
* A single patch which renames any upstream files that conflict with RMC files
* A single patch that applies any API changes needed to the upstream code for RMC to link
* The upstream `master` branch as of the date `yyyy-mm-dd`.

### Updating the main branch

The main branch is rebased against upstream on a weekly cadence.
If you have PRs that are open across a `main*` branch rollover, we recommend that you rebase and force-push.

## License
### Rust compiler
RMC contains code from the Rust compiler.
The rust compiler is primarily primarily distributed under the terms of both the MIT license and the Apache License (Version 2.0), with portions covered by various BSD-like licenses.

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT), and
[UPSTREAM-COPYRIGHT](UPSTREAM-COPYRIGHT) for details.

### RMC additions
RMC is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT)for details.
