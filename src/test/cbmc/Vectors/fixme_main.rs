// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct GuestAddress(pub u64);

#[derive(Debug)]
pub struct GuestRegionMmap {
    guest_base: GuestAddress,
}

// TODO: running this with --unwrap 2 causes CBMC to hang in propositional reduction.
fn main() {
    let r = GuestRegionMmap { guest_base: GuestAddress(0) };
    let mut regions: Vec<GuestRegionMmap> = vec![];
    regions.push(r);
    regions.sort_by_key(|x| x.guest_base);
    assert!(regions[0].guest_base == GuestAddress(0));
}
