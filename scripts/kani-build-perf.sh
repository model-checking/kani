#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Run compiler benchmarks (only build time)

set -o pipefail
set -o nounset


SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
source ${SCRIPT_DIR}/kani-perf-setup.sh
benchmark_build
