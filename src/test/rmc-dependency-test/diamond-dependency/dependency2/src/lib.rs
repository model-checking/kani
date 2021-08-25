// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use dependency3;

pub fn delegate_get_int() -> i32 {
    dependency3::get_int()
}

pub fn delegate_use_struct() -> i32 {
    let foo = dependency3::give_foo();
    dependency3::take_foo(&foo)
}
