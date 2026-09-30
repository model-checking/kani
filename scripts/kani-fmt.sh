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

# Parse arguments to check for --check flag
check_flag=""
for arg in "$@"; do
  if [ "$arg" = "--check" ]; then
    check_flag="--check"
    break
  fi
done

# Verify crates.
error=0

# Check all crates. Only fail at the end.
cargo fmt ${check_flag} || error=1

# Check test source files.
TESTS=("tests" "docs/src/tutorial")
# Add ignore patterns for code we don't want to format.
# `*/perf/s2n-quic/*` excludes the upstream submodule.
# `*/perf/overlays/*` excludes the overlay sources (see
# `tests/perf/overlays/README.md`): these are partial copies of submodule
# files staged for `cp -r` by `scripts/kani-perf.sh`, and they can reference
# `mod` declarations whose siblings only exist in the submodule, so rustfmt
# cannot standalone-parse them.
IGNORE=("*/perf/s2n-quic/*" "*/perf/overlays/*")

# Arguments for the find command for excluding the IGNORE paths
IGNORE_ARGS=()
for ignore in "${IGNORE[@]}"; do
    IGNORE_ARGS+=(-not -path "$ignore")
done

for suite in "${TESTS[@]}"; do
    # Find uses breakline to split between files. This ensures that we can
    # handle files with space in their path.
    set -f; IFS=$'\n'
    files=($(find "${suite}" -name "*.rs" ${IGNORE_ARGS[@]}))
    set +f; unset IFS
    # Note: We set the configuration file here because some submodules have
    # their own configuration file.
    rustfmt --config-path rustfmt.toml ${check_flag} "${files[@]}" || error=1
done

exit $error
