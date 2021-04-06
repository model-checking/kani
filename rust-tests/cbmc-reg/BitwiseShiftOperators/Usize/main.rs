// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn bitmap_new(byte_size: usize, page_size: usize) -> usize {
    let bit_size: usize = byte_size / page_size;
    let map_size = ((bit_size - 1) >> 6) + 1;
    map_size
}
fn main() {
    let map_size = bitmap_new(1024, 128);
    assert!(map_size == 1);
}
