#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# rustc only accepts `...` with a non-C ABI on x86-64 ("`...` is not supported for
# `extern "sysv64"` functions" elsewhere), so those two checks can only run there. They assert
# inside the script rather than through the `expected` file, which has to match on every target.
if [ "$(uname -m)" = "x86_64" ]; then
    echo "--- reaching a non-C variadic reports an unsupported construct"
    kani calls_variadic.rs --output-format=terse > non_c.log 2>&1 || true
    grep -q "Variadic function with a non-C calling convention is not currently supported" non_c.log
    # Three harnesses reach one (directly, through a foreign declaration, through a function
    # pointer) and fail; taking the address without calling reaches nothing and verifies.
    grep -q "Complete - 1 successfully verified harnesses, 3 failures, 4 total" non_c.log

    echo "--- autoharness names the calling convention, not the VaList argument"
    kani autoharness -Z autoharness --output-format=terse variadic.rs > non_c_auto.log 2>&1 || true
    grep -q "Unsupported variadic calling convention" non_c_auto.log

    rm -f non_c.log non_c_auto.log
fi

echo "--- C variadics are unaffected"
kani autoharness -Z autoharness --output-format=terse c_variadic.rs
