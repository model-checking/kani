#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test `kani verify-artifacts`: build artifacts with `cargo kani --only-codegen`,
# then verify them with `kani verify-artifacts`. Round-trips the build/verify
# split that build systems with their own caching (nix, bazel, sccache, CI
# sharding) need.

set -eu

TARGET=$(rustc -vV | awk '/^host/ { print $2 }')
ARTIFACTS="target/kani/${TARGET}/debug/deps"

# Clean up any prior run.
rm -rf target

# Phase 1: produce artifacts only.
echo "[TEST] Build artifacts with --only-codegen"
cargo kani --only-codegen

# Sanity: the artifacts the verify phase needs are present.
echo "[TEST] Artifacts produced"
ls "${ARTIFACTS}"/*.kani-metadata.json > /dev/null
ls "${ARTIFACTS}"/*.symtab.out > /dev/null

# Phase 2: verify the pre-built artifacts. One harness passes, one fails by
# design — the exit code 1 proves the pipeline reached CBMC and propagated the
# verdict, not merely that the command ran.
echo "[TEST] Verify pre-built artifacts"
EXIT=0
kani verify-artifacts "${ARTIFACTS}" -Z unstable-options || EXIT=$?

echo "[TEST] Exit code: ${EXIT}"

# Clean up.
rm -rf target
