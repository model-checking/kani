// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z quantifiers

//! FIXME: <https://github.com/model-checking/kani/issues/4020>

use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<u32>(); 3];
    let v = original_v.clone();
    let v_len = v.len();
    let v_ptr = v.as_ptr();
    unsafe {
        kani::assume(
            kani::forall!(|i in (0,v_len) | *v_ptr.wrapping_byte_offset(4*i as isize) < 5),
        );
    }

    // Prevent running `v`'s destructor so we are in complete control
    // of the allocation.
    let mut v = mem::ManuallyDrop::new(v);

    // Pull out the various important pieces of information about `v`
    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    unsafe {
        // Overwrite memory
        for i in 0..len {
            *p.add(i) += 1;
            if i == 1 {
                *p.add(i) = 0;
            }
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        let rebuilt_ptr = v.as_ptr();
        assert!(
            kani::exists!(| i in (0, len) | *rebuilt_ptr.wrapping_byte_offset(4*i as isize) == 0)
        );
    }
}
