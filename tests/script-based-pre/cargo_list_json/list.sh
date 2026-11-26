#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that the json file produced by `kani list` is correct.
# Note that the list.expected file omits the value for "kani-version"
# to avoid having to update the test every time we bump versions.

output=$(cargo kani list -Z function-contracts --format json)

# Check that Kani prints the absolute path to target/kani/kani-list.json
list_file="target/kani/kani-list.json"
absolute_path="$(cd "$(dirname "$list_file")" && pwd -P && cd - > /dev/null)/$(basename "$list_file")"
expected_last_line="Wrote list results to $absolute_path"
last_line=$(echo "$output" | tail -n 1)

if [ "$last_line" = "$expected_last_line" ]; then
    cat "$list_file"
    exit 0
else
    echo "Test failed: Absolute path to kani-list.json is missing from printed output"
    exit 1
fi
