//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::hash`.

use std::alloc::*;
use std::cmp;
use std::ops::Range;
use std::usize;

use strategy::*;
use strategy::statics::static_map;
use arbitrary::*;

arbitrary!(CannotReallocInPlace; CannotReallocInPlace);
arbitrary!(Global; Global);

// Not Debug.
//lazy_just!(System, || System);

arbitrary!(Layout, SFnPtrMap<(Range<u8>, StrategyFor<usize>), Self>;
    // 1. align must be a power of two and <= (1 << 31):
    // 2. "when rounded up to the nearest multiple of align, must not overflow".
    static_map((0u8..32u8, any::<usize>()), |(align_power, size)| {
        let align = 1usize << align_power;
        let max_size = 0usize.wrapping_sub(align);
        // Not quite a uniform distribution due to clamping,
        // but probably good enough
        Layout::from_size_align(cmp::min(max_size, size), align).unwrap()
    })
);

arbitrary!(AllocErr, Just<Self>; Just(AllocErr));
arbitrary!(CollectionAllocErr, TupleUnion<(W<Just<Self>>, W<Just<Self>>)>;
           prop_oneof![Just(CollectionAllocErr::AllocErr),
                       Just(CollectionAllocErr::CapacityOverflow)]);

#[cfg(test)]
mod test {
    no_panic_test!(
        layout => Layout,
        alloc_err => AllocErr,
        collection_alloc_err => CollectionAllocErr
    );
}
