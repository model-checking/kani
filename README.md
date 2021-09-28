# Rust Model Checker (RMC)
The Rust Model Checker (RMC) aims to be a bit-precise model-checker for Rust.

## Project Status
RMC is currently in the initial development phase.
It **does not yet support all Rust language features**.
We are working to extend our support of language features.
If you encounter issues when using RMC we encourage you to 
[report them to us](https://github.com/model-checking/rmc/issues/new/choose).

## Quickstart

1. Install the dependencies needed for [`rustc`](https://github.com/rust-lang/rust),
   [CBMC](https://github.com/diffblue/cbmc) and
   [CBMC Viewer](https://github.com/awslabs/aws-viewer-for-cbmc/releases/latest).

   The [RMC Installation Guide](https://model-checking.github.io/rmc/install-guide.html)
   shows how to quickly install them using our setup scripts.

1. Configure RMC. 
   We recommend using the following options:
   ```
   ./configure \
      --enable-debug \
      --set=llvm.download-ci-llvm=true \
      --set=rust.debug-assertions-std=false \
      --set=rust.deny-warnings=false
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
We are actively working to integrate RMC into `cargo` (see [experimental Cargo integration](#experimental-cargo-integration)).
Until then, the easiest way to use RMC is as follows


1. Add `rmc/scripts` to your path
1. Go to a folder that contains a rust file you would like to verify with RMC.
   For example, `cd rmc/src/test/rmc/Parenths`.
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

### Using your Config.toml to configure RMC

When invoking RMC using `cargo rmc`, you can use your Config.toml to configure the flags for RMC.
If you want to configure your project to use the following command:

`cargo rmc --c-lib src/lib/harness.c src/lib/api.c src/lib/utils.c --function test_all_rmc --no-memory-safety-checks --verbose --target-dir dev/target --visualize`

Then you could put the following into your Cargo.toml:

```
...
[rmc.flags]
c-lib = [
   "src/lib/harness.c",
   "src/lib/api.c"
   "src/lib/utils.c"
]
function = "test_all_rmc"
memory-safety-checks = false
verbose = true
target-dir = "dev/target"
visualize = true
...
```
and invoke RMC with `cargo rmc /path/to/project`.

You can additionally specify a different toml file to use with the `--config-toml` or disable this feature with `--no-config-toml`.

Lastly, you can override specific flags from command line, e.g. with `cargo rmc /path/to/project --function test_different_rmc`.

### Advanced flags
RMC supports a set of advanced flags that give you control over the verification process.
For example, consider the `CopyIntrinsics` regression test:
1. `cd rmc/src/test/rmc/CopyIntrinsics`
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
   rmc filename.rs --cbmc-args <additional CBMC arguments>
   ```
   To see which arguments CBMC supports, run `cbmc --help`.
   In this case, we want the `--unwind` argument to limit the unwinding.
   We also use the `--unwinding-assertions` argument to ensure that our unwind bounds are sufficient.
   Note that:
   ```
   rmc main.rs --cbmc-args --unwind 1 --unwinding-assertions
   ```
   produces an unwinding failure, while
   ```
   rmc main.rs --cbmc-args --unwind 17 --unwinding-assertions
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
1. The `--gen-c` flag does not produce runnable C code due to differences in the Rust and C languages.
   To produce a runnable C program, try passing the `--gen-c-runnable` flag to RMC
   ```
   rmc --gen-c-runnable main.rs <other-args>
   ```
   This generates a file `main_runnable.c`. 
   Note that this makes some compromises to produce runnable C code, so you should not expect exact semantic equivalence.
1. You can also view the raw CBMC internal representation using the `--keep-temps` option.

### Experimental Cargo integration

We are actively working to improve RMC's integration with Rust's Cargo package and build system. Currently, you can build projects with Cargo via a multi-step process.

For example, we will describe using RMC as a backend to build the [`rand-core` crate](https://crates.io/crates/rand_core). These instructions have been tested on Ubuntu Linux with the `x86_64-unknown-linux-gnu` target.

1. Build RMC
   ```
   ./x.py build -i --stage 1 library/std
   ```

2. Clone `rand` and navigate to the `rand-core` directory:
   ```
   git clone git@github.com:rust-random/rand.git
   cd rand/rand-core
   ```
3. Next, we need to add an entry-point for CBMC to the crate's source. For now, we will just pick an existing unit test. Open `src/le.rs` and find the `test_read` function at the bottom of the file. Add the following attribute to keep the function name unmangled, so we can later pass it to CBMC. 

   ```rust
   // #[test]      <- Remove/comment out this
   #[no_mangle] // <- Add this
   fn test_read() {
      let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

      let mut buf = [0u32; 4];
      read_u32_into(&bytes, &mut buf);
      assert_eq!(buf[0], 0x04030201);
      assert_eq!(buf[3], 0x100F0E0D);
      // ...
   }
   ```

4. Now, we can run Cargo and specify that RMC should be the backend. We also pass a location for the build artifacts (`rand-core-demo`) and a target (`x86_64-unknown-linux-gnu`). 
   ```
    CARGO_TARGET_DIR=rand-core-demo RUST_BACKTRACE=1 RUSTFLAGS="-Z codegen-backend=gotoc --cfg=rmc" RUSTC=rmc-rustc cargo build --target x86_64-unknown-linux-gnu
   ```

5. Now, navigate to the output directory for the given target. 
   ```
    cd rand-core-demo/x86_64-unknown-linux-gnu/debug/deps/
   ```
   
6. The output of Cargo with RMC is a series of JSON files that define CMBC symbols. We now can use CBMC commands to convert and link these files:
   ```
   symtab2gb rand_core-*.json --out a.out              // Convert from JSON to Gotoc 
   goto-cc --function test_read a.out -o a.out         // Add the entry point we previously selected
   goto-instrument --drop-unused-functions a.out a.out // Remove unused functions
   cbmc a.out                                          // Run CBMC
   ```

You should then see verification succeed:
```
** 0 of 43 failed (1 iterations)
VERIFICATION SUCCESSFUL
```

To sanity-check that verification is working, try changing ` assert_eq!(buf[0], 0x04030201);` to a different value and rerun these commands.

For crates with multiple JSON files in the `deps` folder, we suggest running the first command in this step with [`parallel`](https://www.gnu.org/software/parallel/):
   ```
   ls *.json | parallel -j 16 symtab2gb {} --out {.}.out // Convert from JSON to Gotoc 
   ```

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
