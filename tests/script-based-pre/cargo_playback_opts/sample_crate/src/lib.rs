// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that running the unit test generated using the concrete playback feature
// with `RUSTFLAGS="--cfg=kani" cargo +nightly test doesn't cause a compilation error.
// There is an existing UI test to generate the unit test itself (in kani/tests/ui/concrete-playback/result).

#[cfg(kani)]
mod harnesses {
    #[kani::proof]
    fn harness() {
        let result_1: Result<u8, u8> = kani::any();
        let result_2: Result<u8, u8> = kani::any();
        assert!(!(result_1 == Ok(101) && result_2 == Err(102)));
    }

    #[test]
    fn kani_concrete_playback_harness_15598097466099501582() {
        let concrete_vals: Vec<Vec<u8>> = vec![
            // 1
            vec![1],
            // 101
            vec![101],
            // 0
            vec![0],
            // 102
            vec![102],
        ];
        kani::concrete_playback_run(concrete_vals, harness);
    }
}
