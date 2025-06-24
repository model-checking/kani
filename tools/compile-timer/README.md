# Compile-Timer
This is a simple script for timing the Kani compiler's end-to-end performance on crates.

## Setup
You can run it by first compiling Kani (with `cargo build-dev --release` in the project root), then building this script (with `cargo build --release` in this `compile-timer` directory). This will build new `compile-timer` & `compile-analyzer` binaries in `kani/target/release`. 

## Recording Compiler Times with `compile-timer`
After doing that, you should make sure you have Kani on your $PATH (see instructions [here](https://model-checking.github.io/kani/build-from-source.html#adding-kani-to-your-path)) after which you can run `compile-timer --out-path [OUT_JSON_FILE]` in any directory to profile the compiler's performance on it.

By default, the script recursively goes into directories and will use `cargo kani` to profile any Rust projects it encounters (which it determines by looking for a `Cargo.toml`). You can tell it to ignore specific subtrees by passing in the `--ignore [DIR_NAME]` flag.

## Visualizing Compiler Times with `compile-analyzer`
`compile-timer` itself will have some debug output including each individual run's time and aggregates for each crate.

`compile-analyzer` is specifically for comparing performance across multiple commits. 

Once you've run `compile-timer` on both commits, you can run `compile-analyzer --path-pre [FIRST_JSON_FILE] --path-post [SECOND_JSON_FILE]` to see the change in performance going from the first to second commit. 

By default, `compile-analyzer` will just print to the console, but if you specify the `--only-markdown` option, it's output will be formatted for GitHub flavored markdown (as is useful in CI).