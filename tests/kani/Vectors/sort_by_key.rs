// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct GuestAddress(pub u64);

#[derive(Debug)]
pub struct GuestRegionMmap {
    guest_base: GuestAddress,
}

#[kani::proof]
#[kani::unwind(3)]
fn main() {
    let r = GuestRegionMmap { guest_base: GuestAddress(0) };
    let mut regions: Vec<GuestRegionMmap> = vec![];
    regions.push(r);
    regions.sort_by_key(|x| x.guest_base);
    assert!(regions[0].guest_base == GuestAddress(0));
}
