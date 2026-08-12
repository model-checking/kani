#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

echo "[without --bounded-arguments]"
cargo kani autoharness -Z autoharness --list 2>&1 | grep -m1 'Requires --bounded-arguments'
echo "[with --bounded-arguments]"
cargo kani autoharness -Z autoharness --bounded-arguments
