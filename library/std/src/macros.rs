pub extern crate core;
pub extern crate kani;

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
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! assert {
    ($cond:expr $(,)?) => {
        // The double negation is to resolve https://github.com/model-checking/kani/issues/2108
        $crate::macros::kani::assert(!!$cond, concat!("assertion failed: ", stringify!($cond)));
    };
    // Before edition 2021, the `assert!` macro could take a single argument
    // that wasn't a string literal. This is not supported in edition 2021 and above.
    // Because we reexport the 2021 edition macro, we need to support this
    // case. For this, if there is a single argument we do the following:
    // If it is a literal: Just pass it through and stringify it.
    // If it isn't a literal: We add a default format
    // specifier to the macro (see https://github.com/model-checking/kani/issues/1375).
    ($cond:expr, $first:literal $(,)?) => {{
        $crate::macros::kani::assert(!!$cond, stringify!($first));
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
            $crate::macros::core::assert!(true, "{}", $first);
        }
    }};
    ($cond:expr, $first:expr $(,)?) => {{
        $crate::macros::kani::assert(!!$cond, stringify!($first));
        // See comment above
        if false {
            $crate::macros::core::assert!(true, "{}", $first);
        }
    }};
    ($cond:expr, $($arg:tt)+) => {{
        $crate::macros::kani::assert(!!$cond, concat!(stringify!($($arg)+)));
        // See comment above
        if false {
            $crate::macros::core::assert!(true, $($arg)+);
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
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr $(,)?) => ({
        // Add parentheses around the operands to avoid a "comparison operators
        // cannot be chained" error, but exclude the parentheses in the message
        $crate::macros::kani::assert(($left) == ($right), concat!("assertion failed: ", stringify!($left == $right)));
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        assert!(($left) == ($right), $($arg)+);
    });
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr $(,)?) => ({
        // Add parentheses around the operands to avoid a "comparison operators
        // cannot be chained" error, but exclude the parentheses in the message
        $crate::macros::kani::assert(($left) != ($right), concat!("assertion failed: ", stringify!($left != $right)));
    });
    ($left:expr, $right:expr, $($arg:tt)+) => ({
        assert!(($left) != ($right), $($arg)+);
    });
}

// Treat the debug assert macros same as non-debug ones
#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! debug_assert {
    ($($x:tt)*) => ({ if cfg!(debug_assertions) { $crate::assert!($($x)*); } })
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! debug_assert_eq {
    ($($x:tt)*) => ({ if cfg!(debug_assertions) { $crate::assert_eq!($($x)*); } })
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! debug_assert_ne {
    ($($x:tt)*) => ({ if cfg!(debug_assertions) { $crate::assert_ne!($($x)*); } })
}

// Override the print macros to skip all the printing functionality (which
// is not relevant for verification)
#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! print {
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! eprint {
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! eprintln {
    () => { $crate::eprint!("\n") };
    ($($x:tt)*) => {{ let _ = format_args!($($x)*); }};
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! unreachable {
    // The argument, if present, is a literal that represents the error message, i.e.:
    // `unreachable!("Error message")` or `unreachable!()`
    ($($msg:literal)? $(,)?) => (
        $crate::macros::kani::panic(concat!("internal error: entered unreachable code: ", $($msg)?))
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
        $crate::macros::kani::panic(concat!("internal error: entered unreachable code: ", stringify!($($msg)?)))
    );
    // The first argument is the format and the rest contains tokens to be included in the msg.
    // `unreachable!("Error: {}", code);`
    // We have the same issue as with panic!() described bellow where we over-approx what we can
    // handle.
    ($fmt:expr, $($arg:tt)*) => {{
        if false {
            $crate::macros::core::assert!(true, $fmt, $($arg)+);
        }
        $crate::macros::kani::panic(concat!("internal error: entered unreachable code: ",
        stringify!($fmt, $($arg)*)))}};
}

#[cfg(not(feature = "concrete_playback"))]
#[stable(feature = "rust1", since = "1.0.0")]
#[macro_export]
macro_rules! panic {
    // No argument is given.
    () => (
        $crate::macros::kani::panic("explicit panic")
    );
    // The argument is a literal that represents the error message, i.e.:
    // `panic!("Error message")`
    ($msg:literal $(,)?) => ({
        if false {
            $crate::macros::core::assert!(true, $msg);
        }
        $crate::macros::kani::panic(concat!($msg))
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
        $crate::macros::kani::panic(stringify!($msg));
    });
    // All other cases, e.g.:
    // `panic!("Error: {}", code);`
    ($($arg:tt)+) => {{
        if false {
            $crate::macros::core::assert!(true, $($arg)+);
        }
        $crate::macros::kani::panic(stringify!($($arg)+));
    }};
}
