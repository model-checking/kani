// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: --harness my_mod::harness -Z stubbing
//
//! This tests whether we take into account names imported through use lists (`use a::{b, c}`)
//! when resolving paths in `kani::stub` attributes, including renames, globs and nested lists
//! inside a list. Since nightly-2026-10-06, rustc keeps such a `use` as one HIR item with a tree
//! of imports, instead of lowering it to one item per imported name.

fn magic_number13() -> u32 {
    13
}

struct MyType {}

impl MyType {
    fn magic_number101() -> u32 {
        101
    }
}

mod numbers {
    pub fn magic_number7() -> u32 {
        7
    }

    pub mod deep {
        pub fn magic_number9() -> u32 {
            9
        }
    }

    pub mod globbed {
        pub fn magic_number5() -> u32 {
            5
        }
    }
}

mod my_mod {
    use self::inner_mod::{
        magic_number42,
        nested::{magic_number21, magic_number22},
    };
    use super::numbers::{deep::magic_number9, globbed::*, magic_number7 as seven};
    use super::{MyType, magic_number13};

    mod inner_mod {
        pub fn magic_number42() -> u32 {
            42
        }

        pub mod nested {
            pub fn magic_number21() -> u32 {
                21
            }

            pub fn magic_number22() -> u32 {
                22
            }
        }
    }

    #[kani::proof]
    #[kani::stub(zero, magic_number13)]
    #[kani::stub(one, magic_number42)]
    #[kani::stub(two, MyType::magic_number101)]
    #[kani::stub(three, seven)]
    #[kani::stub(four, magic_number9)]
    #[kani::stub(five, magic_number5)]
    #[kani::stub(six, magic_number21)]
    #[kani::stub(seventh, magic_number22)]
    fn harness() {
        assert_eq!(zero(), magic_number13());
        assert_eq!(one(), magic_number42());
        assert_eq!(two(), MyType::magic_number101());
        assert_eq!(three(), seven());
        assert_eq!(four(), magic_number9());
        assert_eq!(five(), magic_number5());
        assert_eq!(six(), magic_number21());
        assert_eq!(seventh(), magic_number22());
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

    fn three() -> u32 {
        3
    }

    fn four() -> u32 {
        4
    }

    fn five() -> u32 {
        5
    }

    fn six() -> u32 {
        6
    }

    fn seventh() -> u32 {
        7
    }
}
