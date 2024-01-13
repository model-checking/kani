// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --use-abs --abs-type kani
fn main() {
    fn as_mut_ptr_test() {
        let size = 4;
        let mut x: Vec<i32> = Vec::with_capacity(size);
        let x_ptr = x.as_mut_ptr();

        // Initialize elements via raw pointer writes, then set length.
        unsafe {
            for i in 0..size {
                *x_ptr.add(i) = i as i32;
            }
            x.set_len(size);
        }
        assert_eq!(&*x, &[0, 1, 2, 3]);
    }

    as_mut_ptr_test();
}
