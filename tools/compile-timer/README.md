# Compile-Timer
This is a simple script for timing the Kani compiler's end-to-end performance on crates.

You can run it by first compiling Kani (with `cargo build-dev --release` in the project root), then building this script (with `cargo build --release` in this `compile-timer` directory). This will build a new `compile-timer` binary in `kani/target/release`. After doing that, you should make sure you have Kani on your $PATH (see instructions [here](https://model-checking.github.io/kani/build-from-source.html#adding-kani-to-your-path)) and then you can just run `compile-timer --out-path [PATH]` in any directory to profile the compiler's performance on it. 

By default, the script recursively goes into directories and will use `cargo kani` to profile any Rust projects it encounters (which it determines by looking for a `Cargo.toml`). You can tell it to ignore specific subtrees by passing in the `--ignore [DIR_NAME]` flag.