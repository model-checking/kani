# Profiling Kani's Performance

To profile Kani's performance at a fine-grained level, we use a tool called [`samply`](https://github.com/mstange/samply) that allows the compiler & driver to periodically record the current stack trace, allowing us to construct flamegraphs of where they are spending most of their time.

## Install samply
First, install `samply` using [the instructions](https://github.com/mstange/samply?tab=readme-ov-file#installation) from their repo. The easier methods include installing a prebuilt binary or installing from crates.io.


## Running Kani for profiling output
1. First, build Kani from source with `cargo build-dev --profile profiling` to ensure you are getting all release mode optimizations without stripping useful debug info.
2. Then, you can profile the Kani compiler on a crate of your choice by [exporting Kani to your local PATH](build-from-source.md#adding-kani-to-your-path) and  running `FLAMEGRAPH=[OPTION] cargo kani` within the crate.

The `FLAMEGRAPH` environment variable can be set to `driver` (to profile the complete `kani-driver` execution) or `compiler` (to profile each time the `kani-compiler` is called).

We have to instrument the driver and compiler separately because samply's instrumentation usually cannot handle detecting the subprocess the driver uses to call the compiler.

Our default sampling rate is *8000 Hz*, but you can change it yourself in [`session.rs`](../../kani-driver/src/session.rs) for the compiler or the [cargo-kani](../../scripts/cargo-kani) script for the driver.

> Note: Specifically when profiling the compiler, ensure you are running `cargo clean` immediately before `cargo kani`, or parts of the workspace may not be recompiled by the Kani compiler.


## Displaying profiling output
This will create a new `flamegraphs` directory in the crate which will contain a single `driver.json.gz` output file and one `compiler-{crate_name}.json.gz` file for each crate in the workspace. Run `samply load flamegraphs/XXX.json.gz` on any of these to open a local server that will display the file's flamegraph.

Once the server has opened, you'll see a display with the list of threads in rows at the top, and a flamegraph for the currently selected thread at the bottom. There is typically only one process when profiling the driver. When profiling the compiler, the process that runs the `kani-compiler` and handles all codegen is usually at the very bottom of the thread window.

In the flamegraph view, I've found it very useful to right click on a function of interest and select "focus on subtree only" so that it zooms in and you can more clearly see the callees it uses. This can then be undone with the breadcrumb trail at the top of the flamegraph.