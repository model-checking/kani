// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
static STATIC: [&str; 1] = ["FOO"];
fn main() {
    let x = STATIC[0];
    let bytes = x.as_bytes();
    assert!(bytes.len() == 3);
    assert!(bytes[0] == b'F');
    assert!(bytes[1] == b'O');
    assert!(bytes[2] == b'O');
}
