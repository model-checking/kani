// Copyright 2018 Mazdak Farrokhzad
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_variables, unused_imports)]

#[macro_use]
extern crate proptest_derive;

extern crate proptest;

#[test]
fn derive_unit_struct() {
    #[derive(Debug, Arbitrary)]
    struct U1;

    #[derive(Debug, Arbitrary)]
    struct U2();

    #[derive(Debug, Arbitrary)]
    struct U3 {}
}

#[test]
fn derive_struct_top_params() {
    #[derive(Default)]
    struct ComplexType {
        int_max: i64,
    }

    #[derive(Debug, Arbitrary)]
    #[proptest(params(ComplexType))]
    struct TopHasParams {
        string: String,
        #[proptest(strategy = "0..params.int_max")]
        int: i64,
    }

    #[derive(Debug, Arbitrary)]
    #[proptest(no_params)]
    struct TopNoParams {
        string: String,
        int: i64,
    }

    #[derive(Debug, Arbitrary)]
    struct Bar {
        string: String,
        #[proptest(strategy = "0i64..1i64")]
        int: i64,
    }

    use std::marker::PhantomData;
    #[derive(Debug, Arbitrary)]
    struct Bar2<A, B, C> {
        a: A,
        b: B,
        c: PhantomData<C>,
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    struct A {
        string: String,
        #[proptest(no_params)]
        int: i64,
        float: f64,
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    #[proptest(params(ComplexType))]
    struct B {
        #[proptest(strategy = "\"abc\"")]
        string: String,
        #[proptest(strategy = "0..params.int_max")]
        int: i64,
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    struct C {
        #[proptest(params = "&'static str", strategy = "params")]
        string: String,
        #[proptest(params(u8), strategy = "0i64..params as i64")]
        int: i64,
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    struct D {
        #[proptest(params(u8), strategy = "0i64..params as i64")]
        int: i64,
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    struct F {
        #[proptest(value = "1")]
        x: usize
    }

    // OK.
    #[derive(Debug, Arbitrary)]
    struct E;

    // OK.
    #[derive(Debug, Arbitrary)]
    struct G(u32, u32);

    // OK.
    #[derive(Debug, Arbitrary)]
    struct H {
        #[proptest(params = "()", strategy = "1u32..2")]
        a: u32,
        #[proptest(params = "()", strategy = "1u32..3")]
        b: u32,
    }

/*
*/
    
    // An idea.
    /*
    #[derive(Debug, Arbitrary)]
    #[proptest(with = "Foo::ctor(1337, :usize:.other_fn(:f64:, #0..7#))")]
    struct Foo {
        //..
    }
    */
}

#[test]
fn derive_enum() {
    #[derive(Default)]
    struct Complex;

    #[derive(Debug, Arbitrary)]
    #[proptest(params(Complex))]
    enum Foo {
        #[proptest(value = "Foo::F0(1, 1)")]
        F0(usize, u8),
    }

    #[derive(Clone, Debug, Arbitrary)]
    #[proptest(params = "usize")]
    enum A {
        //#[proptest(value = "A::B")]
        B,
        #[proptest(strategy = "Just(A::C(1))")]
        C(usize)
    }

    use proptest::strategy::Just;

    #[derive(Clone, Debug, Arbitrary)]
    enum Bobby {
        #[proptest(no_params)]
        B(usize),
        #[proptest(no_params, value = "Bobby::C(1)")]
        C(usize),
        #[proptest(no_params, strategy = "Just(Bobby::D(1))")]
        D(usize),
        //#[proptest(params(Complex), value = "A::E(1)")]
        //E(usize),
        #[proptest(params(Complex), strategy = "Just(Bobby::D(1))")]
        F(usize),
    }
    /*
    */

    #[derive(Clone, Debug, Arbitrary)]
    enum Quux {
        //B( #[proptest(no_params)] usize),
        //C(usize, String),
        //#[proptest(value = "A::D(2, \"a\".into())")]
        //D(usize, String),
        //#[proptest(strategy = "Just(A::E(1337))")]
        //E(u32),
        F {
            #[proptest(strategy = "10usize..20usize")]
            foo: usize
        }
    }

    #[derive(Clone, Debug, Arbitrary)]
    enum Alan {
        A(usize),
        B(String),
        C(()),
        D(u32),
        E(f64),
        F(char)
    }

    #[derive(Clone, Debug, Arbitrary)]
    struct May {}

    #[derive(Clone, Debug, Arbitrary)]
    enum Wobble {
        V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11,
    }
}

/*
*/

/*

#[test]
fn it_works() {
    /*
    // NOT OK.
    #[derive(Debug, Arbitrary)]
    #[proptest(params(ComplexType), strategy = "Just()")]
    struct A(String, i64);

    // Not OK.
    #[derive(Debug, Arbitrary)]
    #[proptest(params(ComplexType))]
    struct B {
        #[proptest(params(&'static str), strategy = "params")]
        string: String,
        #[proptest(params(max, strategy = "0..parameters.int_max")]
        int: i64,
    }
    */


    /*
    #[derive(Arbitrary)]
    enum E {
        //#[proptest(foobar)]
        //#![proptest(foobar)]
        V1 {
            //#![proptest(foobar)]
            //#[proptest(foobar)]
            field: usize,
        },
        V2(usize),
    }
    */

    /* test the thing */
}

*/
