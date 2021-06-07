// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn move_(n: i32, from: i32, to: i32, via: i32) {
    if n > 0 {
        move_(n - 1, from, via, to);
        move_(n - 1, via, to, from);
    }
}

fn main() {
    move_(4, 1, 2, 3);
}
