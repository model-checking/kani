# Installation

Kani offers an easy installation option on three platforms:

* `x86_64-unknown-linux-gnu` (Most Linux distributions)
* `x86_64-apple-darwin` (Intel Mac OS)
* `aarch64-apple-darwin` (Apple Silicon Mac OS)

Other platforms are either not yet supported or require instead that
you [build from source](build-from-source.md). To use Kani in your
GitHub CI workflows, see [GitHub CI Action](./install-github-ci.md).

## Dependencies

The following must already be installed:

* **Python version 3.7 or newer** and the package installer `pip`.
* Rust 1.58 or newer installed via `rustup`.
* `ctags` is required for Kani's `--visualize` option to work correctly. [Universal ctags](https://ctags.io/) is recommended.

## Installing the latest version

To install the latest version of Kani, run:

```bash
cargo install --locked kani-verifier
cargo kani setup
```

This will build and place in `~/.cargo/bin` (in a typical environment) the `kani` and `cargo-kani` binaries.
The second step (`cargo kani setup`) will download the Kani compiler and other necessary dependencies, and place them under `~/.kani/` by default.
A custom path can be specified using the `KANI_HOME` environment variable.

## Installing an older version

```bash
cargo install --locked kani-verifier --version <VERSION>
cargo kani setup
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
