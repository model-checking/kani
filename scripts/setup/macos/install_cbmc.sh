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
FORMULA=diffblue/cbmc/cbmc@${CBMC_VERSION}
if [ "$(uname -m)" = "x86_64" ]; then
  # The tap's only Intel bottle is for macOS 14 (Sonoma), so on later macOS versions
  # `brew install` builds CBMC from source. That now fails: Intel Macs are a Tier 3 platform
  # for Homebrew, and the `openjdk@21` build dependency requires arm64. Install the binaries of
  # the Sonoma bottle instead, which run on later macOS versions and do not depend on their
  # install location. Its checksum is taken from the tap's formula.
  FORMULA_FILE="$(brew --repository diffblue/cbmc)/Formula/cbmc@${CBMC_VERSION}.rb"
  BOTTLE_SHA=$(sed -n 's/.* sonoma: *"\([0-9a-f]*\)".*/\1/p' "${FORMULA_FILE}")
  BOTTLE=cbmc@${CBMC_VERSION}-${CBMC_VERSION}.sonoma.bottle.tar.gz
  BOTTLE_DIR=$(mktemp -d)
  curl -fL -o "${BOTTLE_DIR}/${BOTTLE}" \
    "https://github.com/diffblue/homebrew-cbmc/releases/download/bag-of-goodies/${BOTTLE}"
  echo "${BOTTLE_SHA}  ${BOTTLE_DIR}/${BOTTLE}" | shasum -a 256 -c -
  tar -xzf "${BOTTLE_DIR}/${BOTTLE}" -C "${BOTTLE_DIR}"
  sudo install -m 0755 "${BOTTLE_DIR}/cbmc@${CBMC_VERSION}/${CBMC_VERSION}"/bin/* /usr/local/bin/
  rm -rf "${BOTTLE_DIR}"
else
  brew install --overwrite "${FORMULA}"
fi
cbmc --version
