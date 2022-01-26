// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can handle set len on drop. If drop_in_place is not
// called correctly, this will fail to actually extend the vector.

fn main() {
    let mut v: Vec<u32> = Vec::new();
    v.extend(42..=42);
    assert!(v[0] == 42);
}
