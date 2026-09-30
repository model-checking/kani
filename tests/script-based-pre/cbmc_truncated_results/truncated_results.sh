#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that Kani does not report success when CBMC fails while writing its results and leaves a
# truncated result array behind (https://github.com/model-checking/kani/issues/4905). A real
# out-of-memory failure is hard to trigger reliably, so `bin/cbmc` simulates one.
set -o nounset

REAL_CBMC=$(command -v cbmc)
export REAL_CBMC
export PATH=$(pwd)/bin:$PATH

kani truncated_results.rs
