# RMC on a package

> RMC currently ships with a `cargo-rmc` script, but this support is deeply limited (e.g. to a single crate).
> This will be corrected soon, and this documentation updated.
> In the meantime, we document the current build process for a larger project with dependencies here.

To build a larger project (one with dependencies or multiple crates) with RMC, you currently need to:

1. Build the project with an appropriate set of flags to output CBMC "symbol table" `.json` files.
2. Link these together into a single "goto binary", with appropriate preprocessing flags.
3. Directly call CBMC on this resulting binary.

We give an example of this kind of script, with explanations, below.

# Building and running

Let's assume you have a project you can build with `cargo build` and you've written a proof harness somewhere in it that you want to run RMC on:

```rust
#[no_mangle]
#[cfg(rmc)]
fn my_harness() {
}
```

A sample build script might start like this:

```bash
{{#include sample-rmc-build.sh:cargo}}
```

This allows us to re-use the `cargo` build system, but with flags that override `rustc` with RMC instead.
More specifically, by setting the `RUSTC` environment variable to `rmc-rustc`, each Rust source file targeted by `cargo build` is "compiled" with RMC instead of `rustc`.
The result of running `rmc-rustc` on a source file is a symbol table json file written in the CBMC Goto-C language.
The use of an alternate target directory ensures RMC and rustc don't confuse each other with different intermediate output.

Next we can convert the symbol tables into goto binaries, in parallel, and then link them together:

```bash
{{#include sample-rmc-build.sh:linking}}
```

At this point we have the project built, but now we want to transform it into something that will run a specific proof harness.
To do that, we specialize it, preprocess it, and then run CBMC on the result:
(In practice, we might want to do the above steps once, then repeat the below steps for each proof harness.)

```bash
{{#include sample-rmc-build.sh:cbmc}}
```

At this point we have a complete script and should now be able to run `./sample-rmc-build my_harness` to run a particular proof harness.
Even in very large projects the removal of unreachable code should mean only the parts relevant to that proof harness are preserved in the RMC run.
