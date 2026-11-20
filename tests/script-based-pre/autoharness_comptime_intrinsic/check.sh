#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

kani autoharness -Z autoharness --only-codegen --output-format=terse comptime.rs
