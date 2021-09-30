# RMC Installation Guide

RMC must currently be built from source.

In general, the following dependencies are required:

1. The dependencies needed to built `rustc`. RMC is a fork of the Rust compiler, and so we have the same minimum requirements.
2. [CBMC](https://github.com/diffblue/cbmc) (>= 5.36.0)
3. [CBMC Viewer](https://github.com/awslabs/aws-viewer-for-cbmc) (>= 2.6)

## Installing on Ubuntu 20.04

The simplest way to install (especially if you're using a fresh VM) is following our CI scripts:

```
# git clone git@github.com:model-checking/rmc.git
git clone https://github.com/model-checking/rmc.git
cd rmc
git submodule update --init
./scripts/setup/ubuntu-20.04/install_deps.sh
./scripts/setup/ubuntu-20.04/install_cbmc.sh
./scripts/setup/install_viewer.sh 2.6
./scripts/setup/install_rustup.sh
```

## Installing on Mac OS

You need to have [Homebrew](https://brew.sh/) installed already.

```
# git clone git@github.com:model-checking/rmc.git
git clone https://github.com/model-checking/rmc.git
cd rmc
git submodule update --init
./scripts/setup/macos-10.15/install_deps.sh
./scripts/setup/macos-10.15/install_cbmc.sh
./scripts/setup/install_viewer.sh 2.6
./scripts/setup/install_rustup.sh
```

## Building and testing RMC

Perform one-time build configuration:

```
./configure \
    --enable-debug \
    --set=llvm.download-ci-llvm=true \
    --set=rust.debug-assertions-std=false \
    --set=rust.deny-warnings=false
```

**NOTE: If you skip the above (`llvm.download-ci-llvm=true` specifically), builds may take a long time as all of LLVM would need to be built from scratch.**

Then build RMC:

```
./x.py build -i --stage 1 library/std
```

Then, optionally, run the regression tests:

```
./scripts/rmc-regression.sh
```

This script has a lot of noisy output, but on a successful run you will see:

```
All RMC regression tests completed successfully.
```

## Try running RMC

Get the RMC script in your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

Create a test file:

```rust
// File: test.rs
pub fn main() {
    assert!(1 == 2);
}
```

Run RMC on the single file:

```
rmc test.rs
```

You should get a result like this one:

```
[snipped output]
** Results:
test.rs function main
[main.assertion.1] line 2 assertion failed: 1 == 2: FAILURE

** 1 of 1 failed (2 iterations)
VERIFICATION FAILED
```

Fix the test and you should see `rmc` succeed.
