#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

UBUNTU_VERSION=$(lsb_release -rs)
CBMC_VERSION=5.56.0
FILE="ubuntu-${UBUNTU_VERSION}-cbmc-${CBMC_VERSION}-Linux.deb"
URL="https://github.com/diffblue/cbmc/releases/download/cbmc-${CBMC_VERSION}/$FILE"

set -x

wget -O "$FILE" "$URL"
sudo dpkg -i "$FILE"

cbmc --version

# Clean up on success
rm $FILE
