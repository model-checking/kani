// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::ptr::read;

fn main() {
    let var = 1;
    unsafe {
        assert_eq!(read(&var), var);
    }
}
