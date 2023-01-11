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

# CBMC currently only release a 18.04 and a 20.04 versions.
if [[ "${MAJOR}" -le "18" ]]
then
    MIRROR_VERSION="18.04"
else
    MIRROR_VERSION="20.04"
fi

FILE="ubuntu-${MIRROR_VERSION}-cbmc-${CBMC_VERSION}-Linux.deb"
URL="https://github.com/diffblue/cbmc/releases/download/cbmc-${CBMC_VERSION}/$FILE"

set -x

wget -O "$FILE" "$URL"
sudo dpkg -i "$FILE"

cbmc --version

# Clean up on success
rm $FILE
