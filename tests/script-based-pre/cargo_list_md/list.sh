#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that the MD file produced by `kani list` is correct.
# Note that the list.expected file omits the value for "kani-version"
# to avoid having to update the test every time we bump versions.

output=$(cargo kani list -Z function-contracts --format markdown)

# Check that Kani prints the absolute path to kani-list.md
absolute_path="$(cd "$(dirname "kani-list.md")" && pwd -P && cd - > /dev/null)/$(basename "kani-list.md")"
expected_last_line="Wrote list results to $absolute_path"
last_line=$(echo "$output" | tail -n 1)

if [ "$last_line" = "$expected_last_line" ]; then
    cat kani-list.md
    exit 0
else
    echo "Test failed: Absolute path to kani-list.md is missing from printed output"
    exit 1
fi
