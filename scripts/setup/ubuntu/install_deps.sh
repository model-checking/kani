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
  software-properties-common
  wget
  zlib1g
  zlib1g-dev
)

# Version specific dependencies.
declare -A VERSION_DEPS
VERSION_DEPS["20.04"]="python-is-python3"
VERSION_DEPS["18.04"]=""

set -x

sudo apt-get --yes update
sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}"

UBUNTU_VERSION=$(lsb_release -rs)
OTHER_DEPS="${VERSION_DEPS[${UBUNTU_VERSION}]:-""}"
if [[ ! -z ${OTHER_DEPS} ]]
then
    # This package was added on ubuntu 20.04.
    sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${OTHER_DEPS[@]}"
fi

# Add Python package dependencies
PYTHON_DEPS=(
  toml # Used for parsing `cargo-rmc` config toml
)

python3 -m pip install "${PYTHON_DEPS[@]}"
