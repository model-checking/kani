#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test that kani-compiler sets its required rustc flags unconditionally.
#
# Case 1 (minimal): invoke kani-compiler directly with the MINIMAL flag set
# and assert the #[cfg(kani)]-gated harnesses appear in the metadata. Without
# the defaults, --cfg=kani would be unset, the harness module would be
# invisible, and the metadata would have 0 proof_harnesses: a vacuous
# "verification" with nothing verified.
#
# The single-token `--flag=value` encoding of the cfg defaults must be parsed
# AND applied, not just accepted:
# - --cfg=kani: harnesses appear at all — the whole module is gated on it.
# - --check-cfg=cfg(kani): fixture.rs also carries a deliberately undeclared
#   cfg. rustc only checks cfg names when at least one --check-cfg argument
#   is present, and none is passed below — so that cfg drawing an
#   unexpected_cfgs warning proves the default was parsed as a check-cfg
#   spec, and #[cfg(kani)] drawing none proves `kani` is the name it
#   registered.
#
# Case 2 (conflict): the caller explicitly passes the opposite of three
# required flags and the required values must still win — see the run_case
# call below for how each winner is observed.

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

# Args every case shares. Deliberately omits every flag
# KANI_REQUIRED_RUSTC_ARGS now defaults: -Cpanic=abort, -Coverflow-checks=on,
# -Csymbol-mangling-version, -Zalways-encode-mir, -Zpanic_abort_tests,
# -Zmir-enable-passes, --cfg=kani, --check-cfg=cfg(kani). What remains is
# what every caller still has to pass: install paths, the routing marker,
# -Zunstable-options (gates `--extern noprelude:`), -Zcrate-attr (errors on
# duplicate so it stays caller-supplied), and the reachability intent.
COMMON_ARGS=(
    --kani-compiler
    --crate-type lib
    --sysroot "${KANI_HOME}"
    -L "${KANI_LIB}"
    -Z unstable-options
    --extern kani
    --extern noprelude:std="${KANI_LIB}/libstd.rlib"
    -Z crate-attr="feature(register_tool)"
    -Z crate-attr="register_tool(kanitool)"
    -Cllvm-args=--reachability=harnesses
)

# The case's metadata must list all of fixture.rs's harnesses (the count
# observes which cfgs resolved — see fixture.rs) with v0-mangled
# (`_R`-prefixed; legacy is `_ZN`) symbol names.
check_metadata() {
    local label="$1"
    local metas=("${OUTDIR}/${label}"/*.kani-metadata.json)
    [[ ${#metas[@]} -eq 1 && -f "${metas[0]}" ]] || {
        echo "ERROR: ${label}: expected exactly one metadata file, got: ${metas[*]}"
        exit 1
    }
    local count
    count=$(jq '.proof_harnesses | length' "${metas[0]}")
    echo "[TEST] ${label} proof_harnesses: ${count}"
    if ! jq -e '(.proof_harnesses | length) > 0 and all(.proof_harnesses[]; .mangled_name | startswith("_R"))' \
        "${metas[0]}" > /dev/null; then
        echo "ERROR: ${label}: missing or non-v0 mangled harness names:"
        jq -r '.proof_harnesses[].mangled_name' "${metas[0]}"
        exit 1
    fi
    echo "[TEST] ${label} mangled names are v0"
}

# Compile fixture.rs as one test case: `label` names the case (and the
# crate), remaining args are the caller flags under test. kani-compiler
# appends KANI_REQUIRED_RUSTC_ARGS after everything passed here.
run_case() {
    local label="$1"
    shift
    mkdir "${OUTDIR}/${label}"
    "${KANI_COMPILER}" "${COMMON_ARGS[@]}" \
        --crate-name "${label}" \
        --out-dir "${OUTDIR}/${label}" \
        "$@" \
        fixture.rs 2> "${OUTDIR}/${label}.stderr" || {
        cat "${OUTDIR}/${label}.stderr"
        exit 1
    }
    echo "[TEST] ${label}: kani-compiler exited 0"
    check_metadata "${label}"
}

# Case 1: minimal invocation — every required flag missing; the defaults
# must supply them all.
run_case minimal

# Negative control: fixture.rs's undeclared cfg must draw unexpected_cfgs.
# Should rustc ever start checking cfg names without any --check-cfg being
# passed, this leg turns vacuous — but the absence leg below still catches a
# dropped or mis-encoded default, because #[cfg(kani)] would then warn.
WARN='unexpected `cfg` condition name'
if ! grep -q "${WARN}: \`not_a_kani_cfg\`" "${OUTDIR}/minimal.stderr"; then
    echo 'ERROR: undeclared cfg drew no unexpected_cfgs warning — cfg checking is not active'
    cat "${OUTDIR}/minimal.stderr"
    exit 1
fi
echo "[TEST] unexpected_cfgs fired for undeclared cfg"

# ...and `kani` must be the name --check-cfg=cfg(kani) registered: with cfg
# checking proven active, #[cfg(kani)] must not warn.
if grep -q "${WARN}: \`kani\`" "${OUTDIR}/minimal.stderr"; then
    echo 'ERROR: unexpected_cfgs fired for cfg(kani) — --check-cfg=cfg(kani) did not register `kani`'
    cat "${OUTDIR}/minimal.stderr"
    exit 1
fi
echo "[TEST] no unexpected_cfgs warning for cfg(kani)"

# Case 2: the caller explicitly passes conflicting values and the required
# values must still win (today: defaults are appended after caller args and
# rustc is last-flag-wins for scalar -C/-Z flags; these assertions pin the
# outcome, not the mechanism). Each winner is observable without CBMC:
# - -Cpanic=unwind or -Coverflow-checks=off winning is a hard error: kani-
#   compiler's own session gates refuse to run ("Kani can only handle abort
#   panic strategy", "Kani requires overflow checks"), so exiting 0 proves
#   the required values resolved. The #[cfg(panic = "abort")]-gated harness
#   doubles as a count backstop should those gates ever move.
# - -Csymbol-mangling-version=legacy winning flips mangled names off v0's
#   `_R` prefix, failing check_metadata.
run_case conflict -Cpanic=unwind -Coverflow-checks=off -Csymbol-mangling-version=legacy

rm -rf "${OUTDIR}"
