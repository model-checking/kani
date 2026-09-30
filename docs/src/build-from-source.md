# Installing from source code

> If you were able to [install Kani](install-guide.md) normally, you do not need to build Kani from source.
> You probably want to proceed to the [Kani tutorial](kani-tutorial.md).

## Dependencies

In general, the following dependencies are required to build Kani from source.

> **NOTE**: These dependencies may be installed by running the scripts shown
> below and don't need to be manually installed.

1. Cargo installed via [rustup](https://rustup.rs/)
2. [CBMC](https://github.com/diffblue/cbmc) (latest release)
3. [Kissat](https://github.com/arminbiere/kissat) (Release 4.0.1)
3. [Z3](https://github.com/Z3Prover/z3)
4. [cvc5](https://github.com/cvc5/cvc5)

Kani has been tested in [Ubuntu](#install-dependencies-on-ubuntu) and [macOS](##install-dependencies-on-macos) platforms.

### Install dependencies on Ubuntu

Support is available for Ubuntu 20.04, 22.04, and 24.04.
The simplest way to install dependencies (especially if you're using a fresh VM)
is following our CI scripts:

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
 # For Ubuntu 20.04, use: `./scripts/setup/ubuntu-20.04/install_deps.sh`
./scripts/setup/ubuntu/install_deps.sh
# If you haven't already (or from https://rustup.rs/):
./scripts/setup/install_rustup.sh
source $HOME/.cargo/env
```

### Install dependencies on macOS

Support is available for macOS 11. You need to have [Homebrew](https://brew.sh/) installed already.

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
./scripts/setup/macos/install_deps.sh
# If you haven't already (or from https://rustup.rs/):
./scripts/setup/install_rustup.sh
source $HOME/.cargo/env
```

## Build and test Kani

Build the Kani package using:

```
cargo build-dev -- --release
```
to compile with optimizations turned on or using:
```
cargo build-dev
```
to compile in debug/development mode.

Then, optionally, run the regression tests:

```
./scripts/kani-regression.sh
```

This script has a lot of noisy output, but on a successful run you'll see at the end of the execution:

```
All Kani regression tests completed successfully.
```

## LLBC back-end

Kani includes an experimental back-end that generates Low-Level Borrow Calculus (LLBC)
output, which is the format used by [Charon](https://github.com/AeneasVerif/charon) and
[Aeneas](https://github.com/AeneasVerif/aeneas). This back-end translates Rust code into
LLBC, enabling integration with verification tools built on the Aeneas framework.

To build Kani with the LLBC back-end enabled, use:

```
cargo build-dev -- --features cprover --features llbc
```

This builds Kani with both the default CBMC/CPROVER back-end and the LLBC back-end.
To run the LLBC-specific regression tests:

```
./scripts/kani-llbc-regression.sh
```

> **NOTE**: The LLBC back-end is experimental and under active development.

## Adding Kani to your path

To use a locally-built Kani from anywhere, add the Kani scripts to your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

## Next steps

If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
