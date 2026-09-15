#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Pattern types (`RigidTy::Pat`) wrap a base scalar type with a validity constraint
# (e.g. `pattern_type!(*const T is !null)` for NonNull). Autoharness must recognize
# them as derivable and constrain generated values to the pattern's valid range.
kani autoharness -Z autoharness --output-format=regular pattern_type_probe.rs
