#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we can skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
#brew update

# Install dependencies via `brew`
brew install universal-ctags wget

# Add Python package dependencies
PYTHON_DEPS=(
  autopep8
  colorama # Used for introducing colors into terminal output
)

python3 -m pip install "${PYTHON_DEPS[@]}"
