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
#
# Also prove the single-token `--flag=value` encoding of the cfg defaults is
# parsed AND applied, not just accepted:
# - --cfg=kani: proof_harnesses is 1, which requires cfg(kani) to be set.
# - --check-cfg=cfg(kani): fixture.rs also carries a deliberately undeclared
#   cfg. rustc only checks cfg names when at least one --check-cfg argument
#   is present, and none is passed below — so that cfg drawing an
#   unexpected_cfgs warning proves the default was parsed as a check-cfg
#   spec, and #[cfg(kani)] drawing none proves `kani` is the name it
#   registered.

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
    fixture.rs 2> "${OUTDIR}/fixture.stderr" || {
    cat "${OUTDIR}/fixture.stderr"
    exit 1
}

echo "[TEST] kani-compiler exited 0"

# The metadata must list the #[cfg(kani)]-gated harness — proves --cfg=kani
# was set internally.
META=$(ls "${OUTDIR}"/*.kani-metadata.json)
HARNESSES=$(jq '.proof_harnesses | length' "${META}")
echo "[TEST] proof_harnesses: ${HARNESSES}"

# Negative control: fixture.rs's undeclared cfg must draw unexpected_cfgs.
# Should rustc ever start checking cfg names without any --check-cfg being
# passed, this leg turns vacuous — but the absence leg below still catches a
# dropped or mis-encoded default, because #[cfg(kani)] would then warn.
WARN='unexpected `cfg` condition name'
if ! grep -q "${WARN}: \`not_a_kani_cfg\`" "${OUTDIR}/fixture.stderr"; then
    echo 'ERROR: undeclared cfg drew no unexpected_cfgs warning — cfg checking is not active'
    cat "${OUTDIR}/fixture.stderr"
    exit 1
fi
echo "[TEST] unexpected_cfgs fired for undeclared cfg"

# ...and `kani` must be the name --check-cfg=cfg(kani) registered: with cfg
# checking proven active, #[cfg(kani)] must not warn.
if grep -q "${WARN}: \`kani\`" "${OUTDIR}/fixture.stderr"; then
    echo 'ERROR: unexpected_cfgs fired for cfg(kani) — --check-cfg=cfg(kani) did not register `kani`'
    cat "${OUTDIR}/fixture.stderr"
    exit 1
fi
echo "[TEST] no unexpected_cfgs warning for cfg(kani)"

rm -rf "${OUTDIR}"
