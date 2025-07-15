#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we could skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
brew update

# Install Python separately to workround recurring homebrew CI issue.
# See https://github.com/actions/runner-images/issues/9471 for more details.
brew install python@3 || true
brew link --overwrite python@3

# Install SMT solvers being used in regression tests
brew install z3
ARCH=$(uname -m)
curl -L --remote-name https://github.com/cvc5/cvc5/releases/download/cvc5-1.3.0/cvc5-macOS-${ARCH}-static.zip
sudo unzip -j -d /usr/local/bin cvc5-macOS-${ARCH}-static.zip cvc5-macOS-${ARCH}-static/bin/cvc5
rm cvc5-macOS-${ARCH}-static.zip
cvc5 --version

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
