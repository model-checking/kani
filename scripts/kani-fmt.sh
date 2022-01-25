#!/usr/bin/env bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Runs `rustfmt` in our source crates and tests.
# The arguments given to this script are passed to rustfmt.
set -o errexit
set -o pipefail
set -o nounset

# Run from the repository root folder
ROOT_FOLDER=$(git rev-parse --show-toplevel)
cd ${ROOT_FOLDER}

# Verify crates.
# TODO: We should be able to use workspace once we unfork from rustc.
# https://github.com/model-checking/kani/issues/719
CRATES=(
    "bookrunner"
    "cbmc"
    "compiletest"
    "kani-compiler"
    "kani"
    "kani_macros"
    "kani_queries"
    "kani_restrictions"
    "rustc_codegen_kani"
)

error=0
for crate in ${CRATES[@]}; do
    # Check all crates. Only fail at the end.
    cargo fmt "$@" -p ${crate} || error=1
done

# Check test source files.
TESTS=("tests/kani"
    "tests/prusti"
    "tests/smack"
    "tests/expected"
    "tests/cargo-kani"
    "docs/src/tutorial")

for suite in "${TESTS[@]}"; do
    # Find uses breakline to split between files. This ensures that we can
    # handle files with space in their path.
    set -f; IFS=$'\n'
    files=($(find "${suite}" -name "*.rs"))
    set +f; unset IFS
    rustfmt --unstable-features "$@" "${files[@]}" || error=1
done

exit $error
