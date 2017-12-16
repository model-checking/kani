//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

/// Easily define `proptest` tests.
///
/// Within `proptest!`, define one or more functions without return type
/// normally, except instead of putting `: type` after each parameter, write
/// `in strategy`, where `strategy` is an expression evaluating to some
/// `Strategy`.
///
/// Each function will be wrapped in a function which sets up a `TestRunner`,
/// and then invokes the function body with inputs generated according to the
/// strategies. Note that the inputs are borrowed from the test runner, so if
/// they are not `Copy`, you will need to use `ref` with each parameter name.
///
/// Example:
///
/// ```
/// #[macro_use] extern crate proptest;
///
/// proptest! {
///   # /*
///   #[test]
///   # */
///   fn test_addition(a in 0..10, b in 0..10) {
///     prop_assert!(a + b <= 18);
///   }
///
///   // Note the `ref a` and `ref b` --- `String` is not `Copy`,
///   // so we can't take ownership implicitly.
///   # /*
///   #[test]
///   # */
///   fn test_string_concat(ref a in ".*", ref b in ".*") {
///     let cat = format!("{}{}", a, b);
///     prop_assert_eq!(a.len() + b.len(), cat.len());
///   }
/// }
/// #
/// # fn main() { test_addition(); test_string_concat(); }
/// ```
///
/// To override the default configuration, you can start the `proptest!` block
/// with `#![proptest_config(expr)]`, where `expr` is an expression that
/// evaluates to a `proptest::test_runner::Config` (or a reference to one).
///
/// ```
/// #[macro_use] extern crate proptest;
/// use proptest::prelude::*;
///
/// proptest! {
///   #![proptest_config(ProptestConfig {
///     cases: 99, .. ProptestConfig::default()
///   })]
///   # /*
///   #[test]
///   # */
///   fn test_addition(a in 0..10, b in 0..10) {
///     prop_assert!(a + b <= 18);
///   }
/// }
/// #
/// # fn main() { test_addition(); }
/// ```
#[macro_export]
macro_rules! proptest {
    (#![proptest_config($config:expr)]
     $(
        $(#[$meta:meta])*
        fn $test_name:ident($($parm:pat in $strategy:expr),+) $body:block
    )*) => {
        $(
            $(#[$meta])*
            fn $test_name() {
                let mut runner = $crate::test_runner::TestRunner::new(
                    $config.clone());
                let names = proptest_helper!(@_WRAPSTR ($($parm),*));
                match runner.run(
                    &$crate::strategy::Strategy::prop_map(
                        proptest_helper!(@_WRAP ($($strategy)*)),
                        |values| $crate::sugar::NamedArguments(names, values)),
                    |&$crate::sugar::NamedArguments(
                        _, proptest_helper!(@_WRAPPAT ($($parm),*)))|
                    {
                        $body;
                        Ok(())
                    })
                {
                    Ok(_) => (),
                    Err(e) => panic!("{}\n{}", e, runner),
                }
            }
        )*
    };

    ($(
        $(#[$meta:meta])*
        fn $test_name:ident($($parm:pat in $strategy:expr),+) $body:block
    )*) => { proptest! {
        #![proptest_config($crate::test_runner::Config::default())]
        $($(#[$meta])*
          fn $test_name($($parm in $strategy),+) $body)*
    } };
}

/// Rejects the test input if assumptions are not met.
///
/// Used directly within a function defined with `proptest!` or in any function
/// returning `Result<_, TestCaseError>`.
///
/// This is invoked as `prop_assume!(condition, format, args...)`. `condition`
/// is evaluated; if it is false, `Err(TestCaseError::Reject)` is returned. The
/// message includes the point of invocation and the format message. `format`
/// and `args` may be omitted to simply use the condition itself as the
/// message.
#[macro_export]
macro_rules! prop_assume {
    ($expr:expr) => {
        prop_assume!($expr, "{}", stringify!($expr))
    };

    ($expr:expr, $fmt:tt $(, $fmt_arg:expr),*) => {
        if !$expr {
            return $crate::test_runner::reject_case(
                format!(concat!("{}:{}:{}: ", $fmt),
                        file!(), line!(), column!()
                        $(, $fmt_arg)*));
        }
    };
}

/// Produce a strategy which picks one of the listed choices.
///
/// This is conceptually equivalent to calling `prop_union` on the first two
/// elements and then chaining `.or()` onto the rest after implicitly boxing
/// all of them. As with `Union`, values shrink across elements on the
/// assumption that earlier ones are "simpler", so they should be listed in
/// order of ascending complexity when possible.
///
/// The macro invocation has two forms. The first is to simply list the
/// strategies separated by commas; this will cause value generation to pick
/// from the strategies uniformly. The other form is to provide a weight in the
/// form of a `u32` before each strategy, separated from the strategy with
/// `=>`.
///
/// Note that the exact type returned by the macro varies depending on how many
/// inputs there are. In particular, if given exactly one option, it will
/// return it unmodified. It is not recommended to depend on the particular
/// type produced by this macro.
///
/// ## Example
///
/// ```rust,no_run
/// #[macro_use] extern crate proptest;
/// use proptest::prelude::*;
///
/// #[derive(Clone, Copy, Debug)]
/// enum MyEnum {
///   Big(u64),
///   Medium(u32),
///   Little(i16),
/// }
///
/// # #[allow(unused_variables)]
/// # fn main() {
/// let my_enum_strategy = prop_oneof![
///   prop::num::i16::ANY.prop_map(MyEnum::Little),
///   prop::num::u32::ANY.prop_map(MyEnum::Medium),
///   prop::num::u64::ANY.prop_map(MyEnum::Big),
/// ];
///
/// let my_weighted_strategy = prop_oneof![
///   1 => prop::num::i16::ANY.prop_map(MyEnum::Little),
///   // Chose `Medium` twice as frequently as either `Little` or `Big`; i.e.,
///   // around 50% of values will be `Medium`, and 25% for each of `Little`
///   // and `Big`.
///   2 => prop::num::u32::ANY.prop_map(MyEnum::Medium),
///   1 => prop::num::u64::ANY.prop_map(MyEnum::Big),
/// ];
/// # }
/// ```
#[macro_export]
macro_rules! prop_oneof {
    ($($item:expr),+ $(,)*) => {
        prop_oneof![
            $(1 => $item),*
        ]
    };

    ($_weight0:expr => $item0:expr $(,)*) => { $item0 };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr,
     $weight5:expr => $item5:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4), ($weight5, $item5)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr,
     $weight5:expr => $item5:expr,
     $weight6:expr => $item6:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4), ($weight5, $item5),
             ($weight6, $item6)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr,
     $weight5:expr => $item5:expr,
     $weight6:expr => $item6:expr,
     $weight7:expr => $item7:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4), ($weight5, $item5),
             ($weight6, $item6), ($weight7, $item7)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr,
     $weight5:expr => $item5:expr,
     $weight6:expr => $item6:expr,
     $weight7:expr => $item7:expr,
     $weight8:expr => $item8:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4), ($weight5, $item5),
             ($weight6, $item6), ($weight7, $item7),
             ($weight8, $item8)))
    };

    ($weight0:expr => $item0:expr,
     $weight1:expr => $item1:expr,
     $weight2:expr => $item2:expr,
     $weight3:expr => $item3:expr,
     $weight4:expr => $item4:expr,
     $weight5:expr => $item5:expr,
     $weight6:expr => $item6:expr,
     $weight7:expr => $item7:expr,
     $weight8:expr => $item8:expr,
     $weight9:expr => $item9:expr $(,)*) => {
        $crate::strategy::TupleUnion::new(
            (($weight0, $item0), ($weight1, $item1),
             ($weight2, $item2), ($weight3, $item3),
             ($weight4, $item4), ($weight5, $item5),
             ($weight6, $item6), ($weight7, $item7),
             ($weight8, $item8), ($weight9, $item9)))
    };

    ($($weight:expr => $item:expr),+ $(,)*) => {
        $crate::strategy::Union::new_weighted(vec![
            $(($weight, $crate::strategy::Strategy::boxed($item))),*
        ])
    };
}

