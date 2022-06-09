# Installing from source code

This installation option is better suited for Kani developers
who need to change the Kani source code.

## Dependencies

In general, the following dependencies are required.

> **NOTE**: These dependencies may be installed by running the CI scripts shown
> below and there's no need to install them separately, for their respective
> OS.

1. Cargo installed via [rustup](https://rustup.rs/)
2. [CBMC](https://github.com/diffblue/cbmc) (>= 5.58.1)
3. [CBMC Viewer](https://github.com/awslabs/aws-viewer-for-cbmc) (>= 2.10)

Kani has been tested in [Ubuntu](#install-dependencies-on-ubuntu) and [macOS](##install-dependencies-on-macos) platforms.

### Install dependencies on Ubuntu

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

### Install dependencies on macOS

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

## Adding Kani to your path

To use Kani from anywhere, add the Kani scripts to your path:

```bash
export PATH=$(pwd)/scripts:$PATH
```

## Next steps

To check your install, you can
[run a basic Kani test](./install-check.md).
If you're learning Kani for the first time, you may be interested in our [tutorial](kani-tutorial.md).
