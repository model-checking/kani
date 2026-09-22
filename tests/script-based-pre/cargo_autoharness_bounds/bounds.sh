#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

echo "[bound not below the unwinding bound]"
# `--list` does not verify, so this run only exercises the argument check. Capture the command
# status explicitly: piping straight into `grep` would mask a failure of `cargo kani ... --list`
# (the pipeline would report grep's status).
list_output=$(cargo kani autoharness -Z autoharness --list --bounded-arguments --slice-bound 20 2>&1)
echo "$list_output" | grep -m1 'not below the unwinding bound'

echo "[configured bounds]"
# `string_bound` reasons about UTF-8 validity over a nondeterministic string, which is expensive;
# on slower CI runners it exceeds the autoharness default 60s harness timeout, so raise it here.
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular \
    --bounded-arguments --slice-bound 2 --string-bound 2 --bounded-arbitrary-bound 2 \
    --harness-timeout 5m
