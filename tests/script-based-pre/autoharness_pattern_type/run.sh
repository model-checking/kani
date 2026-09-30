#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Pattern types (`RigidTy::Pat`) wrap a base scalar type with a validity constraint
# (e.g. `pattern_type!(u8 is 0..=100)`; std's niche types are built on them). Autoharness
# must recognize them as derivable and constrain generated values to the pattern's range.
# `-Z valid-value-checks` verifies the range is assumed *before* the value is transmuted to
# the pattern type (the transmute itself is validity-checked under that flag).
kani autoharness -Z autoharness -Z valid-value-checks --output-format=regular pattern_type_probe.rs
