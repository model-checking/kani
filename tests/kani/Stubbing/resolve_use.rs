// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness my_mod::harness -Z stubbing
//
//! This tests whether we take into account simple local uses (`use XXX;`) when
//! resolving paths in `kani::stub` attributes.

fn magic_number13() -> u32 {
    13
}

struct MyType {}

impl MyType {
    fn magic_number101() -> u32 {
        101
    }
}

mod my_mod {
    use self::inner_mod::magic_number42;
    use super::MyType;
    use super::magic_number13;

    mod inner_mod {
        pub fn magic_number42() -> u32 {
            42
        }
    }

    #[kani::proof]
    #[kani::stub(zero, magic_number13)]
    #[kani::stub(one, magic_number42)]
    #[kani::stub(two, MyType::magic_number101)]
    fn harness() {
        assert_eq!(zero(), magic_number13());
        assert_eq!(one(), magic_number42());
        assert_eq!(two(), MyType::magic_number101());
    }

    fn zero() -> u32 {
        0
    }

    fn one() -> u32 {
        1
    }

    fn two() -> u32 {
        2
    }
}
