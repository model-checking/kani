#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# The book runner report is generated using [Litani](https://github.com/awslabs/aws-build-accumulator)

# Litani's dependencies:
DEPS=(
  gnuplot # Not required but recommended
  graphviz
)

sudo DEBIAN_FRONTEND=noninteractive apt-get install --no-install-recommends --yes "${DEPS[@]}"

PYTHON_DEPS=(
  bs4 # Used for report updates
  jinja2
)

python3 -m pip install "${PYTHON_DEPS[@]}"