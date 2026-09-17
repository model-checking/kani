#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test that cargo kani --workspace --features works correctly when some
# workspace members don't declare the requested features.
# This is a regression test for the issue where cargo kani fails with
# "none of the selected packages contains these features" when using
# --workspace --features with a workspace containing packages that don't
# all have the same features.

set -e

rm -rf target

# Test with a feature that only exists in mytests, not in mylib
cargo kani --workspace --features use_mylib

# Test with --no-default-features flag
cargo kani --workspace --no-default-features

# Test combination of flags
cargo kani --workspace --no-default-features --features use_mylib

rm -rf target
echo "All workspace feature tests passed"
