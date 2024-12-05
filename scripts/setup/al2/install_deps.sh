#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Dependencies.
# Note: CMake 3.8 or higher is required to build CBMC, but those versions are
# only available in AWS AMIs through `cmake3`. So we install `cmake3` and use it
# to build CBMC.
DEPS=(
  cmake
  cmake3
  gcc10-c++
  git
  openssl-devel
  wget
)

set -x

sudo yum -y update
sudo yum -y groupinstall "Development Tools"
sudo yum -y install "${DEPS[@]}"

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
${SCRIPT_DIR}/../install_rustup.sh
