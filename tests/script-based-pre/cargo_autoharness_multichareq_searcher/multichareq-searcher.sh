#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -euo pipefail

echo "[without --bounded-arguments]"
# Capture the command status explicitly, then search the captured output with a here-string
# rather than a pipeline: `grep -m1` exits at its first match, which could make an `echo` on the
# left of a pipe fail with SIGPIPE, and `pipefail` would then fail the script.
list_output=$(cargo kani autoharness -Z autoharness --list 2>&1)
grep -m1 'Requires --bounded-arguments' <<< "$list_output"

echo "[with --bounded-arguments]"
# The haystack of every searcher is sized by the string bound, and the needle of a
# `CharSliceSearcher` by the slice bound; both are passed explicitly so that the test does not
# depend on their defaults.
# The other autoharness tests use a 5m harness timeout. The slowest harness here
# (`char_slice_value`) takes about 70s locally and gets close to 5 minutes on slower CI runners
# (macos-15-intel), so this test gets a wider limit to avoid a spurious timeout. The verification
# time is driven mostly by the string bound (the haystack size), not by the slice bound.
# This run reports a failure (`haystack_len_is_not_three`), so it exits non-zero (see
# config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular \
    --bounded-arguments --string-bound 4 --slice-bound 4 --harness-timeout 10m
