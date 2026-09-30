// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn check_add() {
        let a: u8 = kani::any();
        kani::assume(a < 10);
        assert!(a + 1 <= 10);
    }
}
