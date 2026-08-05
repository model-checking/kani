#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Without --constructor-args, raw field synthesis violates the private types' representation
# invariants and reports false alarms; with it, values are generated through the types'
# public constructors and the false alarms disappear (harnesses are marked "(ctor)").
echo "=== without flag ==="
cargo kani autoharness -Z autoharness --output-format=regular 2>&1 \
    | grep -E '^\| cargo_autoharness_constructor \| .*(Success|Failure)' | tr -s ' ' | sort
echo "=== with flag ==="
cargo kani autoharness -Z autoharness --constructor-args --output-format=regular 2>&1 \
    | grep -E '^\| cargo_autoharness_constructor \| .*(Success|Failure)|Note: harnesses marked \"\(ctor\)\"' | tr -s ' ' | sort
