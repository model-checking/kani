// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod first {
    #[kani::proof]
    fn check() {
        assert_eq!(1 + 1, 2);
    }
}

mod second {
    #[kani::proof]
    fn check() {
        assert_eq!(2 + 2, 4);
    }
}

#[kani::proof]
pub fn check() {
    assert_eq!(3 + 3, 6);
}
