# Kani Installation Guide

Kani must currently be built from source.

In general, the following dependencies are required. Note: These dependencies may be installed by running the CI scripts shown below and there is no need to install them separately, for their respective OS.

1. Cargo installed via rustup
2. [CBMC](https://github.com/diffblue/cbmc) (>= 5.53.1)
3. [CBMC Viewer](https://github.com/awslabs/aws-viewer-for-cbmc) (>= 2.10)

## Installing on Ubuntu 20.04

The simplest way to install (especially if you're using a fresh VM) is following our CI scripts:

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
./scripts/setup/ubuntu-20.04/install_deps.sh
./scripts/setup/ubuntu-20.04/install_cbmc.sh
./scripts/setup/install_viewer.sh 2.10
./scripts/setup/install_rustup.sh
source $HOME/.cargo/env
```

## Installing on Mac OS

You need to have [Homebrew](https://brew.sh/) installed already.

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

## Building and testing Kani

Build Kani's packages:

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

Get the Kani script in your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

Create a test file:

```rust
// File: test.rs
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
[snipped output]
RESULTS:
Check 1: main.assertion.1
         - Status: FAILURE
         - Description: "assertion failed: 1 == 2"
[...]
VERIFICATION:- FAILED
```

Fix the test and you should see `kani` succeed.
