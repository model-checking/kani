#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# The book runner report is generated using [Litani](https://github.com/awslabs/aws-build-accumulator)
FILE="litani-1.22.0.deb"
URL="https://github.com/awslabs/aws-build-accumulator/releases/download/1.22.0/$FILE"

set -x

# Install Litani
wget -O "$FILE" "$URL"
sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes ./"$FILE"

PYTHON_DEPS=(
  bs4 # Used for report updates
)

python3 -m pip install "${PYTHON_DEPS[@]}"