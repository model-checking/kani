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
# The needle of a `CharSliceSearcher` is sized by the slice bound, which is lowered here to keep
# the harnesses small.
# This run reports a failure (`haystack_len_is_not_three`), so it exits non-zero (see
# config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular --bounded-arguments --slice-bound 4 --harness-timeout 5m
