#!/bin/bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# This file checks that the proptest rlib and symtab are up to are up
# to date compared to the kani binary. If not, `cargo kani` is called
# on proptest to refresh the symtab and rlib. This script is not meant
# to called manually, but rather from `scripts/kani` and
# `scripts/cargo-kani`.

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
KANI_REPO_ROOT="$SCRIPT_DIR/.."

PROPTEST_SYMTAB_PATH="$(find $KANI_REPO_ROOT/target -name '*symtab.json' | head -1)"
KANI_BINARY_PATH="$KANI_REPO_ROOT/debug/kani-compiler"

if [ ! -f "$PROPTEST_SYMTAB_PATH" ] || [[ "$PROPTEST_SYMTAB_PATH" -ot "$KANI_BINARY_PATH" ]]; then
    echo 'Proptest symtab not found or too old. (Re)compiling proptest..'
    (
        cd $KANI_REPO_ROOT/library/proptest;
        cargo kani --only-codegen;

        # TODO: not needed after workspace PR #1421 merges.
        cp -r ./target $KANI_REPO_ROOT;
        rm -rf ./target
    )
fi

# delete the normal rlib to avoid confusion.
rm $(find $KANI_REPO_ROOT/target/debug -name '*libproptest*.rlib') 2> /dev/null || true
