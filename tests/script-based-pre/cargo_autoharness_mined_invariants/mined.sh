#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Mined type invariants (assertions over self fields stated by >= 2 methods):
# - assumed for generated values under --constructor-args (day_user passes; the
#   single-method precondition on Gauge::drain is NOT mined, so prec_user still fails);
# - checked on return values under --check-invariants (buggy_make_day fails with the
#   distinct property class; good_make_day passes).
cargo kani autoharness -Z autoharness --constructor-args --check-invariants --output-format=regular
