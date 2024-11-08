# Installing from source code

> If you were able to [install Kani](install-guide.md) normally, you do not need to build Kani from source.
> You probably want to proceed to the [Kani tutorial](kani-tutorial.md).

## Dependencies

In general, the following dependencies are required to build Kani from source.

> **NOTE**: These dependencies may be installed by running the scripts shown
> below and don't need to be manually installed.

1. Cargo installed via [rustup](https://rustup.rs/)
2. [CBMC](https://github.com/diffblue/cbmc) (latest release)
3. [Kissat](https://github.com/arminbiere/kissat) (Release 3.1.1)

Kani has been tested in [Ubuntu](#install-dependencies-on-ubuntu) and [macOS](##install-dependencies-on-macos) platforms.

### Install dependencies on Ubuntu

Support is available for Ubuntu 18.04, 20.04 and 22.04.
The simplest way to install dependencies (especially if you're using a fresh VM)
is following our CI scripts:

```
# git clone git@github.com:model-checking/kani.git
git clone https://github.com/model-checking/kani.git
cd kani
git submodule update --init
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

Build the Kani package:

```
cargo build-dev
```

Then, optionally, run the regression tests:

```
./scripts/kani-regression.sh
```

This script has a lot of noisy output, but on a successful run you'll see at the end of the execution:

```
All Kani regression tests completed successfully.
```

## Adding Kani to your path

To use a locally-built Kani from anywhere, add the Kani scripts to your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

## Next steps

If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
