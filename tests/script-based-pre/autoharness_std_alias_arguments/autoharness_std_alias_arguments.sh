#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# The model behind `--alias-arguments` must be available in the `verify-std` flow, which only
# links `kani_core`.
# 1. Make a copy of the rust standard library.
# 2. Inject `kani_lib!` and two functions that are only correct for non-aliasing arguments
#    into `core`.
# 3. Verify the automatic harnesses for those functions, without and with `--alias-arguments`.

set +e

TMP_DIR="tmp_dir"

rm -rf ${TMP_DIR}
mkdir ${TMP_DIR}

echo "[TEST] Copy standard library from the current toolchain"
SYSROOT=$(rustc --print sysroot)
STD_PATH="${SYSROOT}/lib/rustlib/src/rust/library"
cp -r "${STD_PATH}" "${TMP_DIR}"

echo "[TEST] Modify library"
cat verify_alias_arguments.rs >> ${TMP_DIR}/library/core/src/lib.rs

# Verify the injected functions, passing any arguments on to `kani autoharness`. The result rows
# are prefixed with the given label, since the rows of the two runs otherwise differ only in the
# verification result, and the tables are padded to the longest name in `core`, so squeeze the
# padding.
run_autoharness() {
    label="$1"
    shift
    # Capture the output first: piping the command straight into `grep` would hide its exit
    # status.
    output=$(kani autoharness -Z autoharness -Z unstable-options --output-format=regular "$@" \
        --std "${TMP_DIR}/library" \
        --target-dir "${TMP_DIR}/target" \
        --include-pattern "verify_alias_arguments::" 2>&1)
    echo "[${label}] exit status: $?"
    echo "${output}" | grep -E '^\| core +\| verify_alias_arguments::.*(Success|Failure)' \
        | tr -s ' ' | sort | sed "s/^/[${label}] /"
}

echo "[TEST] Run kani autoharness --std"
run_autoharness independent
echo "[TEST] Run kani autoharness --std --alias-arguments"
run_autoharness aliasing --alias-arguments

rm -r ${TMP_DIR}
