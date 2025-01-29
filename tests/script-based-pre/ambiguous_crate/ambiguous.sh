#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Test how Kani handle ambiguous crate names.

rm -rf target
set -e
cargo kani --output-format terse && echo "No package is needed"
cargo kani -p zerocopy@0.0.1 --output-format terse && echo "But passing the correct package works"

# These next commands should fail so disable failures
set +e
cargo kani -p zerocopy || echo "Found expected ambiguous crate error"
cargo kani -p zerocopy@0.8.4 || echo "Found expected out of workspace error"

rm -rf target
