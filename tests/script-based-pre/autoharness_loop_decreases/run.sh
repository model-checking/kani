#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# `#[kani::loop_decreases]` defines a helper function in the function that has the loop.
# Autoharness must not generate a harness for that helper: it must only verify `count_down`.
kani autoharness -Z autoharness -Z loop-contracts --output-format=regular count_down.rs
