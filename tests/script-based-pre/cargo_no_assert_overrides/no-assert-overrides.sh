#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

echo "[without flag]"
cargo kani 2>&1 | grep -m1 'E0659'
echo "[with flag]"
cargo kani --no-assert-overrides
