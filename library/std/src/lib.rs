// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The purpose of this crate is to allow kani to selectively override
//! definitions from the standard library.  Definitions provided below would
//! override the standard library versions.

// See discussion in
// https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
// for more details.

// re-export all std symbols
pub use std::*;

/// This assert macro calls kani's assert function passing it down the condition
/// as well as a message that will be used when reporting the assertion result.
///
/// For the first form that does not involve a message, the macro will generate the following message:
/// assertion failed: cond
/// where `cond` is the stringified condition. For example, for
/// ```rust
/// assert!(1 + 1 == 2);
/// ```
/// the message will be:
/// assertion failed: 1 + 1 == 2
///
/// For the second form that involves a custom message possibly with arguments,
/// the macro will generate a message that is a concat of the custom message
/// along with all the arguments. For example, for
/// ```rust
/// assert!(a + b == c, "The sum of {} and {} is {}", a, b, c);
/// ```
/// the assert message will be:
/// "The sum of {} and {} is {}", 1, 1, 2
#[macro_export]
macro_rules! assert {
    ($cond:expr $(,)?) => {
        kani::assert($cond, concat!("assertion failed: ", stringify!($cond)));
    };
    ($cond:expr, $($arg:tt)+) => {
        // Note that by stringifying the arguments to the custom message, any
        // compile-time checks on those arguments (e.g. checking that the symbol
        // is defined and that it implements the Display trait) are bypassed:
        // https://github.com/model-checking/kani/issues/803
        kani::assert($cond, concat!(stringify!($($arg)+)));
    };
}

#[macro_export]
macro_rules! evaluate_print_args {
    () => { /* do nothing */ };
    ($x:expr $(, $arg:expr)* $(,)?) => {
        // Evaluate each of the arguments since they may have side effects
        {
            $(
                $arg;
            )*
        }
    };
}

// Override the print macros to skip all the formatting functionality (which
/// is not relevant for verification)
#[macro_export]
macro_rules! print {
    ($($x:tt)*) => { evaluate_print_args!($($x)*); };
}

#[macro_export]
macro_rules! eprint {
    ($($x:tt)*) => { evaluate_print_args!($($x)*); };
}

#[macro_export]
macro_rules! println {
    ($($x:tt)*) => { evaluate_print_args!($($x)*); };
}

#[macro_export]
macro_rules! eprintln {
    ($($x:tt)*) => { evaluate_print_args!($($x)*); };
}
