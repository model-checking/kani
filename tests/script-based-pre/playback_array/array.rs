// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test that concrete playback generates concrete values for arrays over the length of 64
//! and that playback can run those tests and find the index out of bounds bug,
//! c.f. https://github.com/model-checking/kani/issues/3787

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn index_array_65() {
        let arr: [u16; 65] = kani::any();
        let idx: usize = kani::any();
        arr[idx];
    }
}
