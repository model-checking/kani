#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

# Install cbmc-viewer 2.5
wget https://github.com/awslabs/aws-viewer-for-cbmc/releases/download/viewer-2.5/cbmc_viewer-2.5-py3-none-any.whl \
  && sudo python3 -m pip install --upgrade cbmc_viewer-2.5-py3-none-any.whl
