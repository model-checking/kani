# Debugging verification failures

When the result of a certain check comes back as a `FAILURE`,
Kani offers different options to help debug:
* `--concrete-playback`. This _experimental_ feature generates a Rust unit test case that plays back a failing
proof harness using a concrete counterexample.
* `--visualize`. This feature generates an HTML text-based trace that
enumerates the execution steps leading to the check failure.

## Concrete playback

When concrete playback is enabled, Kani will generate unit tests for assertions that failed during verification,
as well as cover statements that are reachable.

These tests can then be executed using Kani's playback subcommand.

### Usage

In order to enable this feature, run Kani with the `-Z concrete-playback --concrete-playback=[print|inplace]` flag.
After getting a verification failure, Kani will generate a Rust unit test case that plays back a failing
proof harness with a concrete counterexample.
The concrete playback modes mean the following:
* `print`: Kani will just print the unit test to stdout.
You will then need to copy this unit test into the same module as your proof harness.
This is also helpful if you just want to quickly find out which values were assigned by `kani::any()` calls.
* `inplace`: Kani will automatically copy the unit test into your source code.
Before running this mode, you might find it helpful to have your existing code committed to `git`.
That way, you can easily remove the unit test with `git revert`.
Note that Kani will not copy the unit test into your source code if it detects
that the exact same test already exists. 

After the unit test is in your source code, you can run it with the `playback` subcommand.
To debug it, there are a couple of options:
* You can try [Kani's experimental extension](https://github.com/model-checking/kani-vscode-extension)
provided for VSCode.
* Otherwise, you can debug the unit test on the command line.

To manually compile and run the test, you can use Kani's `playback` subcommand:
```
cargo kani playback -Z concrete-playback -- {unit_test_func_name}
```

The output from this command is similar to `cargo test`.
The output will have a line in the beginning like
`Running unittests {files} ({binary})`.

You can further debug the binary with tools like `rust-gdb` or `lldb`.

### Example

Running `kani -Z concrete-playback --concrete-playback=print` on the following source file:
```rust
#[kani::proof]
fn proof_harness() {
    let a: u8 = kani::any();
    let b: u16 = kani::any();
    assert!(a / 2 * 2 == a &&
            b / 2 * 2 == b);
}
```
yields a concrete playback Rust unit test similar to the one below:
```rust
#[test]
fn kani_concrete_playback_proof_harness_16220658101615121791() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 133
        vec![133],
        // 35207
        vec![135, 137],
    ];
    kani::concrete_playback_run(concrete_vals, proof_harness);
}
```
Here, `133` and `35207` are the concrete values that, when substituted for `a` and `b`,
cause an assertion failure.
`vec![135, 137]` is the byte array representation of `35207`.

### Request for comments

This feature is experimental and is therefore subject to change.
If you have ideas for improving the user experience of this feature,
please add them to [this GitHub issue](https://github.com/model-checking/kani/issues/1536).
We are tracking the existing feature requests in
[this GitHub milestone](https://github.com/model-checking/kani/milestone/10).

### Limitations 

* This feature does not generate unit tests for failing non-panic checks (e.g., UB checks).
This is because checks would not trigger runtime errors during concrete playback.
Kani generates warning messages for this.
* This feature does not support generating unit tests for multiple assertion failures within the same harness.
This limitation might be removed in the future.
Kani generates warning messages for this.
* This feature requires that you use the same Kani version to generate the test and to playback. 
Any extra compilation option used during verification must be used during playback.
