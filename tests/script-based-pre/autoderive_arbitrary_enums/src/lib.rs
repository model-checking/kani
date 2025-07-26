// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Test deriving Arbitrary for enums inside the compiler

#![allow(dead_code)]
#![allow(unused_variables)]

mod should_derive {
    pub enum Foo {
        UnitVariant,
        AnonVariant(u8),
        AnonMultipleVariant(u32, char),
        NamedVariant { val: u32 },
        NamedMultipleVariant { num: i128, char: char },
    }

    pub enum MultipleGenerics<T, U> {
        First(T),
        Second { val: U },
        Both(T, U),
        Neither,
    }

    pub enum PartiallyUsedGenerics<T, U> {
        Data(T),
        Count(usize),
        Optional(Option<U>),
    }

    fn foo(foo: Foo, divisor: i128) {
        match foo {
            Foo::UnitVariant
            | Foo::AnonVariant(_)
            | Foo::AnonMultipleVariant(..)
            | Foo::NamedVariant { .. } => {}
            Foo::NamedMultipleVariant { num, char } if num % divisor == 0 => {}
            _ => panic!("foo held an i28, but it didn't divide evenly"),
        }
    }

    fn multiple_generics_test(foo: MultipleGenerics<usize, char>) -> usize {
        match foo {
            MultipleGenerics::First(n) => {
                assert!(n % 2 > 0);
                n % 2
            }
            _ => 0,
        }
    }

    fn partially_used_generics_test(
        foo: PartiallyUsedGenerics<Option<Option<(u64, u32)>>, bool>,
    ) -> usize {
        match foo {
            PartiallyUsedGenerics::Data(opt) => {
                opt.unwrap_or(Some((0, 0))).unwrap_or((0, 0)).1 as usize + 100
            }
            _ => 0,
        }
    }

    #[derive(Eq, PartialEq)]
    pub enum AlignmentEnum {
        _Align1Shl0 = 1 << 0,
        _Align1Shl1 = 1 << 1,
        _Align1Shl2 = 1 << 2,
        _Align1Shl3 = 1 << 3,
        _Align1Shl4 = 1 << 4,
        _Align1Shl5 = 1 << 5,
        _Align1Shl6 = 1 << 6,
        _Align1Shl7 = 1 << 7,
        _Align1Shl8 = 1 << 8,
        _Align1Shl9 = 1 << 9,
        _Align1Shl10 = 1 << 10,
        _Align1Shl11 = 1 << 11,
        _Align1Shl12 = 1 << 12,
        _Align1Shl13 = 1 << 13,
        _Align1Shl14 = 1 << 14,
        _Align1Shl15 = 1 << 15,
    }

    fn alignment_fail(align: AlignmentEnum) {
        let int = 7;
        assert_eq!(std::mem::align_of_val(&int) % (align as usize), 0);
    }

    #[kani::requires(align == AlignmentEnum::_Align1Shl0 || align == AlignmentEnum::_Align1Shl1 || align == AlignmentEnum::_Align1Shl2)]
    fn alignment_pass(align: AlignmentEnum) {
        let int = 7;
        assert_eq!(std::mem::align_of_val(&int) % (align as usize), 0);
    }

    enum RecursivelyEligible {
        Foo(Foo),
    }

    enum ComplexGenerics<T, U, V> {
        First(Result<T, V>),
        Second(MultipleGenerics<U, V>),
        Third((T, U)),
    }

    fn recursively_eligible(val: RecursivelyEligible) {}
    fn generic_recursively_eligible(val: ComplexGenerics<char, u32, i8>) {}
}

mod should_not_derive {
    use super::should_derive::*;
    use std::marker::PhantomPinned;

    // Zero-variant enum
    enum Never {}

    // None of the variants impl Arbitrary
    enum NoVariantsEligible {
        Str(&'static str),
        Ptr(*const i8),
    }

    // At least one of the variants doesn't impl Arbitrary
    enum NotAllVariantsEligible {
        Pin(PhantomPinned),
        Ref(&'static mut i32),
        Num(u32),
    }

    // Generic enum with unsupported field type
    enum UnsupportedGenericField<T> {
        First(Vec<T>),
        Second(T),
    }

    fn never(n: Never) {}
    fn no_variants_eligible(val: NoVariantsEligible) {}
    fn not_all_variants_eligible(val: NotAllVariantsEligible) {}
    fn some_arguments_support(
        unsupported: Foo,
        supported: MultipleGenerics<char, i8>,
        unsupported_2: NotAllVariantsEligible,
    ) {
    }
    fn generic_unsupported_arg(unsupported: UnsupportedGenericField<char>) {}
}
