#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Github promises weekly build image updates, so we can skip the update step and
# worst case we should only be 1-2 weeks behind upstream brew.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
#brew update

# Add Python package dependencies
PYTHON_DEPS=(
  autopep8
)

python3 -m pip install "${PYTHON_DEPS[@]}"
