// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the result of using Iterator::flat_map. Note that the same test exists inside
//! kani suite. This test is just to ensure we error when there is a projection issue.

#[kani::proof]
#[kani::unwind(3)]
fn check_flat_map_char() {
    let hello = ["H", "i"];
    let length = hello.iter().flat_map(|s| s.chars()).count();
    assert_eq!(length, 2);
}
