#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Dependencies.
DEPS=(
  bison
  cmake
  curl
  flex
  g++
  gcc
  git
  gpg-agent
  make
  patch
  wget
  zlib1g
  zlib1g-dev
)

set -x

# Github promises weekly build image updates, but recommends running
# `sudo apt-get update` before installing packages in case the `apt`
# index is stale. This prevents package installation failures.
# https://docs.github.com/en/actions/using-github-hosted-runners/customizing-github-hosted-runners#installing-software-on-ubuntu-runners
sudo apt-get --yes update

sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}"

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
