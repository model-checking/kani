# RMC Installation Guide

RMC has not yet reached the point where releases are available, and so to use RMC you must currently build from source.

In general, the following dependencies are required:

1. The dependencies needed to built `rustc`. RMC is a fork of the Rust compiler, and so we have the same minimum requirements.
2. CBMC (>= 5.30.1)
3. CBMC Viewer (>= 2.6)

## Installing on Ubuntu 20.04

We recommend trying out RMC with Ubuntu 20.04.
The simplest way to install is following our CI scripts:

```
git clone git@github.com:model-checking/rmc.git
cd rmc
git submodule update --init
./scripts/setup/ubuntu-20.04/install_deps.sh
./scripts/setup/ubuntu-20.04/install_cbmc.sh
./scripts/setup/install_viewer.sh 2.6
./scripts/setup/install_rustup.sh
```

Perform one-time build configuration:

```
./configure \
    --enable-debug \
    --set=llvm.download-ci-llvm=true \
    --set=rust.debug-assertions-std=false \
    --set=rust.deny-warnings=false
```

**NOTE: If you skip the above, builds may take a long time as all of LLVM would need to be built from scratch.**

Then build RMC:

```
./x.py build -i --stage 1 library/std
```

Then, optionally, run the regression tests:

```
./scripts/rmc-regression.sh
```

## Try running RMC

Get the RMC script in your path:

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
