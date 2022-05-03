#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Flags: -d: Dry-run. Instead of updating, error if there are non-formatted c files

set -o errexit
set -o pipefail
set -o nounset

# Default to inplace update
FLAGS="-i"

while getopts d flag
do
    case "${flag}" in
        d) FLAGS="--dry-run --Werror";;
    esac
done


SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
KANI_DIR=$SCRIPT_DIR/..

find $KANI_DIR/library/kani -name "*.c" | xargs clang-format $FLAGS
find $KANI_DIR/tests/kani -name "*.c" | xargs clang-format $FLAGS
find $KANI_DIR/tests/cargo-kani -name "*.c" | xargs clang-format $FLAGS

