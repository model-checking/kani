// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test enum support in Strata backend

enum Color {
    Red,
    Green,
    Blue,
}

enum Option<T> {
    Some(T),
    None,
}

#[kani::proof]
fn test_simple_enum() {
    let color = Color::Red;
    // Enums represented as discriminants
}

#[kani::proof]
fn test_option_some() {
    let x: Option<u32> = Option::Some(42);
    match x {
        Option::Some(v) => assert!(v == 42),
        Option::None => assert!(false),
    }
}

#[kani::proof]
fn test_option_none() {
    let x: Option<u32> = Option::None;
    match x {
        Option::Some(_) => assert!(false),
        Option::None => assert!(true),
    }
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

#[kani::proof]
fn test_result() {
    let r: Result<u32, bool> = Result::Ok(100);
    match r {
        Result::Ok(v) => assert!(v == 100),
        Result::Err(_) => assert!(false),
    }
}
