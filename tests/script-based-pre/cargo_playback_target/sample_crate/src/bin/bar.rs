// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test is used to ensure we can select binary targets for concrete playback.

fn main() {
    // do nothing
}

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn cover_bar() {
        kani::cover!(kani::any::<u8>() == 0u8);
    }

    #[test]
    fn kani_concrete_playback_cover_bar_1234() {
        let concrete_vals: Vec<Vec<u8>> = vec![
            // 10
            vec![0],
        ];
        kani::concrete_playback_run(concrete_vals, cover_bar);
    }
}
