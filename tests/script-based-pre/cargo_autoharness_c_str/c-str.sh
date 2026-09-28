#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo "[without --bounded-arguments]"
# Capture the command status explicitly: piping straight into `grep` would mask a
# failure of `cargo kani ... --list` (the pipeline would report grep's status).
list_output=$(cargo kani autoharness -Z autoharness --list 2>&1)
echo "$list_output" | grep -m1 'Requires --bounded-arguments'

echo "[with --bounded-arguments]"
# This run reports a failure (`first`), so it exits non-zero (see config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular --bounded-arguments --harness-timeout 5m
