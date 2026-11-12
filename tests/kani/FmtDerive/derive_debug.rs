// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani verifies `derive(Debug)` formatting of every shape the derive handles, through
//! Kani's models of the `core::fmt::Formatter` helpers that the derive calls
//! (`debug_struct_fields_finish` and `debug_c_like_enum_write_str`). Since rust-lang/rust#164726,
//! the standard library looks the names up with `memchr`, which made formatting a derived `Debug`
//! orders of magnitude slower to verify; the models split the names with a byte loop instead.
//!
//! The harnesses check that formatting succeeds (no failed bounds, `unwrap` or slice checks in the
//! models). That the models print exactly what the standard library prints is checked natively in
//! `library/kani/src/fmt_model_tests.rs`: checking exact formatter output in Kani is too slow for a
//! regression test.

#[derive(Debug)]
struct Point {
    x: bool,
    y: (),
}

// More fields than the `debug_struct_field{1..5}_finish` helpers take.
#[derive(Debug)]
struct Six {
    a: bool,
    b: (),
    c: bool,
    d: (),
    e: bool,
    f: (),
}

#[derive(Debug)]
struct Pair(bool, ());

// `derive(Debug)` calls `debug_c_like_enum_write_str` only for fieldless enums with at least five
// variants.
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
}

#[derive(Debug)]
enum Shape {
    Circle { r: bool },
    Line(bool, ()),
}

#[kani::proof]
fn structs() {
    let _ = format!("{:?}", Point { x: true, y: () });
    let _ = format!("{:?}", Six { a: true, b: (), c: false, d: (), e: true, f: () });
    let _ = format!("{:?}", Pair(false, ()));
}

// Each variant is formatted on its own: formatting a nondeterministic variant of a five-variant
// enum takes minutes, with or without the models.
#[kani::proof]
fn c_like_variants() {
    let _ = format!("{:?}", Color::Red);
    let _ = format!("{:?}", Color::Blue);
    let _ = format!("{:?}", Color::Magenta);
}

#[kani::proof]
fn any_enum_variant_with_data() {
    let shape =
        if kani::any() { Shape::Circle { r: kani::any() } } else { Shape::Line(kani::any(), ()) };
    let _ = format!("{shape:?}");
}
