//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Arbitrary implementations for `std::env`.

use std::env::*;
use std::iter::once;
use std::ffi::OsString;

use strategy::*;
use strategy::statics::static_map;
use arbitrary::*;

// FIXME: SplitPaths when lifetimes in strategies are possible.

lazy_just!(
    Args, args;
    ArgsOs, args_os;
    Vars, vars;
    VarsOs, vars_os;
    JoinPathsError, jpe
);

#[cfg(not(target_os = "windows"))]
fn jpe() -> JoinPathsError {
    join_paths(once(":")).unwrap_err()
}

#[cfg(target_os = "windows")]
fn jpe() -> JoinPathsError {
    join_paths(once("\"")).unwrap_err()
}

// Algorithm from: https://stackoverflow.com/questions/47749164
#[cfg(target_os = "windows")]
fn make_utf16_invalid(buf: &mut Vec<u16>, p: usize) {
    // Verify that length is non-empty.
    // An empty string is always valid UTF-16.
    let len = buf.len();
    assert!(len > 0);

    // If first elem or previous entry is not a leading surrogate.
    let gen_trail = (0 == p) || (0xd800 != buf[p - 1] & 0xfc00);
    // If last element or succeeding entry is not a traililng surrogate.
    let gen_lead = (p == buf.len() - 1) || (0xdc00 != buf[p + 1] & 0xfc00);
    let (force_bits_mask, force_bits_value) = if gen_trail {
        if gen_lead {
            // Trailing or leading surrogate.
            (0xf800, 0xd800)
        } else {
            // Trailing surrogate.
            (0xfc00, 0xdc00)
        }
    } else {
        // Leading surrogate.
        debug_assert!(gen_lead);
        (0xfc00, 0xd800)
    };
    debug_assert_eq!(0, (force_bits_value & !force_bits_mask));
    buf[p] = (buf[p] & !force_bits_mask) | force_bits_value;
}

/// Generates the set of `WTF-16 \ UTF-16` and makes
/// an `OsString` that is not a valid String from it.
#[cfg(target_os = "windows")]
fn osstring_invalid_string() -> BoxedStrategy<OsString> {
    use std::os::windows::ffi::OsStringExt;
    use collection::vec;

    any::<u16>().prop_flat_map(|vlen| {
        let len = vlen as usize;
        let sbuf = vec(..::std::u16::MAX, len..len + 1);
        static_map((sbuf, 0..len - 1), |(mut buf, p)| {
            make_utf16_invalid(&mut buf, p);
            OsString::from_wide(buf.as_slice()).into_string().unwrap_err()
        })
    }).boxed()
}

#[cfg(not(target_os = "windows"))]
fn osstring_invalid_string() -> BoxedStrategy<OsString> {
    use std::os::unix::ffi::OsStringExt;
    static_map(not_utf8_bytes(true), OsString::from_vec).boxed()
}

arbitrary!(VarError,
    TupleUnion<(
        W<Just<Self>>,
        W<SFnPtrMap<BoxedStrategy<OsString>, Self>>
    )>;
    prop_oneof![
        Just(VarError::NotPresent),
        static_map(osstring_invalid_string(), VarError::NotUnicode)
    ]
);

#[cfg(test)]
mod test {
    no_panic_test!(
        args => Args,
        args_os => ArgsOs,
        vars => Vars,
        vars_os => VarsOs,
        join_paths_error => JoinPathsError,
        var_error => VarError
    );
}