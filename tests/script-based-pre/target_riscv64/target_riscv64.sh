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

echo "[TEST] the linked goto binary keeps the riscv64 machine model"
# CBMC takes its C semantics (char signedness, long double width, the C library it adds) from the
# __CPROVER_architecture_* symbols in the binary it checks, so inspect the binary Kani linked
# rather than what kani-compiler wrote.
TMP_DIR=$(mktemp -d)
cp riscv64.rs "$TMP_DIR"
kani "$TMP_DIR/riscv64.rs" --target $TARGET -Z unstable-options --harness target_is_riscv64gc \
    --keep-temps > /dev/null
for goto in "$TMP_DIR"/*target_is_riscv64gc.out; do
    goto-instrument --show-symbol-table "$goto" | awk '
        /^Symbol\.+: __CPROVER_architecture_(arch|char_is_unsigned)$/ { name = $2 }
        /^Value/ && name { sub(/^\(__CPROVER_integer\)/, "", $2); print name " = " $2; name = "" }'
done
rm -rf "$TMP_DIR"

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
