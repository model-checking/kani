#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

cargo kani autoharness -Z autoharness --output-format=regular --include-pattern cargo_autoharness_include::include
