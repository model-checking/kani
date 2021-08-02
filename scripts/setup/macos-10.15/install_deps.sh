#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Update tools in macOS 10.15 via `brew`
brew update
brew install ctags

# Add Python package dependencies
PYTHON_DEPS=(
  toml # Used for parsing `cargo-rmc` config toml
)

python3 -m pip install "${PYTHON_DEPS[@]}"
