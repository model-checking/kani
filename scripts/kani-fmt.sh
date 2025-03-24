#!/usr/bin/env bash
# Copyright Kani Contributors
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
error=0

export RUSTC_BOOTSTRAP=1

# Check all crates. Only fail at the end.
cargo fmt "$@" || error=1

# Check test source files.
# Note that this will respect the ignore section of rustfmt.toml. If you need to
# skip any file / directory, add it there.
TESTS=("tests" "docs/src/tutorial")

for suite in "${TESTS[@]}"; do
    # Find uses breakline to split between files. This ensures that we can
    # handle files with space in their path.
    set -f; IFS=$'\n'
    files=($(find "${suite}" -name "*.rs" -not -path "*/perf/s2n-quic/*"))
    set +f; unset IFS
    # Note: We set the configuration file here because some submodules have
    # their own configuration file.
    rustfmt "$@" --config-path rustfmt.toml "${files[@]}" || error=1
done

exit $error
