// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks
// cbmc-flags: --unwind 2

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [_RNvXs3_NtNtCs9Odk7Lrvgnw_4core3str4iterNtB5_11CharIndicesNtNtNtNtB9_4iter6traits8iterator8Iterator4nextCs21hi0yVfW1J_4main.overflow.1] line 141 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&var_4 + 0)) - 1114112: FAILURE
// [_RNvXs5_NtNtCs9Odk7Lrvgnw_4core3str7patternINtB5_19MultiCharEqSearcherNtB7_12IsWhitespaceENtB5_8Searcher4nextCs21hi0yVfW1J_4main.overflow.1] line 641 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&var_5 + 8)) - 1114112: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

fn main() {
    let mut iter = "A few words".split_whitespace();
    match iter.next() {
        None => assert!(false),
        Some(x) => assert!(x == "A"),
    }
}
