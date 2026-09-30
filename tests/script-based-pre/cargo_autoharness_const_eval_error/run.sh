#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A constant whose evaluation fails (a type too big for the target) should be reported as a
# regular compilation error, not crash the reachability collector with "Instance with
# polymorphic constant". See https://github.com/model-checking/kani/issues/4814.
kani autoharness -Z autoharness --output-format=regular large_array.rs
