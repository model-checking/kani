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

# Binaries are not released for AL2, so build from source
WORK_DIR=$(mktemp -d)
git clone \
  --branch cbmc-${CBMC_VERSION} --depth 1 \
  https://github.com/diffblue/cbmc \
  "${WORK_DIR}"

pushd "${WORK_DIR}"

mkdir build
git submodule update --init

cmake3 -S . -Bbuild -DWITH_JBMC=OFF -Dsat_impl="minisat2;cadical"
cmake3 --build build -- -j$(nproc)
sudo make -C build install

popd
rm -rf "${WORK_DIR}"
