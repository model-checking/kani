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
# `string_head`/`string_first_byte` reason about UTF-8 over a bounded nondeterministic
# `String`, which is expensive; on slower CI runners it can exceed the autoharness default
# 60s harness timeout, so raise it here to keep these harnesses from spuriously timing out.
# This run reports failures (`vec_first`/`string_first_byte`), so it exits non-zero (see config.yml).
cargo kani autoharness -Z autoharness -Z unstable-options --bounded-arguments --harness-timeout 5m
