#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Install CBMC 5.30.1 for Ubuntu 20.04

FILE="ubuntu-20.04-cbmc-5.30.1-Linux.deb"
URL="https://github.com/diffblue/cbmc/releases/download/cbmc-5.30.1/$FILE"

set -x

wget -O "$FILE" "$URL"
sudo dpkg -i "$FILE"

cbmc --version
