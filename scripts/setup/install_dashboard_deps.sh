#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# The RMC Dashboard is generated using [Litani](https://github.com/awslabs/aws-build-accumulator)

# Litani's dependencies:
DEPS=(
  gnuplot # Not required but recommended
  graphviz
)

PYTHON_DEPS=(
  jinja2
)

python3 -m pip install "${PYTHON_DEPS[@]}"