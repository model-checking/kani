#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Dependencies.
DEPS=(
  git
  python3-pip # Default in CI, but missing in AWS AMI
  python3-setuptools
)

# Version specific dependencies.
declare -A VERSION_DEPS
VERSION_DEPS["20.04"]="universal-ctags python-is-python3"
VERSION_DEPS["18.04"]="exuberant-ctags"

UBUNTU_VERSION=$(lsb_release -rs)
OTHER_DEPS="${VERSION_DEPS[${UBUNTU_VERSION}]:-""}"

set -x

# Github promises weekly build image updates, but recommends running
# `sudo apt-get update` before installing packages in case the `apt`
# index is stale. This prevents package installation failures.
# https://docs.github.com/en/actions/using-github-hosted-runners/customizing-github-hosted-runners#installing-software-on-ubuntu-runners
sudo apt-get --yes update

sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}" ${OTHER_DEPS[@]}

# Add Python package dependencies
PYTHON_DEPS=(
  autopep8
)

python3 -m pip install "${PYTHON_DEPS[@]}"
