#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Set the timeout to 5m to ensure that the gcd_recursion test gets killed because of the unwind bound
# and not because CBMC times out.
cargo kani autoharness -Z autoharness -Z function-contracts --harness-timeout 5m -Z unstable-options
