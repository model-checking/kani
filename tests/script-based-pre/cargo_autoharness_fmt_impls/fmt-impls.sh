#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Raise the harness timeout above autoharness's 60s default, like the other autoharness tests: on
# the macOS x86_64 CI runners the `LowerExp`, `UpperExp` and `Pointer` harnesses take 40-60s and
# often hit the default, which makes their expected failures time out instead.
cargo kani autoharness -Z autoharness -Z function-contracts -Z unstable-options --harness-timeout 5m