/// Convenience to define functions which produce new strategies.
///
/// The macro has two general forms. In the first, you define a function with
/// two argument lists. The first argument list uses the usual syntax and
/// becomes exactly the argument list of the defined function. The second
/// argument list uses the `in strategy` syntax as with `proptest!`, and is
/// used to generate the other inputs for the function. The second argument
/// list has access to all arguments in the first. The return type indicates
/// the type of value being generated; the final return type of the function is
/// `BoxedStrategy<$type>`.
///
/// ```rust,no_run
/// # #![allow(dead_code)]
/// #[macro_use] extern crate proptest;
///
/// #[derive(Clone, Debug)]
/// struct MyStruct {
///   integer: u32,
///   string: String,
/// }
///
/// prop_compose! {
///   fn my_struct_strategy(max_integer: u32)
///                        (integer in 0..max_integer, string in ".*")
///                        -> MyStruct {
///     MyStruct { integer, string }
///   }
/// }
/// #
/// # fn main() { }
/// ```
///
/// This form is simply sugar around making a tuple and then calling `prop_map`
/// on it.
///
/// The second form is mostly the same, except that it takes _three_ argument
/// lists. The third argument list can see all values in both prior, which
/// permits producing strategies based on other strategies.
///
/// ```rust,no_run
/// # #![allow(dead_code)]
/// #[macro_use] extern crate proptest;
///
/// prop_compose! {
///   fn nearby_numbers()(centre in -1000..1000)
///                    (a in centre-10..centre+10,
///                     b in centre-10..centre+10)
///                    -> (i32, i32) {
///     (a, b)
///   }
/// }
/// #
/// # fn main() { }
/// ```
///
/// However, the body of the function does _not_ have access to the second
/// argument list. If the body needs access to those values, they must be
/// passed through explicitly.
///
/// ```rust,no_run
/// # #![allow(dead_code)]
/// #[macro_use] extern crate proptest;
/// use proptest::prelude::*;
///
/// prop_compose! {
///   fn vec_and_index
///     (max_length: usize)
///     (vec in prop::collection::vec(1..10, 1..max_length))
///     (index in 0..vec.len(), vec in Just(vec))
///     -> (Vec<i32>, usize)
///   {
///     (vec, index)
///   }
/// }
/// # fn main() { }
/// ```
///
/// The second form is sugar around making a strategy tuple, calling
/// `prop_flat_map()`, then `prop_map()`.
///
/// To give the function a visibility or unsafe modifier, put it in brackets
/// before the `fn` token.
///
/// ```rust,no_run
/// # #![allow(dead_code)]
/// #[macro_use] extern crate proptest;
/// use proptest::prelude::*;
///
/// prop_compose! {
///   [pub(crate) unsafe] fn pointer()(v in prop::num::usize::ANY)
///                                 -> *const () {
///     v as *const ()
///   }
/// }
/// # fn main() { }
/// ```
///
/// ## Comparison with Hypothesis' `@composite`
///
/// `prop_compose!` makes it easy to do a lot of things you can do with
/// [Hypothesis' `@composite`](https://hypothesis.readthedocs.io/en/latest/data.html#composite-strategies),
/// but not everything.
///
/// - You can't filter via this macro. For filtering, you need to make the
/// strategy the "normal" way and use `prop_filter()`.
///
/// - More than two layers of strategies or arbitrary logic between the two
/// layers. If you need either of these, you can achieve them by calling
/// `prop_flat_map()` by hand.
#[macro_export]
macro_rules! prop_compose {
    ($(#[$meta:meta])*
     $([$($vis:tt)*])* fn $name:ident $params:tt
     ($($var:pat in $strategy:expr),+ $(,)*)
       -> $return_type:ty $body:block) =>
    {
        $(#[$meta])*
        $($($vis)*)* fn $name $params
                 -> $crate::strategy::BoxedStrategy<$return_type> {
            let strat = proptest_helper!(@_WRAP ($($strategy)*));
            let strat = $crate::strategy::Strategy::prop_map(
                strat,
                |proptest_helper!(@_WRAPPAT ($($var),*))| $body);
            $crate::strategy::Strategy::boxed(strat)
        }
    };

    ($(#[$meta:meta])*
     $([$($vis:tt)*])* fn $name:ident $params:tt
     ($($var:pat in $strategy:expr),+ $(,)*)
     ($($var2:pat in $strategy2:expr),+ $(,)*)
       -> $return_type:ty $body:block) =>
    {
        $(#[$meta])*
        $($($vis)*)* fn $name $params
                 -> $crate::strategy::BoxedStrategy<$return_type> {
            let strat = proptest_helper!(@_WRAP ($($strategy)*));
            let strat = $crate::strategy::Strategy::prop_flat_map(
                strat,
                |proptest_helper!(@_WRAPPAT ($($var),*))|
                proptest_helper!(@_WRAP ($($strategy2)*)));
            let strat = $crate::strategy::Strategy::prop_map(
                strat,
                |proptest_helper!(@_WRAPPAT ($($var2),*))| $body);
            $crate::strategy::Strategy::boxed(strat)
        }
    };
}

/// Similar to `assert!` from std, but returns a test failure instead of
/// panicking if the condition fails.
///
/// This can be used in any function that returns a `Result<_, TestCaseError>`,
/// including the top-level function inside `proptest!`.
///
/// Both panicking via `assert!` and returning a test case failure have the
/// same effect as far as proptest is concerned; however, the Rust runtime
/// implicitly prints every panic to stderr by default (including a backtrace
/// if enabled), which can make test failures unnecessarily noisy. By using
/// `prop_assert!` instead, the only output on a failing test case is the final
/// panic including the minimal test case.
///
/// ## Example
///
/// ```
/// #[macro_use] extern crate proptest;
/// use proptest::prelude::*;
///
/// proptest! {
///   # /*
///   #[test]
///   # */
///   fn triangle_inequality(a in 0.0f64..10.0, b in 0.0f64..10.0) {
///     // Called with just a condition will print the condition on failure
///     prop_assert!((a*a + b*b).sqrt() <= a + b);
///     // You can also provide a custom failure message
///     prop_assert!((a*a + b*b).sqrt() <= a + b,
///                  "Triangle inequality didn't hold for ({}, {})", a, b);
///     // If calling another function that can return failure, don't forget
///     // the `?` to propagate the failure.
///     assert_from_other_function(a, b)?;
///   }
/// }
///
/// // The macro can be used from another function provided it has a compatible
/// // return type.
/// fn assert_from_other_function(a: f64, b: f64) -> Result<(), TestCaseError> {
///   prop_assert!((a*a + b*b).sqrt() <= a + b);
///   Ok(())
/// }
/// #
/// # fn main() { triangle_inequality(); }
/// ```
#[macro_export]
macro_rules! prop_assert {
    ($cond:expr) => {
        prop_assert!($cond, concat!("assertion failed: ", stringify!($cond)))
    };

    ($cond:expr, $($fmt:tt)*) => {
        if !$cond {
            let message = format!($($fmt)*);
            let message = format!("{} at {}:{}", message, file!(), line!());
            return $crate::test_runner::fail_case(message);
        }
    };
}

/// Similar to `assert_eq!` from std, but returns a test failure instead of
/// panicking if the condition fails.
///
/// See `prop_assert!` for a more in-depth discussion.
///
/// ## Example
///
/// ```
/// #[macro_use] extern crate proptest;
///
/// proptest! {
///   # /*
///   #[test]
///   # */
///   fn concat_string_length(ref a in ".*", ref b in ".*") {
///     let cat = format!("{}{}", a, b);
///     // Use with default message
///     prop_assert_eq!(a.len() + b.len(), cat.len());
///     // Can also provide custom message (added after the normal
///     // assertion message)
///     prop_assert_eq!(a.len() + b.len(), cat.len(),
///                     "a = {:?}, b = {:?}", a, b);
///   }
/// }
/// #
/// # fn main() { concat_string_length(); }
/// ```
#[macro_export]
macro_rules! prop_assert_eq {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let right = $right;
        prop_assert!(left == right, "assertion failed: `(left == right)` \
                                     (left: `{:?}`, right: `{:?}`)",
                     left, right);
    }};

    ($left:expr, $right:expr, $fmt:tt $($args:tt)*) => {{
        let left = $left;
        let right = $right;
        prop_assert!(left == right, concat!(
            "assertion failed: `(left == right)` \
             (left: `{:?}`, right: `{:?}`): ", $fmt),
                     left, right $($args)*);
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! proptest_helper {
    (@_WRAP ($a:tt)) => { $a };
    (@_WRAP ($a0:tt $a1:tt)) => { ($a0, $a1) };
    (@_WRAP ($a0:tt $a1:tt $a2:tt)) => { ($a0, $a1, $a2) };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt)) => { ($a0, $a1, $a2, $a3) };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt $a4:tt)) => {
        ($a0, $a1, $a2, $a3, $a4)
    };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt $a4:tt $a5:tt)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5)
    };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt $a4:tt $a5:tt $a6:tt)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6)
    };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt
             $a4:tt $a5:tt $a6:tt $a7:tt)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7)
    };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt $a4:tt
             $a5:tt $a6:tt $a7:tt $a8:tt)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8)
    };
    (@_WRAP ($a0:tt $a1:tt $a2:tt $a3:tt $a4:tt
             $a5:tt $a6:tt $a7:tt $a8:tt $a9:tt)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8, $a9)
    };
    (@_WRAP ($a:tt $($rest:tt)*)) => {
        ($a, proptest_helper!(@_WRAP ($($rest)*)))
    };
    (@_WRAPPAT ($item:pat)) => { $item };
    (@_WRAPPAT ($a0:pat, $a1:pat)) => { ($a0, $a1) };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat)) => { ($a0, $a1, $a2) };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat)) => {
        ($a0, $a1, $a2, $a3)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat)) => {
        ($a0, $a1, $a2, $a3, $a4)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat, $a5:pat)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat,
                $a4:pat, $a5:pat, $a6:pat)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat,
                $a4:pat, $a5:pat, $a6:pat, $a7:pat)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat,
                $a5:pat, $a6:pat, $a7:pat, $a8:pat)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8)
    };
    (@_WRAPPAT ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat,
                $a5:pat, $a6:pat, $a7:pat, $a8:pat, $a9:pat)) => {
        ($a0, $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8, $a9)
    };
    (@_WRAPPAT ($a:pat, $($rest:pat),*)) => {
        ($a, proptest_helper!(@_WRAPPAT ($($rest),*)))
    };
    (@_WRAPSTR ($item:pat)) => { stringify!($item) };
    (@_WRAPSTR ($a0:pat, $a1:pat)) => { (stringify!($a0), stringify!($a1)) };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2),
         stringify!($a3), stringify!($a4))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat, $a5:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3),
         stringify!($a4), stringify!($a5))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat,
                $a4:pat, $a5:pat, $a6:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3),
         stringify!($a4), stringify!($a5), stringify!($a6))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat,
                $a4:pat, $a5:pat, $a6:pat, $a7:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3),
         stringify!($a4), stringify!($a5), stringify!($a6), stringify!($a7))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat,
                $a5:pat, $a6:pat, $a7:pat, $a8:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3),
         stringify!($a4), stringify!($a5), stringify!($a6), stringify!($a7),
         stringify!($a8))
    };
    (@_WRAPSTR ($a0:pat, $a1:pat, $a2:pat, $a3:pat, $a4:pat,
                $a5:pat, $a6:pat, $a7:pat, $a8:pat, $a9:pat)) => {
        (stringify!($a0), stringify!($a1), stringify!($a2), stringify!($a3),
         stringify!($a4), stringify!($a5), stringify!($a6), stringify!($a7),
         stringify!($a8), stringify!($a9))
    };
    (@_WRAPSTR ($a:pat, $($rest:pat),*)) => {
        (stringify!($a), proptest_helper!(@_WRAPSTR ($($rest),*)))
    };
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct NamedArguments<N, V>(
    #[doc(hidden)] pub N, #[doc(hidden)] pub V);

impl<V : fmt::Debug> fmt::Debug for NamedArguments<&'static str, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} = ", self.0)?;
        self.1.fmt(f)
    }
}

