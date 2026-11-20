#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Source kani-dependencies to get CBMC_VERSION
source kani-dependencies

if [ -z "${CBMC_VERSION:-}" ]; then
  echo "$0: Error: CBMC_VERSION is not specified"
  exit 1
fi

# Install CBMC for macOS from CBMC tap
# https://github.com/diffblue/cbmc/blob/develop/doc/ADR/homebrew_tap.md
# Homebrew 6.0+ requires explicit trust for third-party taps
brew trust diffblue/cbmc 2>/dev/null || true
brew tap diffblue/cbmc
brew install --overwrite diffblue/cbmc/cbmc@${CBMC_VERSION}
