#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eux

if [[ $# -eq 1 ]] ; then
wget https://apt.llvm.org/llvm.sh \
  && chmod a+x llvm.sh \
  && sudo ./llvm.sh $1
else
  echo "Error: Specify the LLVM version to install"
  exit 1
fi
