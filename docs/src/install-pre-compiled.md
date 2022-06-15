# Installing from pre-compiled binaries

This is the default installation option
that is suitable for most users.

## Dependencies

The following must already be installed:

* **Python version 3.6 or greater** and the package installer pip.
* Rust installed via `rustup`.
* `ctags` is required for Kani's `--visualize` option to work correctly.

## Installing the latest version

To install the latest version of Kani, run:

```bash
cargo install --locked kani-verifier
cargo-kani setup
```

This will build and place in `~/.cargo/bin` (in a typical environment) the `kani` and `cargo-kani` binaries.
The second step (`cargo-kani setup`) will download the Kani compiler and other necessary dependencies (and place them under `~/.kani/`).

## Installing an older version

```bash
cargo install --lock kani-verifier --version <VERSION>
cargo-kani setup
```

## Checking your installation 

After you've installed Kani,
you can try running it by creating a test file:

```rust
// File: test.rs
#[kani::proof]
fn main() {
    assert!(1 == 2);
}
```

Run Kani on the single file:

```
kani test.rs
```

You should get a result like this one:

```
[...]
RESULTS:
Check 1: main.assertion.1
         - Status: FAILURE
         - Description: "assertion failed: 1 == 2"
[...]
VERIFICATION:- FAILED
```

Fix the test and you should see a result like this one:

```
[...]
VERIFICATION:- SUCCESSFUL
```

## Next steps

If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
