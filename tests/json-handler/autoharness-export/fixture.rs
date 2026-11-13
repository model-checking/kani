// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[derive(kani::BoundedArbitrary)]
pub struct Packet {
    #[bounded]
    payload: Vec<u8>,
    flag: bool,
}

pub fn packet_len(p: Packet) -> usize {
    if p.flag { p.payload.len() } else { 0 }
}

pub fn first_byte(bytes: &[u8]) -> u8 {
    if bytes.is_empty() { 0 } else { bytes[0] }
}

pub fn add_one(x: u8) -> u8 {
    x.wrapping_add(1)
}

#[kani::proof]
fn manual_harness() {
    let x: u8 = kani::any();
    assert!(add_one(x) == x.wrapping_add(1));
}
