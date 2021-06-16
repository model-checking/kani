#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $RMC_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

EXTRA_X_PY_BUILD_ARGS="${EXTRA_X_PY_BUILD_ARGS:-}"

# Required dependencies
check-cbmc-version.py --major 5 --minor 30
check-cbmc-viewer-version.py --major 2 --minor 5

# Formatting check
./x.py fmt --check

# Standalone rmc tests, cargo tests, and expected tests
./x.py build -i --stage 1 library/std ${EXTRA_X_PY_BUILD_ARGS}
./x.py test -i --stage 1 cbmc firecracker prusti smack cargo-rmc expected
