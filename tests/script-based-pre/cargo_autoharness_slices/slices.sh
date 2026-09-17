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
# `count_a` iterates over every `char` of a nondeterministic string, which is expensive to
# reason about; on slower CI runners it can exceed the autoharness default 60s harness timeout,
# so raise it here to keep this (genuinely passing) harness from spuriously timing out.
# This run reports failures (`first`/`first_byte`), so it exits non-zero (see config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --output-format=regular --bounded-arguments --harness-timeout 5m
