// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-flags: --function foo
#![feature(never_type)]

/// Test using the never type
#[no_mangle]
pub fn foo(never: !) -> i32 {
    return 1
}
