// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z stubbing

//! Kani replaces the `core::fmt::Formatter` helpers that `derive(Debug)` calls with its own models.
//! A user stub of one of those helpers must still take precedence over the model.

use std::fmt;

#[derive(Debug)]
struct Point {
    x: bool,
    y: (),
}

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

// The helpers are methods of `impl<'a> Formatter<'a>`, so the stubs need an early-bound
// (`'a: 'a`) lifetime to match their generic parameters.
fn failing_fields_finish<'a: 'a>(
    _f: &mut fmt::Formatter<'a>,
    _names: &str,
    _values: &[&dyn fmt::Debug],
) -> fmt::Result {
    panic!("the stub of `debug_struct_fields_finish` was called")
}

fn failing_enum_write_str<'a: 'a>(
    _f: &mut fmt::Formatter<'a>,
    _names: &str,
    _start: usize,
) -> fmt::Result {
    panic!("the stub of `debug_c_like_enum_write_str` was called")
}

#[kani::proof]
#[kani::should_panic]
#[kani::stub(core::fmt::Formatter::debug_struct_fields_finish, failing_fields_finish)]
fn struct_helper_stub_is_used() {
    let _ = format!("{:?}", Point { x: true, y: () });
}

#[kani::proof]
#[kani::should_panic]
#[kani::stub(core::fmt::Formatter::debug_c_like_enum_write_str, failing_enum_write_str)]
fn enum_helper_stub_is_used() {
    let color = if kani::any() { Color::Red } else { Color::Green };
    let _ = format!("{color:?}");
}
