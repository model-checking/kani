// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
const FIFO_SIZE: usize = 2;
include!("../../rmc-prelude.rs");

fn main() {
    let len: usize = __nondet();
    if !(len <= FIFO_SIZE) {
        return;
    }
    let _buf1: Vec<u8> = vec![0u8; len]; //< this works
    let elt: u8 = __nondet();
    let _buf2: Vec<u8> = vec![elt; len]; //< this fails: "memset destination region writeable"
    let idx: usize = __nondet();
    if idx < len {
        assert!(_buf1[idx] == 0u8);
        assert!(_buf2[idx] == elt);
    }
}
