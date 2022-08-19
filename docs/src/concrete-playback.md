# Debugging verification failures with concrete playback

When the result of a certain check comes back as a `FAILURE`,
Kani offers different options to help debug:
* `--concrete-playback`. This _experimental_ feature generates a Rust unit test case that plays back a failing
proof harness using a concrete counterexample.
* `--visualize`. This feature generates an HTML text-based trace that
enumerates the execution steps leading to the check failure.

This document describes the concrete playback feature in more detail.

## Setup

The Kani library needs to be linked as a dev dependency to the crate you're trying to debug.
To use the latest Kani library version, add the following lines to your `Cargo.toml` file:
```toml
[dev-dependencies]
kani = { git = "https://github.com/model-checking/kani", features = ["concrete_playback"] }
```
Otherwise, if you would like to use a specific version of the Kani library (v0.9+) and have already downloaded its source code files,
add the following lines to your `Cargo.toml` file:
```toml
[dev-dependencies]
kani = { path = "{path_to_kani_root}/library/kani", features = ["concrete_playback"] }
```

## Usage

Run Kani with the `--concrete-playback=JustPrint` flag.
After verifying the proof harness checks, Kani will extract concrete values for the `kani::any()` variables
and generate a Rust unit test case.
This unit test initializes the concrete values and calls the proof harness.
You can then either 1) copy this unit test into the same module as their proof harness or
2) run Kani with the `--concrete-playback=InPlace` flag to have Kani automatically do this.
Before adding the unit test, you might find it helpful to have your existing code committed to `git`.
That way, you can easily remove the unit test with `git revert`.

After the unit test is in your source code, you can run it with `cargo test`.
To debug it, there are a couple of options.
If you have certain IDEs, there are extensions (e.g., `rust-analyzer` for `VS Code`)
that support UI elements like a `Run Test | Debug` button next to all unit tests.
Otherwise, you can debug the unit test on the command line.
To do this, you first run `cargo test {unit_test_func_name}`.
The output from this will have a line in the beginning like `Running unittests {files} ({binary})`.
You can then debug the binary with tools like `rust-gdb` or `lldb`.

## Common issues

`error[E0425]: cannot find function x in this scope`:
this is usually caused by having `#[cfg(kani)]` somewhere in the control flow path of the user's proof harness.
To fix this, remove `#[cfg(kani)]` from those paths.

## Request for comments

This feature is experimental and is therefore subject to change.
If you have ideas for improving the user experience of this feature,
please add them to [this github issue](https://github.com/model-checking/kani/issues/1536).

## Limitations 

1. This feature does not generate unit tests for failing non-panic checks (e.g., UB checks).
This is because checks would not trigger runtime errors during concrete playback.
Kani generates warning messages for this.
2. This feature does not support generating unit tests for multiple assertion failures within the same harness.
This limitation might be removed in the future.
Kani generates warning messages for this.
3. This feature requires that you do not change your code or runtime configurations between when Kani generates the unit test and when you run it.
For instance, if you linked with library A during unit test generation and library B during unit test play back,
that might cause unintended errors in the unit test counterexample.
Kani currently has no way to detect this issue.
