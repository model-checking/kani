#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

DEPS=(
  bison
  cmake
  ctags
  curl
  flex
  g++
  gcc
  git
  gpg-agent
  libssl-dev
  lsb-release
  make
  ninja-build
  patch
  pkg-config
  python-is-python3
  python3-pip # Default in CI, but missing in AWS AMI
  software-properties-common
  wget
  zlib1g
  zlib1g-dev
)

set -x

sudo apt-get --yes update
sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}"

# Add Python package dependencies
PYTHON_DEPS=(
  toml # Used for parsing `cargo-rmc` config toml
)

python3 -m pip install "${PYTHON_DEPS[@]}"
