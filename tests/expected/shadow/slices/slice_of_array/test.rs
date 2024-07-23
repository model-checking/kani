// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// This test demonstrates a possible usage of the shadow memory API to check that
// every element of an arbitrary slice of an array is initialized.
// Since the instrumentation is done manually in the harness only but not inside
// the library functions, the test only verifies that the slices point to memory
// that is within the original array.

const N: usize = 16;

static mut SM: kani::shadow::ShadowMem<bool> = kani::shadow::ShadowMem::new(false);

#[kani::proof]
#[kani::unwind(17)]
fn check_slice_init() {
    let arr: [char; N] = kani::any();
    // tag every element of the array as initialized
    for i in &arr {
        unsafe {
            SM.set(i as *const char, true);
        }
    }
    // create an arbitrary slice of the array
    let end: usize = kani::any_where(|x| *x <= N);
    let begin: usize = kani::any_where(|x| *x < end);
    let slice = &arr[begin..end];

    // verify that all elements of the slice are initialized
    for i in slice {
        assert!(unsafe { SM.get(i as *const char) });
    }
}
