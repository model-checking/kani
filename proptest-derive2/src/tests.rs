//! This module provides integration tests that test the expansion
//! of the derive macro.

extern crate proc_macro2;

//==============================================================================
// Macros:
//==============================================================================

// Borrowed from:
// https://docs.rs/synstructure/0.7.0/src/synstructure/macros.rs.html#104-135
macro_rules! test_derive {
    ($name:path { $($i:tt)* } expands to { $($o:tt)* }) => {
        {
            #[allow(dead_code)]
            fn ensure_compiles() {
                $($i)*
                $($o)*
            }

            test_derive!($name { $($i)* } expands to { $($o)* } no_build);
        }
    };
    ($name:path { $($i:tt)* } expands to { $($o:tt)* } no_build) => {
        {
            let expected = stringify!( $($o)* )
                .parse::<$crate::tests::proc_macro2::TokenStream>()
                .expect("output should be a valid TokenStream");

            let i = stringify!( $($i)* );
            let parsed = $crate::syn::parse_str::<$crate::syn::DeriveInput>(i).expect(
                concat!("Failed to parse input to `#[derive(",
                    stringify!($name),
                ")]`")
            );
            let res = $name(parsed);
            assert_eq!(
                format!("{}", res),
                format!("{}", expected)
            );
        }
    };
}

macro_rules! test {
    (no_build $test_name:ident { $($i:tt)* } expands to { $($o:tt)* }) => {
        #[test]
        fn $test_name() {
            test_derive!(
                $crate::derive::impl_proptest_arbitrary { $($i)* }
                expands to { $($o)* } no_build
            );
        }
    };
    ($test_name:ident { $($i:tt)* } expands to { $($o:tt)* }) => {
        #[test]
        fn $test_name() {
            test_derive!(
                $crate::derive::impl_proptest_arbitrary { $($i)* }
                expands to { $($o)* }
            );
        }
    };
}

//==============================================================================
// Unit structs:
//==============================================================================

test! {
    struct_unit_unit {
        #[derive(Debug)]
        struct MyUnitStruct;
    } expands to {
        #[allow(non_upper_case_globals)]
        const _IMPL_PROPTEST_ARBITRARY_FOR_MyUnitStruct : () = {
            extern crate proptest as crate_proptest;
        impl crate_proptest::arbitrary::Arbitrary for MyUnitStruct {
            type Parameters = ();
            type Strategy = fn() -> Self;

            fn arbitrary_with(_top: Self::Parameters) -> Self::Strategy {
                || MyUnitStruct {}
            }
        }
        };
    }
}

test! {
    struct_unit_tuple {
        #[derive(Debug)]
        struct MyTupleUnitStruct();
    } expands to {
        #[allow(non_upper_case_globals)]
        const _IMPL_PROPTEST_ARBITRARY_FOR_MyTupleUnitStruct : () = {
            extern crate proptest as crate_proptest;
        impl crate_proptest::arbitrary::Arbitrary for MyTupleUnitStruct {
            type Parameters = ();
            type Strategy = fn() -> Self;

            fn arbitrary_with(_top: Self::Parameters) -> Self::Strategy {
                || MyTupleUnitStruct {}
            }
        }
        };
    }
}

test! {
    struct_unit_named {
        #[derive(Debug)]
        struct MyNamedUnitStruct {}
    } expands to {
        #[allow(non_upper_case_globals)]
        const _IMPL_PROPTEST_ARBITRARY_FOR_MyNamedUnitStruct : () = {
            extern crate proptest as crate_proptest;
        impl crate_proptest::arbitrary::Arbitrary for MyNamedUnitStruct {
            type Parameters = ();
            type Strategy = fn() -> Self;

            fn arbitrary_with(_top: Self::Parameters) -> Self::Strategy {
                || MyNamedUnitStruct {}
            }
        }
        };
    }
}

//==============================================================================
// Struct, Has top param:
//==============================================================================

/*
test! {
    named_struct_has_top_param {
        #[derive(Default)]
        struct ComplexType {
            int_max: i64,
        }

        #[derive(Debug)]
        #[proptest(params(ComplexType))]
        struct TopHasParams {
            string: String,
            #[proptest(strategy = "0..params.int_max")]
            int: i64,
        }
    } expands to {
        #[allow(non_upper_case_globals)]
        const IMPL_PROPTEST_ARBITRARY_FOR_TopHasParams: () = {
        extern crate proptest as crate_proptest;
        impl crate_proptest::arbitrary::Arbitrary for TopHasParams {
            type Parameters = ComplexType;
            type Strategy = crate_proptest::strategy::Map<
                (
                    <String as crate_proptest::arbitrary::Arbitrary>::Strategy,
                    crate_proptest::strategy::BoxedStrategy<i64>,
                ),
                fn(
                    crate_proptest::strategy::ValueFor<(
                        <String as crate_proptest::arbitrary::Arbitrary>::Strategy,
                        crate_proptest::strategy::BoxedStrategy<i64>,
                    ),>,
                ) -> Self,
            >;
            fn arbitrary_with(_top: Self::Parameters) -> Self::Strategy {
                {
                    let params = _top;
                    crate_proptest::strategy::Strategy::prop_map(
                        (
                            crate_proptest::arbitrary::any::<String>(),
                            crate_proptest::strategy::Strategy::boxed(0..params.int_max),
                        ),
                        |(tmp_0, tmp_1)| TopHasParams {
                            string: tmp_0,
                            int: tmp_1,
                        },
                    )
                }
            }
        }
        };
    }
}

*/
