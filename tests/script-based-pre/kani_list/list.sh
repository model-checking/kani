#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that the JSON file produced by `kani list` is correct.
# Note that the list.expected file omits the value for "kani-version"
# to avoid having to update the test every time we bump versions.

kani list -Z list -Z function-contracts src/lib.rs --format json
cat "kani-list.json"
