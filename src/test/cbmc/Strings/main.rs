// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [_RINvMNtCs9Odk7Lrvgnw_4core6optionINtB3_6OptioncE5ok_orjECs21hi0yVfW1J_4main.overflow.1] line 569 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&self + 0)) - 1114112: FAILURE
// [_RNvXsG_NtCs9Odk7Lrvgnw_4core6optionINtB5_6OptioncENtNtB7_3cmp9PartialEq2eqCs21hi0yVfW1J_4main.overflow.1] line 160 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&(*var_4) + 0)) - 1114112: FAILURE
// [_RNvXsG_NtCs9Odk7Lrvgnw_4core6optionINtB5_6OptioncENtNtB7_3cmp9PartialEq2eqCs21hi0yVfW1J_4main.overflow.2] line 160 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&(*var_7) + 0)) - 1114112: FAILURE
// [_RNvXsG_NtCs9Odk7Lrvgnw_4core6optionINtB5_6OptioncENtNtB7_3cmp9PartialEq2eqCs21hi0yVfW1J_4main.overflow.3] line 160 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&(*var_13.0) + 0)) - 1114112: FAILURE
// [_RNvXsG_NtCs9Odk7Lrvgnw_4core6optionINtB5_6OptioncENtNtB7_3cmp9PartialEq2eqCs21hi0yVfW1J_4main.overflow.4] line 160 arithmetic overflow on unsigned - in *((unsigned int *)((unsigned char *)&(*var_13.1) + 0)) - 1114112: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

fn test1() {
    let str = "foo";
    let string = str.to_string();
    assert!(str.chars().nth(1) == Some('o'));
    assert!(string.chars().nth(1) == Some('o'));
    assert!(string.len() == 3);
}

fn main() {
    test1();
}
