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

    /// Test generated for harness `verify::index_array_65`
    ///
    /// Check for `assertion`: "index out of bounds: the length is less than or equal to the given index"

    #[test]
    fn kani_concrete_playback_index_array_65_7727508567333384671() {
        let concrete_vals: Vec<Vec<u8>> = vec![
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 65535
        vec![255, 255],
        // 18446744073709551615ul
        vec![255, 255, 255, 255, 255, 255, 255, 255],
    ];
    kani::concrete_playback_run(concrete_vals, index_array_65);
}
}
