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

cmake3 -S . -Bbuild -DWITH_JBMC=OFF -Dsat_impl="minisat2;cadical" \
  -DCMAKE_C_COMPILER=gcc10-cc -DCMAKE_CXX_COMPILER=gcc10-c++ \
  -DCMAKE_CXX_STANDARD_LIBRARIES=-lstdc++fs \
  -DCMAKE_CXX_FLAGS=-Wno-error=register
cmake3 --build build -- -j$(nproc)
sudo make -C build install

popd
rm -rf "${WORK_DIR}"
