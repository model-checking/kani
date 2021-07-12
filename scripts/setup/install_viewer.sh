#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Install cbmc-viewer

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Specify the version to install"
  exit 1
fi

FILE="cbmc_viewer-$1-py3-none-any.whl"
URL="https://github.com/awslabs/aws-viewer-for-cbmc/releases/download/viewer-$1/$FILE"

set -x

curl --fail --silent --location "$URL" -o "$FILE"
sudo python3 -m pip install --upgrade "$FILE"
