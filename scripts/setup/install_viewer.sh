#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# Install cbmc-viewer

if [[ $# -ne 1 ]]; then
  echo "$0: Error: Specify the version to install"
  exit 1
fi

set -x

python3 -m pip install cbmc-viewer==$1
