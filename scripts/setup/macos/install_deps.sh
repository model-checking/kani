#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we could skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
brew update

# Install SMT solvers being used in regression tests. Homebrew no longer ships bottles for macOS 14
# or for Intel Macs, where `brew install` builds from source: z3 5.3.1 does not compile with the
# macOS 14 runner's clang, and building takes a long time on Intel. Use the release binaries, which
# support macOS 13.3 and later. (Nothing else installed here pulls in Homebrew's Python, so the
# runner's own Python 3 is used.)
ARCH=$(uname -m)
Z3_VERSION=5.3.1
Z3_ARCH=$([ "${ARCH}" = "x86_64" ] && echo x64 || echo "${ARCH}")
Z3_DIR=z3-${Z3_VERSION}-${Z3_ARCH}-osx-13.3
curl -L --remote-name https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/${Z3_DIR}.zip
sudo unzip -o -j -d /usr/local/bin ${Z3_DIR}.zip ${Z3_DIR}/bin/z3
rm ${Z3_DIR}.zip
z3 --version

curl -L --remote-name https://github.com/cvc5/cvc5/releases/download/cvc5-1.3.0/cvc5-macOS-${ARCH}-static.zip
sudo unzip -o -j -d /usr/local/bin cvc5-macOS-${ARCH}-static.zip cvc5-macOS-${ARCH}-static/bin/cvc5
rm cvc5-macOS-${ARCH}-static.zip
cvc5 --version

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
