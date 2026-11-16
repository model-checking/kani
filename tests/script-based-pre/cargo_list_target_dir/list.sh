#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that `cargo kani list` accepts `--target-dir` and builds into it.

rm -rf custom_target
cargo kani list --target-dir custom_target || exit 1

if [ -d custom_target ]; then
    echo "Build artifacts written to custom target dir"
else
    echo "Test failed: custom_target was not created"
    exit 1
fi
rm -rf custom_target
