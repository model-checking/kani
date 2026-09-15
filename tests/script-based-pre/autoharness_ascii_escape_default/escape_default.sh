#!/usr/bin/env bash

# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

cargo kani autoharness -Z autoharness --output-format=regular 2>&1 \
    | grep -E '^\| autoharness_ascii_escape_default \| consume_escape_default .*Success' \
    | tr -s ' '
