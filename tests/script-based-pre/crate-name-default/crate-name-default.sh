#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

check_file_exists() {
    local file=$1
    if ! [ -e "${file}" ]
    then
        echo "error: expected \`${file}\` to have been generated"
        exit 1
    fi
}

kani --only-codegen --keep-temps a/src/lib.rs
check_file_exists a/src/liblib.rlib
check_file_exists a/src/lib.kani-metadata.json
rm a/src/liblib.rlib
rm a/src/lib.kani-metadata.json

kani --only-codegen --keep-temps my-code.rs
check_file_exists libmy_code.rlib
check_file_exists my_code.kani-metadata.json

RUSTFLAGS="--edition 2021" kani --only-codegen --keep-temps a/src/lib.rs --crate-name="a"
check_file_exists a/src/liba.rlib
check_file_exists a/src/a.kani-metadata.json

RUSTFLAGS="--edition 2021 --extern a=a/src/liba.rlib" kani --only-codegen --keep-temps b/src/lib.rs --crate-name="b"
check_file_exists b/src/libb.rlib
check_file_exists b/src/b.kani-metadata.json

RUSTFLAGS="--edition 2021 --extern b=b/src/libb.rlib --extern a=a/src/liba.rlib" kani c/src/lib.rs

rm a/src/liba.rlib
rm a/src/a.kani-metadata.json
rm b/src/libb.rlib
rm b/src/b.kani-metadata.json
