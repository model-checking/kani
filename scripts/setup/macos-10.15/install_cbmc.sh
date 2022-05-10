#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Specify the CBMC version file"
  exit 1
fi

CBMC_VERSION_FILE=$1
CBMC_VERSION=$(cat ${CBMC_VERSION_FILE})

# Install CBMC for macOS 10.15 from CBMC tap
# https://github.com/diffblue/cbmc/blob/develop/doc/ADR/homebrew_tap.md
brew tap diffblue/cbmc
brew install diffblue/cbmc/cbmc@${CBMC_VERSION}
