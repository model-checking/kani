# Installation

To install the latest version of Kani, run:

```bash
cargo install --locked kani-verifier
cargo-kani setup
```

This will build and place in `~/.cargo/bin` (in a typical environment) the `kani` and `cargo-kani` binaries.
The second step (`cargo-kani setup`) will download the Kani compiler and other necessary dependencies (and place them under `~/.kani/`).

Currently, only two platforms are supported:

* `x86_64-unknown-linux-gnu`
* `x86_64-apple-darwin`

The following must already be installed:

* **Python version 3.8 or greater** and the package installer pip.
* Rust installed via `rustup`
* `ctags` is required for Kani's `--visualize` option to work correctly.

# Installing an older version of Kani

```bash
cargo install --lock kani-verifier --version <VERSION>
cargo-kani setup
```

# Building from source

In general, the following dependencies are required.

> **NOTE**: These dependencies may be installed by running the CI scripts shown
> below and there's no need to install them separately, for their respective
> OS.

1. Cargo installed via [rustup](https://rustup.rs/)
2. [CBMC](https://github.com/diffblue/cbmc) (>= 5.59.0)
3. [CBMC Viewer](https://github.com/awslabs/aws-viewer-for-cbmc) (>= 2.10)

Kani has been tested in [Ubuntu](#install-dependencies-on-ubuntu) and [macOS](##install-dependencies-on-macos) platforms.

## Install dependencies on Ubuntu

Support is available for Ubuntu 18.04 and 20.04.
The simplest way to install dependencies (especially if you're using a fresh VM)
is following our CI scripts:

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
./scripts/setup/ubuntu/install_deps.sh
./scripts/setup/ubuntu/install_cbmc.sh
./scripts/setup/install_viewer.sh 3.2
./scripts/setup/install_rustup.sh
# If you haven't already:
source $HOME/.cargo/env
```

## Install dependencies on macOS

Support is available for macOS 10.15. You need to have [Homebrew](https://brew.sh/) installed already.

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
./scripts/setup/macos-10.15/install_deps.sh
./scripts/setup/macos-10.15/install_cbmc.sh
./scripts/setup/install_viewer.sh 3.2
./scripts/setup/install_rustup.sh
# If you haven't already:
source $HOME/.cargo/env
```

## Build and test Kani

Build the Kani package:

```
cargo build --workspace
```

Then, optionally, run the regression tests:

```
./scripts/kani-regression.sh
```

This script has a lot of noisy output, but on a successful run you'll see:

```
All Kani regression tests completed successfully.
```

## Try running Kani

Add the Kani scripts to your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

Create a test file:

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
