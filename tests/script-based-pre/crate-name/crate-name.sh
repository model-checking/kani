#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# This test performs multiple checks focused on crate names. The first steps
# check expected results with the default naming scheme. The remaining ones
# check expected results with the `--crate-name=<name>` feature which allows
# users to specify the crate name used for compilation with standalone `kani`.
set -eu

check_file_exists() {
    local file=$1
    if ! [ -e "${file}" ]
    then
        echo "error: expected \`${file}\` to have been generated"
        exit 1
    fi
}

# 1. Check expected results with the default naming scheme.
# Note: The assumed crate name is `lib`, so we generate `liblib.rlib`.
kani --only-codegen --keep-temps a/src/lib.rs
check_file_exists a/src/liblib.rlib
check_file_exists a/src/lib.kani-metadata.json
rm a/src/liblib.rlib
rm a/src/lib.kani-metadata.json

# 2. Check expected results with the default naming scheme, which replaces
#    some characters.
# Note: The assumed crate name is `my-code`, so we generate `libmy_code.rlib`.
kani --only-codegen --keep-temps my-code.rs
check_file_exists libmy_code.rlib
check_file_exists my_code.kani-metadata.json

# 3. Check expected results with the `--crate-name=<name>` feature. This feature
#    allows users to specify the crate name used for compilation with standalone
#    `kani`, enabling the compilation of multiple dependencies with similar
#    names.
# Note: In the example below, compiling without `--crate-name=<name>` would
# result in files named `liblib.rlib` for each dependency.
kani --only-codegen --keep-temps a/src/lib.rs --crate-name="a"
check_file_exists a/src/liba.rlib
check_file_exists a/src/a.kani-metadata.json

RUSTFLAGS="--extern a=a/src/liba.rlib" kani --only-codegen --keep-temps b/src/lib.rs --crate-name="b"
check_file_exists b/src/libb.rlib
check_file_exists b/src/b.kani-metadata.json

RUSTFLAGS="--extern b=b/src/libb.rlib --extern a=a/src/liba.rlib" kani c/src/lib.rs

rm a/src/liba.rlib
rm a/src/a.kani-metadata.json
rm b/src/libb.rlib
rm b/src/b.kani-metadata.json
