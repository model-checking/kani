// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check the message printed when a checked operation fails.
fn main() {
    let v1: u8 = kani::any();
    let v2: u8 = kani::any();
    let _ = v1 + v2;
    let _ = v1 - v2;
    let _ = v1 * v2;
}
