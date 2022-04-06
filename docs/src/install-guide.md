# Installation

Kani must currently be built from source.

In general, the following dependencies are required.

> **NOTE**: These dependencies may be installed by running the CI scripts shown
> below and there is no need to install them separately, for their respective
> OS.

1. Cargo installed via [rustup](https://rustup.rs/)
2. [CBMC](https://github.com/diffblue/cbmc) (>= 5.54.0)
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
./scripts/setup/install_viewer.sh 2.10
./scripts/setup/install_rustup.sh
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
./scripts/setup/install_viewer.sh 2.10
./scripts/setup/install_rustup.sh
source $HOME/.cargo/env
```

## Build and test Kani

Build the Kani package:

```
cargo build
```

Then, optionally, run the regression tests:

```
./scripts/kani-regression.sh
```

This script has a lot of noisy output, but on a successful run you will see:

```
All Kani regression tests completed successfully.
```

## Try running Kani

Add the Kani script to your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

Create a test file:

```rust,noplaypen
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
