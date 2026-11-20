#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail
status=0
kani autoharness -Z uninit-checks -Z autoharness --include-pattern uninit_read --output-format=regular main.rs || status=$?
[[ "$status" == 1 ]]
kani autoharness -Z uninit-checks -Z autoharness --include-pattern initialized_read --output-format=regular main.rs
