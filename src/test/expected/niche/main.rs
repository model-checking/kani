// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
enum Option2<T> {
    Some(T),
    None,
}

type T<'a> = Option2<(i8, &'a i8)>;

fn get_opt<'a>() -> T<'a> {
    Option2::None
}

fn get_some<'a>(a: &'a i8) -> T<'a> {
    Option2::Some((*a, a))
}

fn main() {
    let x = get_opt();
    match x {
        Option2::None => {}
        Option2::Some(_) => assert!(false),
    }
    let x = 10;
    match get_some(&x) {
        Option2::None => assert!(false),
        Option2::Some((a, b)) => assert!(a == *b),
    }
}