macro_rules! named_arguments_tuple {
    ($($ix:tt $argn:ident $argv:ident)*) => {
        impl<'a, $($argn : Copy),*, $($argv),*> fmt::Debug
        for NamedArguments<($($argn,)*),&'a ($($argv,)*)>
        where $(NamedArguments<$argn, &'a $argv> : fmt::Debug),*,
              $($argv : 'a),*
        {
            #[allow(unused_assignments)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut first = true;
                $(
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    fmt::Debug::fmt(
                        &NamedArguments((self.0).$ix, &(self.1).$ix), f)?;
                )*
                Ok(())
            }
        }

        impl<$($argn : Copy),*, $($argv),*> fmt::Debug
        for NamedArguments<($($argn,)*), ($($argv,)*)>
        where $(for<'a> NamedArguments<$argn, &'a $argv> : fmt::Debug),*
        {
            #[allow(unused_assignments)]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut first = true;
                $(
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    fmt::Debug::fmt(
                        &NamedArguments((self.0).$ix, &(self.1).$ix), f)?;
                )*
                Ok(())
            }
        }
    }
}

named_arguments_tuple!(0 AN AV);
named_arguments_tuple!(0 AN AV 1 BN BV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV
                       5 FN FV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV
                       5 FN FV 6 GN GV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV
                       5 FN FV 6 GN GV 7 HN HV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV
                       5 FN FV 6 GN GV 7 HN HV 8 IN IV);
