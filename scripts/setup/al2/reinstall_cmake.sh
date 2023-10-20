#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

CMAKE_VERSION="3.27.7"

# Remove other versions of CMake
sudo yum -y remove cmake

sudo rm -rf /tmp/cmake_installation
mkdir /tmp/cmake_installation
pushd /tmp/cmake_installation

wget https://github.com/Kitware/CMake/releases/download/v"${CMAKE_VERSION}"/cmake-"${CMAKE_VERSION}".tar.gz
tar -xzvf cmake-"${CMAKE_VERSION}".tar.gz
cd cmake-"${CMAKE_VERSION}"

./bootstrap
make -j$(nproc)
sudo make install

popd