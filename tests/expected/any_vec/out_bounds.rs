// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test for #2759: Kani does not flag out-of-bounds dereference with `kani::vec::any_vec`
//! <https://github.com/model-checking/kani/issues/2759>
extern crate kani;
use kani::cover;

#[kani::proof]
#[kani::unwind(22)]
fn check_always_out_bounds() {
    let data = kani::vec::any_vec::<u8, 8>();

    // Capacity must match length.
    assert_eq!(data.capacity(), data.len());

    // Create invalid reference.
    let invalid = unsafe { data.get_unchecked(data.len()) };

    macro_rules! cover_len {
        ($fn_name:tt, $val:literal) => {
            fn $fn_name(val: &u8) {
                cover!(*val == 0);
            }

            if data.len() == $val {
                $fn_name(invalid);
            }
        };
    }

    // Ensure any length between 0..=8 can trigger a failure.
    cover_len!(check_0, 0);
    cover_len!(check_1, 1);
    cover_len!(check_2, 2);
    cover_len!(check_3, 3);
    cover_len!(check_4, 4);
    cover_len!(check_5, 5);
    cover_len!(check_6, 6);
    cover_len!(check_7, 7);
    cover_len!(check_8, 8);

    // This shouldn't be covered.
    cover_len!(check_9, 9);
}
