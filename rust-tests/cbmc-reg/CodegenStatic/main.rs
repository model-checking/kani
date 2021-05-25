// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
static STATIC: [&str; 1] = ["FOO"];
fn main() {
    let _x = STATIC[0];
    assert!(_x.as_bytes()[0] == b'F');
}
