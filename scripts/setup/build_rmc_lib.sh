#!/bin/bash
# Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -o errexit
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
SCRIPTS_DIR="$(dirname $SCRIPT_DIR)"
REPO_DIR="$(dirname $SCRIPTS_DIR)"

export RUSTC=$(${SCRIPTS_DIR}/rmc-rustc --rmc-path)
export RUSTFLAGS="--cfg rmc"
cargo clean --manifest-path "${REPO_DIR}/library/rmc/Cargo.toml" $@
cargo build --manifest-path "${REPO_DIR}/library/rmc/Cargo.toml" $@

