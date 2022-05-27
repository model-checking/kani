#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

UBUNTU_VERSION=$(lsb_release -rs)
CBMC_VERSION=5.58.1
FILE="ubuntu-${UBUNTU_VERSION}-cbmc-${CBMC_VERSION}-Linux.deb"
URL="https://github.com/diffblue/cbmc/releases/download/cbmc-${CBMC_VERSION}/$FILE"

set -x

wget -O "$FILE" "$URL"
sudo dpkg -i "$FILE"

cbmc --version

# Clean up on success
rm $FILE
