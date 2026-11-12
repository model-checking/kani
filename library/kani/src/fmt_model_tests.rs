// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani's models of the `derive(Debug)` helpers of `core::fmt::Formatter`
//! (`debug_struct_fields_finish` and `debug_c_like_enum_write_str` in `kani_core`'s models)
//! format exactly like the standard library. Each case formats a type with the real
//! `derive(Debug)` and a twin whose `Debug` calls the model with the names the derive passes.
//!
//! These run natively: checking exact formatter output inside Kani is too slow for a regression
//! test, and a native run compares the models against the real derive directly.

// The fields of the test types are only read by `derive(Debug)`.
#![allow(dead_code)]

use crate::rustc_intrinsics::{debug_c_like_enum_write_str, debug_struct_fields_finish};
use std::fmt::{self, Debug, Formatter};

/// A value whose `Debug` calls `debug_struct_fields_finish` with `names` and `values`.
struct ViaFieldsModel<'a> {
    names: &'a str,
    values: &'a [&'a dyn Debug],
}

impl Debug for ViaFieldsModel<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        debug_struct_fields_finish(f, self.names, self.values)
    }
}

/// A value whose `Debug` calls `debug_c_like_enum_write_str` with `names` and `start_index`.
struct ViaEnumModel<'a> {
    names: &'a str,
    start_index: usize,
}

impl Debug for ViaEnumModel<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        debug_c_like_enum_write_str(f, self.names, self.start_index)
    }
}

/// Assert that `real` and `model` print the same with `{:?}`, `{:#?}` and a width.
fn assert_same(real: &dyn Debug, model: &dyn Debug) {
    assert_eq!(format!("{real:?}"), format!("{model:?}"));
    assert_eq!(format!("{real:#?}"), format!("{model:#?}"));
    assert_eq!(format!("{real:>40?}"), format!("{model:>40?}"));
}

#[derive(Debug)]
struct One {
    a: u8,
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: &'static str,
}

#[derive(Debug)]
struct Six {
    a: u8,
    b: bool,
    c: char,
    d: (),
    e: Option<u8>,
    f: [u8; 2],
}

#[derive(Debug)]
struct Pair(u8, &'static str);

#[derive(Debug)]
struct Unit;

// Five variants: `derive(Debug)` uses `debug_c_like_enum_write_str` only from five on.
#[derive(Debug)]
enum Color {
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
enum Names {
    r#type,
    Ünïcödé,
    snake_case,
}

#[test]
fn named_structs() {
    assert_same(&One { a: 7 }, &ViaFieldsModel { names: "One a", values: &[&7u8] });
    assert_same(
        &Point { x: -1, y: "a \"quoted\" str" },
        &ViaFieldsModel { names: "Point x y", values: &[&-1i32, &"a \"quoted\" str"] },
    );
    assert_same(
        &Six { a: 1, b: true, c: 'z', d: (), e: Some(3), f: [4, 5] },
        &ViaFieldsModel {
            names: "Six a b c d e f",
            values: &[&1u8, &true, &'z', &(), &Some(3u8), &[4u8, 5]],
        },
    );
}

#[test]
fn tuple_structs() {
    assert_same(&Pair(9, "x"), &ViaFieldsModel { names: "Pair", values: &[&9u8, &"x"] });
    assert_same(&Unit, &ViaFieldsModel { names: "Unit", values: &[] });
}

#[test]
fn nested() {
    let inner = Point { x: 2, y: "in" };
    let real = (One { a: 1 }, inner);
    let model_inner = ViaFieldsModel { names: "Point x y", values: &[&2i32, &"in"] };
    let model = (ViaFieldsModel { names: "One a", values: &[&1u8] }, model_inner);
    assert_same(&real, &model);
}

#[test]
fn c_like_enums() {
    let names = "Red Green Blue Cyan Magenta";
    assert_same(&Color::Red, &ViaEnumModel { names, start_index: 0 });
    assert_same(&Color::Green, &ViaEnumModel { names, start_index: 4 });
    assert_same(&Color::Blue, &ViaEnumModel { names, start_index: 10 });
    assert_same(&Color::Cyan, &ViaEnumModel { names, start_index: 15 });
    assert_same(&Color::Magenta, &ViaEnumModel { names, start_index: 20 });
}

#[test]
fn c_like_enum_unusual_names() {
    // Raw identifiers print without `r#`, and names can contain non-ASCII characters.
    let names = "type Ünïcödé snake_case";
    let start = |name: &str| names.find(name).unwrap();
    assert_same(&Names::r#type, &ViaEnumModel { names, start_index: start("type") });
    assert_same(&Names::Ünïcödé, &ViaEnumModel { names, start_index: start("Ünïcödé") });
    assert_same(&Names::snake_case, &ViaEnumModel { names, start_index: start("snake_case") });
}
