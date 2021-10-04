#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we can skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
#brew update

# Install dependencies via `brew`
brew install ctags

# Add Python package dependencies
PYTHON_DEPS=(
  toml # Used for parsing `cargo-rmc` config toml
)

python3 -m pip install "${PYTHON_DEPS[@]}"
