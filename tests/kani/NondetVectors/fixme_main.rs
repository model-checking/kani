// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
const FIFO_SIZE: usize = 2;
#[kani::proof]
fn main() {
    let len: usize = kani::any();
    if !(len <= FIFO_SIZE) {
        return;
    }
    let _buf1: Vec<u8> = vec![0u8; len]; //< this works
    let elt: u8 = kani::any();
    let _buf2: Vec<u8> = vec![elt; len]; //< this fails: "memset destination region writeable"
    let idx: usize = kani::any();
    if idx < len {
        assert!(_buf1[idx] == 0u8);
        assert!(_buf2[idx] == elt);
    }
}
