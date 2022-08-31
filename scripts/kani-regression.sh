#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $KANI_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
KANI_DIR=$SCRIPT_DIR/..

# Formatting check
${SCRIPT_DIR}/kani-fmt.sh --check

# Build all packages in the workspace
cargo build --workspace

# Unit tests
cargo test -p proptest
# proptest-derive not supported.

# TODO: for parts where Kani is enabled, test with kani.

# Check that documentation compiles.
cargo doc --workspace --no-deps --exclude std

echo
echo "All Kani regression tests completed successfully."
echo
