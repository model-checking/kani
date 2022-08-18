# Debugging verification failures with concrete playback

When the result of a certain check comes back as a `FAILURE`,
Kani offers different options to help debug:
* `--visualize`. This feature generates an HTML text-based trace that
enumerates the execution steps leading to the check failure.
* `--concrete-playback`. This _experimental_ feature allows users to concretely play back
their proof harness as a Rust unit test case.
The following document describes the concrete playback feature in more detail.

## Setup

The user needs to have a fairly recent version of the Kani source code somewhere on their computer.
To do this, run `git clone https://github.com/model-checking/kani.git`.
Then, the user needs to add the following lines to their `Cargo.toml` file:
```toml
[dev-dependencies]
kani = { path = "{path_to_kani}/library/kani", features = ["concrete_playback"] }
```

## Usage

Run Kani with the `--concrete-playback=JustPrint` flag.
After verifying the proof harness checks, Kani will extract concrete values for the `kani::any()` variables
and generate a Rust unit test case.
This unit test initializes the concrete values and calls the proof harness.
A user can then either 1) copy this unit test into the same module as their proof harness or
2) run Kani with the `--concrete-playback=InPlace` flag to have Kani automatically do this.

After the unit test is in their source code, users can run it with `cargo test`.
To debug it, there are a couple of options.
If users have certain IDEs, there are extensions (e.g., `rust-analyzer` for `VS Code`)
that support UI elements like a `Run Test | Debug` button next to all unit tests.
Otherwise, users can debug the unit test on the command line.
To do this, they can first run `cargo test {unit_test_func_name}`.
The output from this will have a line in the beginning like `Running unittests {files} ({binary})`.
They can then debug the binary with tools like `rust-gdb` or `lldb`.

## Common issues

`error[E0425]: cannot find function x in this scope`:
this is usually caused by having `#[cfg(kani)]` somewhere in the control flow path of the user's proof harness.
To fix this, remove `#[cfg(kani)]` from those paths.

## Request for comments

This feature is experimental and is therefore subject to change.
If users have ideas for improving the user experience of this feature,
they can add it to [this github issue](https://github.com/model-checking/kani/issues/1536).

## Limitations 

Currently, this feature does not generate unit tests for failing non-panic checks (e.g., UB checks)
These checks would not trigger runtime errors when playing back the unit test.
It also doesn't support generating unit tests for multiple assertion failures within the same harness.
This limitation might be removed in the future.
For the above two, the feature provides warning messages to the user whenever it sees these occur.

This feature also requires that the user doesn't change their code or runtime configurations between when the unit test
was generated and when it was run.
For instance, if the user linked with library A during unit test generation and library B during unit test play back,
that might cause unintended errors in the unit test counterexample.
Kani currently has no way to mitigate this issue.
