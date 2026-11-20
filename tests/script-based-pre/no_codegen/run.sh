#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

rm -rf target

# Test the behavior of the `--no-codegen` option
cargo kani --no-codegen -Zunstable-options

# Ensure no goto binaries (*.out) are generated
[[ -z $(find target -name "*.out") ]] || {
    echo "ERROR: Found goto binaries (*.out) in target directory"
    exit 1
}
