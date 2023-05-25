#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we can skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
#brew update

# Install dependencies via `brew`
brew install universal-ctags wget jq

# Add Python package dependencies
PYTHON_DEPS=(
  autopep8
)

python3 -m pip install "${PYTHON_DEPS[@]}"

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
${SCRIPT_DIR}/install_viewer.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
