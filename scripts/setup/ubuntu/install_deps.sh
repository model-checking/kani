#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Dependencies.
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
  python3-pip # Default in CI, but missing in AWS AMI
  python3-setuptools
  software-properties-common
  wget
  zlib1g
  zlib1g-dev
)

# Version specific dependencies.
declare -A VERSION_DEPS
VERSION_DEPS["20.04"]="python-is-python3"
VERSION_DEPS["18.04"]=""

UBUNTU_VERSION=$(lsb_release -rs)
OTHER_DEPS="${VERSION_DEPS[${UBUNTU_VERSION}]:-""}"

set -x

# Github promises weekly build image updates, so we can skip the update step and
# worst case we should only be 1-2 weeks behind upstream repos.
# https://docs.github.com/en/actions/using-github-hosted-runners/about-github-hosted-runners#supported-software
#sudo apt-get --yes update

sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}" "${OTHER_DEPS[@]}"

# Add Python package dependencies
PYTHON_DEPS=(
  toml # Used for parsing `cargo-rmc` config toml
)

python3 -m pip install "${PYTHON_DEPS[@]}"