named_arguments_tuple!(0 AN AV 1 BN BV 2 CN CV 3 DN DV 4 EN EV
                       5 FN FV 6 GN GV 7 HN HV 8 IN IV 9 JN JV);

/// Similar to `assert_ne!` from std, but returns a test failure instead of
/// panicking if the condition fails.
///
/// See `prop_assert!` for a more in-depth discussion.
///
/// ## Example
///
/// ```
/// #[macro_use] extern crate proptest;
///
/// proptest! {
///   # /*
///   #[test]
///   # */
///   fn test_addition(a in 0i32..100i32, b in 1i32..100i32) {
///     // Use with default message
///     prop_assert_ne!(a, a + b);
///     // Can also provide custom message added after the common message
///     prop_assert_ne!(a, a + b, "a = {}, b = {}", a, b);
///   }
/// }
/// #
/// # fn main() { test_addition(); }
/// ```
#[macro_export]
macro_rules! prop_assert_ne {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let right = $right;
        prop_assert!(left != right, "assertion failed: `(left != right)` \
                                     (left: `{:?}`, right: `{:?}`)",
                     left, right);
    }};

    ($left:expr, $right:expr, $fmt:tt $($args:tt)*) => {{
        let left = $left;
        let right = $right;
        prop_assert!(left != right, concat!(
            "assertion failed: `(left != right)` \
             (left: `{:?}`, right: `{:?}`): ", $fmt),
                     left, right $($args)*);
    }};
}

