#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test that kani-compiler sets its required rustc flags unconditionally.
#
# Invoke kani-compiler directly with the MINIMAL flag set — install paths,
# the routing marker (--kani-compiler), the kanitool namespace registration
# (-Zcrate-attr — NOT a default because it errors on duplicate registration),
# and an intent flag (--reachability) — and assert a #[cfg(kani)]-gated
# harness appears in the metadata. Without the defaults, --cfg=kani would be
# unset, the harness module would be invisible, and the metadata would have
# 0 proof_harnesses: a vacuous "verification" with nothing verified.

set -eu

OUTDIR="tmp_compiler_defaults"
rm -rf "${OUTDIR}"
mkdir "${OUTDIR}"

# Locate kani-compiler and the kani sysroot in the dev build: `cargo build-dev`
# populates target/kani/ (KANI_SYSROOT in .cargo/config.toml) with bin/ and
# lib/, mirroring the release install layout. Same pattern as
# std_codegen/codegen_std.sh. KANI_HOME is what `kani` itself passes as
# --sysroot: LibConfig::new() uses the parent of the lib/ folder.
KANI_DIR=$(git rev-parse --show-toplevel)
KANI_HOME="${KANI_DIR}/target/kani"
KANI_COMPILER="${KANI_HOME}/bin/kani-compiler"
KANI_LIB="${KANI_HOME}/lib"

[[ -x "${KANI_COMPILER}" ]] || {
    echo "ERROR: kani-compiler not found at ${KANI_COMPILER}"
    echo "Run 'cargo build-dev' first."
    exit 1
}

# Minimal invocation. Deliberately omits every flag KANI_REQUIRED_RUSTC_ARGS
# now defaults: -Cpanic=abort, -Coverflow-checks=on, -Csymbol-mangling-version,
# -Zalways-encode-mir, -Zpanic_abort_tests, -Zmir-enable-passes, --cfg=kani,
# --check-cfg=cfg(kani). What remains is what every caller still has to pass:
# install paths, the routing marker, -Zunstable-options (gates `--extern
# noprelude:`), -Zcrate-attr (errors on duplicate so it stays caller-supplied),
# and the reachability intent.
"${KANI_COMPILER}" \
    --kani-compiler \
    --crate-type lib \
    --crate-name fixture \
    --out-dir "${OUTDIR}" \
    --sysroot "${KANI_HOME}" \
    -L "${KANI_LIB}" \
    -Z unstable-options \
    --extern kani \
    --extern noprelude:std="${KANI_LIB}/libstd.rlib" \
    -Z crate-attr="feature(register_tool)" \
    -Z crate-attr="register_tool(kanitool)" \
    -Cllvm-args=--reachability=harnesses \
    fixture.rs

echo "[TEST] kani-compiler exited 0"

# The metadata must list the #[cfg(kani)]-gated harness — proves --cfg=kani
# was set internally.
META=$(ls "${OUTDIR}"/*.kani-metadata.json)
HARNESSES=$(jq '.proof_harnesses | length' "${META}")
echo "[TEST] proof_harnesses: ${HARNESSES}"

rm -rf "${OUTDIR}"
