// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness my_mod::harness -Z stubbing
//
//! This tests whether we take into account simple local use-as statements (`use
//! XXX as YYY;`) when resolving paths in `kani::stub` attributes.

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
    use self::inner_mod::magic_number42 as forty_two;
    use super::MyType as MyFavoriteType;
    use super::magic_number13 as thirteen;

    mod inner_mod {
        pub fn magic_number42() -> u32 {
            42
        }
    }

    #[kani::proof]
    #[kani::stub(zero, thirteen)]
    #[kani::stub(one, forty_two)]
    #[kani::stub(two, MyFavoriteType::magic_number101)]
    fn harness() {
        assert_eq!(zero(), thirteen());
        assert_eq!(one(), forty_two());
        assert_eq!(two(), MyFavoriteType::magic_number101());
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
