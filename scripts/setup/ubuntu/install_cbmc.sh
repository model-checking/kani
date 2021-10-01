#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

UBUNTU_VERSION=$(lsb_release -rs)
FILE="ubuntu-${UBUNTU_VERSION}-cbmc-5.36.0-Linux.deb"
URL="https://github.com/diffblue/cbmc/releases/download/cbmc-5.36.0/$FILE"

set -x

wget -O "$FILE" "$URL"
sudo dpkg -i "$FILE"

cbmc --version
