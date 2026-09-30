#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Verify for `riscv64gc-unknown-linux-gnu` from whatever host this runs on, through both `kani`
# and `cargo kani`. Needs Kani's libraries for that target, which `scripts/kani-regression.sh`
# builds with `cargo build-dev --lib-target riscv64gc-unknown-linux-gnu`.

set +e
TARGET=riscv64gc-unknown-linux-gnu

echo "[TEST] --target needs -Z unstable-options"
kani riscv64.rs --target $TARGET

echo "[TEST] single file on the host: the riscv64 harnesses are compiled out"
kani riscv64.rs

echo "[TEST] single file for riscv64"
kani riscv64.rs --target $TARGET -Z unstable-options

echo "[TEST] cargo kani for riscv64"
pushd sample_crate > /dev/null
cargo kani --target $TARGET -Z unstable-options -Z stubbing
cargo clean
popd > /dev/null

echo "[TEST] --concrete-playback is refused for another target"
kani riscv64.rs --target $TARGET -Z unstable-options -Z concrete-playback --concrete-playback print

echo "[TEST] verify-std is refused with --target"
kani verify-std . --target $TARGET -Z unstable-options

# The rejections above exit non-zero on purpose; the suite passes when the transcript matches.
exit 0
