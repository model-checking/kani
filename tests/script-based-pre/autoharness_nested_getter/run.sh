#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

kani autoharness -Z autoharness --check-invariants --output-format=regular \
    --include-pattern make_ nested.rs
