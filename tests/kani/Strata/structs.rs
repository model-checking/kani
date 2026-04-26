// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test struct support in Strata backend

struct Point {
    x: u32,
    y: u32,
}

struct Rectangle {
    width: u32,
    height: u32,
}

#[kani::proof]
fn test_struct_creation() {
    let p = Point { x: 10, y: 20 };
    assert!(p.x == 10);
    assert!(p.y == 20);
}

#[kani::proof]
fn test_struct_field_access() {
    let rect = Rectangle { width: 100, height: 50 };
    let area = rect.width * rect.height;
    assert!(area == 5000);
}

#[kani::proof]
fn test_struct_assignment() {
    let mut p = Point { x: 0, y: 0 };
    p.x = 5;
    p.y = 10;
    assert!(p.x == 5);
    assert!(p.y == 10);
}

struct Nested {
    point: Point,
    value: u32,
}

#[kani::proof]
fn test_nested_struct() {
    let n = Nested {
        point: Point { x: 1, y: 2 },
        value: 42,
    };
    assert!(n.point.x == 1);
    assert!(n.point.y == 2);
    assert!(n.value == 42);
}

#[kani::proof]
fn test_tuple() {
    let pair: (u32, bool) = (42, true);
    assert!(pair.0 == 42);
    assert!(pair.1);
}

#[kani::proof]
fn test_tuple_destructure() {
    let triple: (u32, u32, u32) = (1, 2, 3);
    let sum = triple.0 + triple.1 + triple.2;
    assert!(sum == 6);
}
