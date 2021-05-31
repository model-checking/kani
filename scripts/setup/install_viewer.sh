#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install cbmc-viewer
if [[ $# -eq 1 ]] ; then
wget https://github.com/awslabs/aws-viewer-for-cbmc/releases/download/viewer-$1/cbmc_viewer-$1-py3-none-any.whl \
  && sudo python3 -m pip install --upgrade cbmc_viewer-$1-py3-none-any.whl
else
  echo "Error: Specify the version to install"
  exit 1
fi
