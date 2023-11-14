#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Source kani-dependencies to get CBMC_VERSION
source kani-dependencies

if [ -z "${CBMC_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VERSION is not specified"
  exit 1
fi

UBUNTU_VERSION=$(lsb_release -rs)
MAJOR=${UBUNTU_VERSION%.*}

if [[ "${MAJOR}" -gt "18" ]]
then
  FILE="ubuntu-${UBUNTU_VERSION}-cbmc-${CBMC_VERSION}-Linux.deb"
  URL="https://github.com/diffblue/cbmc/releases/download/cbmc-${CBMC_VERSION}/$FILE"

  set -x

  wget -O "$FILE" "$URL"
  sudo dpkg -i "$FILE"
  cbmc --version
  rm $FILE
  exit 0
fi

# Binaries are no longer released for 18.04, so build from source

WORK_DIR=$(mktemp -d)
git clone \
  --branch cbmc-${CBMC_VERSION} --depth 1 \
  https://github.com/diffblue/cbmc \
  "${WORK_DIR}"

pushd "${WORK_DIR}"

mkdir build
git submodule update --init

cmake -S . -Bbuild -DWITH_JBMC=OFF -Dsat_impl="minisat2;cadical"
make -C build -j$(nproc)
cpack -G DEB --config build/CPackConfig.cmake
sudo dpkg -i ./cbmc-*.deb

popd
rm -rf "${WORK_DIR}"
