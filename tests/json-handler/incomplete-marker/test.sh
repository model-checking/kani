#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KANI_BIN="$(command -v kani)"
CBMC_BIN="$(command -v cbmc)"

python3 "$SCRIPT_DIR/run.py" "$KANI_BIN" "$CBMC_BIN" "$SCRIPT_DIR"
