// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn main() {
    let list = [1, 2, 3];
    let slice = &list[1..2];
    assert!(slice.len() > 0);
}
