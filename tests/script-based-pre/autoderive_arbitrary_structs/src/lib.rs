// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Test deriving Arbitrary for structs inside the compiler

#![allow(dead_code)]
#![allow(unused_variables)]

mod should_derive {
    #[derive(Eq, PartialEq)]
    pub struct UnitStruct;
    pub struct AnonStruct(u8);
    pub struct AnonMultipleStruct(u32, char);
    pub struct NamedStruct {
        val: u32,
    }
    pub struct NamedMultipleStruct {
        num: i128,
        char: char,
    }

    fn unit_struct(foo: UnitStruct, bar: UnitStruct) -> UnitStruct {
        assert_eq!(foo, bar);
        foo
    }

    fn anon_struct(foo: AnonStruct, divisor: u8) {
        if foo.0 % divisor != 0 {
            panic!("foo held an u32, but it didn't divide evenly");
        }
    }

    fn anon_multiple_struct(foo: AnonMultipleStruct, divisor: u32) {
        if foo.0 % divisor != 0 {
            panic!("foo held an u32, but it didn't divide evenly");
        }
    }

    fn named_struct(foo: NamedStruct, divisor: u32) {
        if foo.val % divisor != 0 {
            panic!("foo held an u32, but it didn't divide evenly");
        }
    }

    fn named_multiple(foo: NamedMultipleStruct, divisor: i128) {
        if foo.num % divisor != 0 {
            panic!("foo held an i28, but it didn't divide evenly");
        }
    }

    #[derive(Eq, PartialEq)]
    pub struct AlignmentStruct(usize);

    impl AlignmentStruct {
        const _ALIGN1SHL0: AlignmentStruct = AlignmentStruct(1 << 0);
        const _ALIGN1SHL1: AlignmentStruct = AlignmentStruct(1 << 1);
        const _ALIGN1SHL2: AlignmentStruct = AlignmentStruct(1 << 2);
        const _ALIGN1SHL3: AlignmentStruct = AlignmentStruct(1 << 3);
        const _ALIGN1SHL4: AlignmentStruct = AlignmentStruct(1 << 4);
        const _ALIGN1SHL5: AlignmentStruct = AlignmentStruct(1 << 5);
        const _ALIGN1SHL6: AlignmentStruct = AlignmentStruct(1 << 6);
        const _ALIGN1SHL7: AlignmentStruct = AlignmentStruct(1 << 7);
        const _ALIGN1SHL8: AlignmentStruct = AlignmentStruct(1 << 8);
        const _ALIGN1SHL9: AlignmentStruct = AlignmentStruct(1 << 9);
        const _ALIGN1SHL10: AlignmentStruct = AlignmentStruct(1 << 10);
        const _ALIGN1SHL11: AlignmentStruct = AlignmentStruct(1 << 11);
        const _ALIGN1SHL12: AlignmentStruct = AlignmentStruct(1 << 12);
        const _ALIGN1SHL13: AlignmentStruct = AlignmentStruct(1 << 13);
        const _ALIGN1SHL14: AlignmentStruct = AlignmentStruct(1 << 14);
        const _ALIGN1SHL15: AlignmentStruct = AlignmentStruct(1 << 15);
    }

    fn alignment_fail(align: AlignmentStruct) {
        let int = 7;
        assert_eq!(std::mem::align_of_val(&int) % align.0, 0);
    }

    #[kani::requires(align == AlignmentStruct::_ALIGN1SHL0 || align == AlignmentStruct::_ALIGN1SHL1 || align == AlignmentStruct::_ALIGN1SHL2)]
    fn alignment_pass(align: AlignmentStruct) {
        let int = 7;
        assert_eq!(std::mem::align_of_val(&int) % align.0, 0);
    }
}

mod should_not_derive {
    use super::should_derive::*;

    struct StrStruct(&'static str);
    struct PtrStruct(*const i8);
    struct RefStruct(&'static mut i32);
    struct RecursiveFoo(NamedMultipleStruct);

    fn no_structs_eligible(val: StrStruct, val2: PtrStruct) {}
    fn recursively_eligible(val: RecursiveFoo) {}
    fn some_arguments_support(supported: NamedMultipleStruct, unsupported: RefStruct) {}
}
