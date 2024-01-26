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

// Bind `core::assert` to a different name to avoid possible name conflicts if a
// crate uses `extern crate std as core`. See
// https://github.com/model-checking/kani/issues/1949
#[cfg(not(feature = "concrete_playback"))]
#[allow(unused_imports)]
pub use core::assert as __kani__workaround_core_assert;

#[cfg(not(feature = "concrete_playback"))]
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
/// "The sum of {} and {} is {}", a, b, c
#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! assert {
    ($cond:expr $(,)?) => {
        // The double negation is to resolve https://github.com/model-checking/kani/issues/2108
        kani::assert(!!$cond, concat!("assertion failed: ", stringify!($cond)));
    };
    ($cond:expr, $($arg:tt)+) => {{
        kani::assert(!!$cond, concat!(stringify!($($arg)+)));
        // Process the arguments of the assert inside an unreachable block. This
        // is to make sure errors in the arguments (e.g. an unknown variable or
        // an argument that does not implement the Display or Debug traits) are
        // reported, without creating any overhead on verification performance
        // that may arise from processing strings involved in the arguments.
        // Note that this approach is only correct with the "abort" panic
        // strategy, but is unsound with the "unwind" panic strategy which
        // requires evaluating the arguments (because they might have side
        // effects). This is fine until we add support for the "unwind" panic
        // strategy, which is tracked in
        // https://github.com/model-checking/kani/issues/692
        if false {
            __kani__workaround_core_assert!(true, $($arg)+);
        }
    }};
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
#[cfg(not(feature = "concrete_playback"))]
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

#[cfg(not(feature = "concrete_playback"))]
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
#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! debug_assert {
    ($($x:tt)*) => ({ $crate::assert!($($x)*); })
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! debug_assert_eq {
    ($($x:tt)*) => ({ $crate::assert_eq!($($x)*); })
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! debug_assert_ne {
    ($($x:tt)*) => ({ $crate::assert_ne!($($x)*); })
}

// Override the print macros to skip all the printing functionality (which
// is not relevant for verification)
#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! print {
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! eprint {
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! println {
    () => { };
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! eprintln {
    () => { };
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! unreachable {
    // The argument, if present, is a literal that represents the error message, i.e.:
    // `unreachable!("Error message")` or `unreachable!()`
    ($($msg:literal)? $(,)?) => (
        kani::panic(concat!("internal error: entered unreachable code: ", $($msg)?))
    );
    // The argument is an expression, such as a variable.
    // ```
    // let msg = format!("Error: {}", code);
    // unreachable!(msg);
    // ```
    // This was supported for 2018 and older rust editions.
    // TODO: Is it possible to trigger an error if 2021 and above?
    // https://github.com/model-checking/kani/issues/1375
    ($($msg:expr)? $(,)?) => (
        kani::panic(concat!("internal error: entered unreachable code: ", stringify!($($msg)?)))
    );
    // The first argument is the format and the rest contains tokens to be included in the msg.
    // `unreachable!("Error: {}", code);`
    // We have the same issue as with panic!() described bellow where we over-approx what we can
    // handle.
    ($fmt:expr, $($arg:tt)*) => {{
        if false {
            __kani__workaround_core_assert!(true, $fmt, $($arg)+);
        }
        kani::panic(concat!("internal error: entered unreachable code: ",
        stringify!($fmt, $($arg)*)))}};
}

#[cfg(not(feature = "concrete_playback"))]
#[macro_export]
macro_rules! panic {
    // No argument is given.
    () => (
        kani::panic("explicit panic")
    );
    // The argument is a literal that represents the error message, i.e.:
    // `panic!("Error message")`
    ($msg:literal $(,)?) => ({
        if false {
            __kani__workaround_core_assert!(true, $msg);
        }
        kani::panic(concat!($msg))
    });
    // The argument is an expression, such as a variable.
    // ```
    // let msg = format!("Error: {}", code);
    // panic!(msg);
    // ```
    // This was supported for 2018 and older rust editions.
    // TODO: Is it possible to trigger an error if 2021 and above?
    // https://github.com/model-checking/kani/issues/1375
    ($msg:expr $(,)?) => ({
        kani::panic(stringify!($msg));
    });
    // All other cases, e.g.:
    // `panic!("Error: {}", code);`
    ($($arg:tt)+) => {{
        if false {
            __kani__workaround_core_assert!(true, $($arg)+);
        }
        kani::panic(stringify!($($arg)+));
    }};
}
