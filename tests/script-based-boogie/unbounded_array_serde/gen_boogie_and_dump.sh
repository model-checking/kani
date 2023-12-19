#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set +e

# Delete any leftover Boogie files
rm *.bpl

echo "[TEST] Run verification..."
kani -Zboogie test.rs

cat *.bpl
