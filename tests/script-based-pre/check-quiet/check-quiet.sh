#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Checks that no output is produced if `--quiet` is used

set -eu

KANI_OUTPUT=`kani assume.rs --quiet | wc -l`

if [[ ${KANI_OUTPUT} -ne 0 ]]; then
    echo "error: \`--quiet\` produced some output"
    exit 1
else
    echo "success: \`--quiet\` produced NO output"
fi
