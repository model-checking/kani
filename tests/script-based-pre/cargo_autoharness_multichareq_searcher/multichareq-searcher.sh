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
# `--default-unwind 6` replaces autoharness's default of 20. With a haystack of at most 4 bytes,
# the loops that validate and decode UTF-8 (`core::str::validations`) run at most 4 times, but
# their length is symbolic, so CBMC unrolls them up to the unwinding bound: at 20, the
# `CharSliceSearcher` harnesses reach 1.4-1.7M SAT variables and time out on the macos-15-intel
# runner (https://github.com/model-checking/kani/issues/5162). At 6 they are about 2.3x smaller.
# This keeps the result sound: unwinding assertions stay enabled, so a bound below a loop's real
# iteration count fails verification rather than cutting behavior off. If the string or slice
# bound grows, raise the unwinding bound with it.
# This run reports a failure (`haystack_len_is_not_three`), so it exits non-zero (see
# config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular \
    --bounded-arguments --string-bound 4 --slice-bound 4 --default-unwind 6 --harness-timeout 10m
