// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance with different struct definitions
//! The test is from https://github.com/model-checking/kani/issues/1958.
//! With CBMC 5.72.0, all harnesses take ~1 second

enum Expr {
    A,
    B(Box<Expr>),
    C(Box<Expr>, Box<Expr>),
    D(Box<Expr>, Box<Expr>, Box<Expr>),
    E(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
}

enum Result<S, T> {
    Ok(S),
    Err(T),
}

enum Err<X, Y, Z> {
    A(X),
    B(Y, Z),
}

type Err1 = Err<String, String, String>;
type Err2<'a> = Err<String, &'a str, String>;
type Err3<'a> = Err<String, String, &'a str>;
type Err4<'a> = Err<String, &'a str, &'a str>;
type Err5<'a> = Err<&'a str, String, String>;
type Err6<'a> = Err<&'a str, &'a str, String>;
type Err7<'a> = Err<&'a str, String, &'a str>;
type Err8<'a> = Err<&'a str, &'a str, &'a str>;
type Err9<'a> = Err<Expr, &'a str, String>;
type Err10<'a> = Err<Box<Expr>, &'a str, String>;

// Takes >10s
#[cfg_attr(kani, kani::proof, kani::unwind(2))]
fn slow_harness1() {
    let _: Result<Expr, Err2> = Result::Ok(Expr::A);
}

// Takes >10s
#[cfg_attr(kani, kani::proof, kani::unwind(2))]
fn slow_harness2() {
    let _: Result<Expr, Err9> = Result::Ok(Expr::A);
}

// Takes ~1s
#[cfg_attr(kani, kani::proof, kani::unwind(2))]
fn fast_harness() {
    let _: Result<Expr, Err1> = Result::Ok(Expr::A);
    //let _: Result<Expr, Err2> = Result::Ok(Expr::A);
    let _: Result<Expr, Err3> = Result::Ok(Expr::A);
    let _: Result<Expr, Err4> = Result::Ok(Expr::A);
    let _: Result<Expr, Err5> = Result::Ok(Expr::A);
    let _: Result<Expr, Err6> = Result::Ok(Expr::A);
    let _: Result<Expr, Err7> = Result::Ok(Expr::A);
    let _: Result<Expr, Err8> = Result::Ok(Expr::A);
    //let _: Result<Expr, Err9> = Result::Ok(Expr::A);
    let _: Result<Expr, Err10> = Result::Ok(Expr::A);
}

fn main() {}
