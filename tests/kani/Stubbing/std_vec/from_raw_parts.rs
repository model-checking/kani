// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
use std::ptr;

fn main() {
    fn from_raw_parts_test() {
        let v = kani_vec![1, 2, 3];

        // Prevent running `v`'s destructor so we are in complete control
        // of the allocation.
        let mut v = mem::ManuallyDrop::new(v);

        // Pull out the various important pieces of information about `v`
        let p = v.as_mut_ptr();
        let len = v.len();
        let cap = v.capacity();

        unsafe {
            // Overwrite memory with 4, 5, 6
            for i in 0..len as isize {
                ptr::write(p.offset(i), 4 + i);
            }

            // Put everything back together into a Vec
            let rebuilt = Vec::from_raw_parts(p, len, cap);
            assert_eq!(rebuilt, [4, 5, 6]);
        }
    }
}