#[cfg(test)]
mod test {
    use ::strategy::Just;

    prop_compose! {
        /// These are docs!
        #[allow(dead_code)]
        fn two_ints(relative: i32)(a in 0..relative, b in relative..)
                   -> (i32, i32) {
            (a, b)
        }
    }

    prop_compose! {
        /// These are docs!
        #[allow(dead_code)]
        [pub] fn two_ints_pub(relative: i32)(a in 0..relative, b in relative..)
                             -> (i32, i32) {
            (a, b)
        }
    }

    prop_compose! {
        #[allow(dead_code)]
        fn a_less_than_b()(b in 0..1000)(a in 0..b, b in Just(b))
                        -> (i32, i32) {
            (a, b)
        }
    }

    proptest! {
        #[test]
        fn test_something(a in 0u32..42u32, b in 1u32..10u32) {
            prop_assume!(a != 41 || b != 9);
            assert!(a + b < 50);
        }
    }

    #[allow(unused_variables)]
    mod test_arg_counts {
        use strategy::Just;

        proptest! {
            #[test]
            fn test_1_arg(a in Just(0)) { }
            #[test]
            fn test_2_arg(a in Just(0), b in Just(0)) { }
            #[test]
            fn test_3_arg(a in Just(0), b in Just(0), c in Just(0)) { }
            #[test]
            fn test_4_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0)) { }
            #[test]
            fn test_5_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0)) { }
            #[test]
            fn test_6_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0)) { }
            #[test]
            fn test_7_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0)) { }
            #[test]
            fn test_8_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0), h in Just(0)) { }
            #[test]
            fn test_9_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0), h in Just(0), i in Just(0)) { }
            #[test]
            fn test_a_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0), h in Just(0), i in Just(0),
                          j in Just(0)) { }
            #[test]
            fn test_b_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0), h in Just(0), i in Just(0),
                          j in Just(0), k in Just(0)) { }
            #[test]
            fn test_c_arg(a in Just(0), b in Just(0), c in Just(0),
                          d in Just(0), e in Just(0), f in Just(0),
                          g in Just(0), h in Just(0), i in Just(0),
                          j in Just(0), k in Just(0), l in Just(0)) { }
        }
    }

    #[test]
    fn named_arguments_is_debug_for_needed_cases() {
        use super::NamedArguments;

        println!("{:?}", NamedArguments("foo", &"bar"));
        println!("{:?}", NamedArguments(("foo",), &(1,)));
        println!("{:?}", NamedArguments(("foo","bar"), &(1,2)));
        println!("{:?}", NamedArguments(("a","b","c"), &(1,2,3)));
        println!("{:?}", NamedArguments(("a","b","c","d"), &(1,2,3,4)));
        println!("{:?}", NamedArguments(("a","b","c","d","e"),
                                        &(1,2,3,4,5)));
        println!("{:?}", NamedArguments(("a","b","c","d","e","f"),
                                        &(1,2,3,4,5,6)));
        println!("{:?}", NamedArguments(("a","b","c","d","e","f","g"),
                                        &(1,2,3,4,5,6,7)));
        println!("{:?}", NamedArguments(("a","b","c","d","e","f","g","h"),
                                        &(1,2,3,4,5,6,7,8)));
        println!("{:?}", NamedArguments(("a","b","c","d","e","f","g","h","i"),
                                        &(1,2,3,4,5,6,7,8,9)));
        println!("{:?}", NamedArguments(("a","b","c","d","e","f","g","h","i","j"),
                                        &(1,2,3,4,5,6,7,8,9,10)));
        println!("{:?}", NamedArguments((("a","b"),"c","d"), &((1,2),3,4)));
    }

    #[test]
    fn oneof_all_counts() {
        fn expect_count<S : ::strategy::Strategy>(n: usize, s: S)
        where S::Value : ::strategy::ValueTree<Value = i32> {
            use std::collections::HashSet;
            use strategy::*;
            use test_runner::*;

            let mut runner = TestRunner::default();
            let mut seen = HashSet::new();
            for _ in 0..1024 {
                seen.insert(s.new_value(&mut runner).unwrap().current());
            }

            assert_eq!(n, seen.len());
        }

        fn assert_static<T>(v: ::strategy::TupleUnion<T>)
                            -> ::strategy::TupleUnion<T>
        { v }

        fn assert_dynamic<T : ::strategy::Strategy>
            (v: ::strategy::Union<T>) -> ::strategy::Union<T>
        { v }

        use strategy::Just as J;
        expect_count(1, prop_oneof![J(0i32)]);
        expect_count(2, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
        ]));
        expect_count(3, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
        ]));
        expect_count(4, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
        ]));
        expect_count(5, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
        ]));
        expect_count(6, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
        ]));
        expect_count(7, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
            J(6i32),
        ]));
        expect_count(8, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
            J(6i32),
            J(7i32),
        ]));
        expect_count(9, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
            J(6i32),
            J(7i32),
            J(8i32),
        ]));
        expect_count(10, assert_static(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
            J(6i32),
            J(7i32),
            J(8i32),
            J(9i32),
        ]));
        expect_count(11, assert_dynamic(prop_oneof![
            J(0i32),
            J(1i32),
            J(2i32),
            J(3i32),
            J(4i32),
            J(5i32),
            J(6i32),
            J(7i32),
            J(8i32),
            J(9i32),
            J(10i32),
        ]));
    }
}
