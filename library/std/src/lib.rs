// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The purpose of this crate is to allow kani to selectively override
//! definitions from the standard library.  Definitions provided below would
//! override the standard library versions.

// See discussion in
// https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
// for more details.

// re-export all std symbols
pub use std::*;

// Override process calls with stubs.
pub mod process;

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

// Override the assert_eq and assert_ne macros to
// 1. Bypass the formatting-related code in the standard library implementation,
//    which is not relevant for verification (see
//    https://github.com/model-checking/kani/issues/14)
// 2. Generate a suitable message for the assert of the form:
//        assertion failed: $left == $right
//    instead of the uninformative:
//        a panicking function core::panicking::assert_failed is invoked
//    (see https://github.com/model-checking/kani/issues/13)
// 3. Call kani::assert so that any instrumentation that it does (e.g. injecting
//    reachability checks) is done for assert_eq and assert_ne
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => ({
        // Add parentheses around the operands to avoid a "comparison operators
        // cannot be chained" error, but exclude the parentheses in the message
        kani::assert(($left) == ($right), concat!("assertion failed: ", stringify!($left == $right)));
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        assert!(($left) == ($right), $($arg)+);
    });
}

#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr $(,)?) => ({
        // Add parentheses around the operands to avoid a "comparison operators
        // cannot be chained" error, but exclude the parentheses in the message
        kani::assert(($left) != ($right), concat!("assertion failed: ", stringify!($left != $right)));
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        assert!(($left) != ($right), $($arg)+);
    });
}

// Treat the debug assert macros same as non-debug ones
#[macro_export]
macro_rules! debug_assert {
    ($($x:tt)*) => ({ $crate::assert!($($x)*); })
}

#[macro_export]
macro_rules! debug_assert_eq {
    ($($x:tt)*) => ({ $crate::assert_eq!($($x)*); })
}

#[macro_export]
macro_rules! debug_assert_ne {
    ($($x:tt)*) => ({ $crate::assert_ne!($($x)*); })
}

#[macro_export]
macro_rules! evaluate_print_args {
    () => { /* do nothing */ };
    // For println!("Some message {} {} ...", arg1, arg2, ...)
    // $msg is "Some message {} {} ..."
    // and $arg has arg1, arg2, ...
    ($msg:expr $(, $arg:expr)* $(,)?) => {
        // Evaluate each of the arguments since they may have side effects
        {
            $(
                let _ = &$arg;
            )*
        }
    };
}

// Override the print macros to skip all the formatting functionality (which
// is not relevant for verification)
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

#[macro_export]
macro_rules! unreachable {
    ($($msg:literal)? $(,)?) => (
        kani::panic(concat!("internal error: entered unreachable code: ", $($msg)?))
    );
    // Needed for 2018 and older rust editions.
    // TODO: Is it possible to trigger an error if 2021 and above?
    ($($msg:expr)? $(,)?) => (
        kani::panic(concat!("internal error: entered unreachable code: ", stringify!($($msg)?)))
    );
    ($fmt:expr, $($arg:tt)*) => (
        kani::panic(concat!("internal error: entered unreachable code: ",
        stringify!($fmt, $($arg)*))));
}

#[macro_export]
macro_rules! panic {
    () => (
        kani::panic("explicit panic")
    );
    ($msg:literal $(,)?) => ({
        kani::panic(concat!($msg));
    });
    // Needed for 2018 and older rust editions.
    // TODO: Is it possible to trigger an error if 2021 and above?
    ($msg:expr $(,)?) => ({
        kani::panic(stringify!($msg));
    });
    ($msg:expr, $($arg:tt)*) => ({
        kani::panic(stringify!($msg, $($arg)*));
    });
}
