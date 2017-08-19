//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

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
///     assert!(a + b <= 18);
///   }
///
///   // Note the `ref a` and `ref b` --- `String` is not `Copy`,
///   // so we can't take ownership implicitly.
///   # /*
///   #[test]
///   # */
///   fn test_string_concat(ref a in ".*", ref b in ".*") {
///     let cat = format!("{}{}", a, b);
///     assert_eq!(a.len() + b.len(), cat.len());
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
/// use proptest::test_runner::Config;
///
/// proptest! {
///   #![proptest_config(Config { cases: 99, .. Config::default() })]
///   # /*
///   #[test]
///   # */
///   fn test_addition(a in 0..10, b in 0..10) {
///     assert!(a + b <= 18);
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
                match runner.run(
                    &proptest!(@_WRAP ($($strategy)*)),
                    |&proptest!(@_WRAPPAT ($($parm),*))| {
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
        ($a, proptest!(@_WRAP ($($rest)*)))
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
        ($a, proptest!(@_WRAPPAT ($($rest),*)))
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
            return Err($crate::test_runner::TestCaseError::Reject(
                format!(concat!("{}:{}:{}: ", $fmt),
                        file!(), line!(), column!()
                        $(, $fmt_arg)*)));
        }
    };
}

/// Produce a strategy which picks one of the listed choices.
///
/// This is equivalent to calling `prop_union` on the first two elements and
/// then chaining `.or()` onto the rest after implicitly boxing all of them. As
/// with `Union`, values shrink across elements on the assumption that earlier
/// ones are "simpler", so they should be listed in order of ascending
/// complexity when possible.
///
/// ## Example
///
/// ```rust,no_run
/// #[macro_use] extern crate proptest;
/// use proptest::strategy::Strategy;
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
///   proptest::num::i16::ANY.prop_map(MyEnum::Little),
///   proptest::num::u32::ANY.prop_map(MyEnum::Medium),
///   proptest::num::u64::ANY.prop_map(MyEnum::Big),
/// ];
/// # }
/// ```
#[macro_export]
macro_rules! prop_oneof {
    ($($item:expr),+ $(,)*) => {
        $crate::strategy::Union::new(vec![
            $($crate::strategy::Strategy::boxed($item)),*
        ])
    }
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
/// use proptest::strategy::Just;
///
/// prop_compose! {
///   fn vec_and_index
///     (max_length: usize)
///     (vec in proptest::collection::vec(1..10, 1..max_length))
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
///
/// prop_compose! {
///   [pub(crate) unsafe] fn pointer()(v in proptest::num::usize::ANY)
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
            let strat = proptest!(@_WRAP ($($strategy)*));
            let strat = $crate::strategy::Strategy::prop_map(
                strat,
                |proptest!(@_WRAPPAT ($($var),*))| $body);
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
            let strat = proptest!(@_WRAP ($($strategy)*));
            let strat = $crate::strategy::Strategy::prop_flat_map(
                strat,
                |proptest!(@_WRAPPAT ($($var),*))|
                proptest!(@_WRAP ($($strategy2)*)));
            let strat = $crate::strategy::Strategy::prop_map(
                strat,
                |proptest!(@_WRAPPAT ($($var2),*))| $body);
            $crate::strategy::Strategy::boxed(strat)
        }
    };
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
}
