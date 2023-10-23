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
  git
  openssl-devel
  python3-pip
  wget
)

set -x

sudo yum -y update
sudo yum -y groupinstall "Development Tools"
sudo yum -y install "${DEPS[@]}"

# Add Python package dependencies
python3 -m pip install autopep8

# Get the directory containing this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

${SCRIPT_DIR}/install_cbmc.sh
${SCRIPT_DIR}/install_viewer.sh
# The Kissat installation script is platform-independent, so is placed one level up
${SCRIPT_DIR}/../install_kissat.sh
