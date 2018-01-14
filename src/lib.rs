//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature="cargo-clippy", allow(doc_markdown))]

//! Proptest is a property testing framework (i.e., the QuickCheck family)
//! inspired by the [Hypothesis](http://hypothesis.works/) framework for
//! Python. It allows to test that certain properties of your code hold for
//! arbitrary inputs, and if a failure is found, automatically finds the
//! minimal test case to reproduce the problem. Unlike QuickCheck, generation
//! and shrinking is defined on a per-value basis instead of per-type, which
//! makes it much more flexible and simplifies composition.
//!
//! If you have dependencies which provide QuickCheck `Arbitrary`
//! implementations, see also the related
//! [`proptest-quickcheck-interop`](https://crates.io/crates/proptest-quickcheck-interop)
//! crates which enables reusing those implementations with proptest.
//!
//! <!-- NOREADME
//! ## Status of this crate
//!
//! The majority of the functionality offered by proptest is in active use and
//! is known to work well.
//!
//! The API is unlikely to see drastic breaking changes, but there may still be
//! minor breaking changes here and there, particularly when "impl Trait"
//! becomes stable and after the upcoming redesign of the `rand` crate.
//!
//! See the [changelog](https://github.com/AltSysrq/proptest/blob/master/CHANGELOG.md)
//! for a full list of substantial historical changes, breaking and otherwise.
//! NOREADME -->
//!
//! ## Introduction
//!
//! _Property testing_ is a system of testing code by checking that certain
//! properties of its output or behaviour are fulfilled for all inputs. These
//! inputs are generated automatically, and, critically, when a failing input
//! is found, the input is automatically reduced to a _minimal_ test case.
//!
//! Property testing is best used to compliment traditional unit testing (i.e.,
//! using specific inputs chosen by hand). Traditional tests can test specific
//! known edge cases, simple inputs, and inputs that were known in the past to
//! reveal bugs, whereas property tests will search for more complicated inputs
//! that cause problems.
//!
//! ## Getting Started
//!
//! Let's say we want to make a function that parses dates of the form
//! `YYYY-MM-DD`. We're not going to worry about _validating_ the date, any
//! triple of integers is fine. So let's bang something out real quick.
//!
//! ```rust,no_run
//! fn parse_date(s: &str) -> Option<(u32, u32, u32)> {
//!     if 10 != s.len() { return None; }
//!     if "-" != &s[4..5] || "-" != &s[7..8] { return None; }
//!
//!     let year = &s[0..4];
//!     let month = &s[6..7];
//!     let day = &s[8..10];
//!
//!     year.parse::<u32>().ok().and_then(
//!         |y| month.parse::<u32>().ok().and_then(
//!             |m| day.parse::<u32>().ok().map(
//!                 |d| (y, m, d))))
//! }
//! ```
//!
//! It compiles, that means it works, right? Maybe not, let's add some tests.
//!
//! ```rust,ignore
//! #[test]
//! fn test_parse_date() {
//!     assert_eq!(None, parse_date("2017-06-1"));
//!     assert_eq!(None, parse_date("2017-06-170"));
//!     assert_eq!(None, parse_date("2017006-17"));
//!     assert_eq!(None, parse_date("2017-06017"));
//!     assert_eq!(Some((2017, 06, 17)), parse_date("2017-06-17"));
//! }
//! ```
//!
//! Tests pass, deploy to production! But now your application starts crashing,
//! and people are upset that you moved Christmas to February. Maybe we need to
//! be a bit more thorough.
//!
//! In `Cargo.toml`, add
//!
//! ```toml
//! [dev-dependencies]
//! proptest = "0.3.4"
//! ```
//!
//! and at the top of `main.rs` or `lib.rs`:
//!
//! ```rust,ignore
//! #[macro_use] extern crate proptest;
//! ```
//!
//! Now we can add some property tests to our date parser. But how do we test
//! the date parser for arbitrary inputs, without making another date parser in
//! the test to validate it? We won't need to as long as we choose our inputs
//! and properties correctly. But before correctness, there's actually an even
//! simpler property to test: _The function should not crash._ Let's start
//! there.
//!
//! ```rust,ignore
//! proptest! {
//!     #[test]
//!     fn doesnt_crash(ref s in "\\PC*") {
//!         parse_date(s);
//!     }
//! }
//! ```
//!
//! What this does is take a literally random `&String` (ignore `\\PC*` for the
//! moment, we'll get back to that — if you've already figured it out, contain
//! your excitement for a bit) and give it to `parse_date()` and then throw the
//! output away.
//!
//! When we run this, we get a bunch of scary-looking output, eventually ending
//! with
//!
//! ```text
//! thread 'main' panicked at 'Test failed: byte index 4 is not a char boundary; it is inside 'ௗ' (bytes 2..5) of `aAௗ0㌀0`; minimal failing input: ref s = "aAௗ0㌀0"
//! 	successes: 102
//! 	local rejects: 0
//! 	global rejects: 0
//! '
//! ```
//!
//! If we look at the top directory after the test fails, we'll see a new
//! `proptest-regressions` directory, which contains some files corresponding
//! to source files containing failing test cases. These are [_failure
//! persistence_](#failure-persistence) files. The first thing we should do is
//! add these to source control.
//!
//! ```text
//! $ git add proptest-regressions
//! ```
//!
//! The next thing we should do is copy the failing case to a traditional unit
//! test since it has exposed a bug not similar to what we've tested in the
//! past.
//!
//! ```rust,ignore
//! #[test]
//! fn test_unicode_gibberish() {
//!     assert_eq!(None, parse_date("aAௗ0㌀0"));
//! }
//! ```
//!
//! Now, let's see what happened... we forgot about UTF-8! You can't just
//! blindly slice strings since you could split a character, in this case that
//! Tamil diacritic placed atop other characters in the string.
//!
//! In the interest of making the code changes as small as possible, we'll just
//! check that the string is ASCII and reject anything that isn't.
//!
//! ```rust,no_run
//! # use std::ascii::AsciiExt; //NOREADME
//! # // NOREADME
//! fn parse_date(s: &str) -> Option<(u32, u32, u32)> {
//!     if 10 != s.len() { return None; }
//!
//!     // NEW: Ignore non-ASCII strings so we don't need to deal with Unicode.
//!     if !s.is_ascii() { return None; }
//!
//!     if "-" != &s[4..5] || "-" != &s[7..8] { return None; }
//!
//!     let year = &s[0..4];
//!     let month = &s[6..7];
//!     let day = &s[8..10];
//!
//!     year.parse::<u32>().ok().and_then(
//!         |y| month.parse::<u32>().ok().and_then(
//!             |m| day.parse::<u32>().ok().map(
//!                 |d| (y, m, d))))
//! }
//! ```
//!
//! The tests pass now! But we know there are still more problems, so let's
//! test more properties.
//!
//! Another property we want from our code is that it parses every valid date.
//! We can add another test to the `proptest!` section:
//!
//! ```rust,ignore
//! proptest! {
//!     // snip...
//!
//!     #[test]
//!     fn parses_all_valid_dates(ref s in "[0-9]{4}-[0-9]{2}-[0-9]{2}") {
//!         parse_date(s).unwrap();
//!     }
//! }
//! ```
//!
//! The thing to the right-hand side of `in` is actually a *regular
//! expression*, and `s` is chosen from strings which match it. So in our
//! previous test, `"\\PC*"` was generating arbitrary strings composed of
//! arbitrary non-control characters. Now, we generate things in the YYYY-MM-DD
//! format.
//!
//! The new test passes, so let's move on to something else.
//!
//! The final property we want to check is that the dates are actually parsed
//! _correctly_. Now, we can't do this by generating strings — we'd end up just
//! reimplementing the date parser in the test! Instead, we start from the
//! expected output, generate the string, and check that it gets parsed back.
//!
//! ```rust,ignore
//! proptest! {
//!     // snip...
//!
//!     #[test]
//!     fn parses_date_back_to_original(y in 0u32..10000,
//!                                     m in 1u32..13, d in 1u32..32) {
//!         let (y2, m2, d2) = parse_date(
//!             &format!("{:04}-{:02}-{:02}", y, m, d)).unwrap();
//!         // prop_assert_eq! is basically the same as assert_eq!, but doesn't
//!         // cause a bunch of panic messages to be printed on intermediate
//!         // test failures. Which one to use is largely a matter of taste.
//!         prop_assert_eq!((y, m, d), (y2, m2, d2));
//!     }
//! }
//! ```
//!
//! Here, we see that besides regexes, we can use any expression which is a
//! `proptest::strategy::Strategy`, in this case, integer ranges.
//!
//! The test fails when we run it. Though there's not much output this time.
//!
//! ```text
//! thread 'main' panicked at 'Test failed: assertion failed: `(left == right)` (left: `(0, 10, 1)`, right: `(0, 0, 1)`) at examples/dateparser_v2.rs:46; minimal failing input: y = 0, m = 10, d = 1
//! 	successes: 2
//! 	local rejects: 0
//! 	global rejects: 0
//! ', examples/dateparser_v2.rs:33
//! note: Run with `RUST_BACKTRACE=1` for a backtrace.
//! ```
//!
//! The failing input is `(y, m, d) = (0, 10, 1)`, which is a rather specific
//! output. Before thinking about why this breaks the code, let's look at what
//! proptest did to arrive at this value. At the start of our test function,
//! insert
//!
//! ```rust,ignore
//!     println!("y = {}, m = {}, d = {}", y, m, d);
//! ```
//!
//! Running the test again, we get something like this:
//!
//! ```text
//! y = 2497, m = 8, d = 27
//! y = 9641, m = 8, d = 18
//! y = 7360, m = 12, d = 20
//! y = 3680, m = 12, d = 20
//! y = 1840, m = 12, d = 20
//! y = 920, m = 12, d = 20
//! y = 460, m = 12, d = 20
//! y = 230, m = 12, d = 20
//! y = 115, m = 12, d = 20
//! y = 57, m = 12, d = 20
//! y = 28, m = 12, d = 20
//! y = 14, m = 12, d = 20
//! y = 7, m = 12, d = 20
//! y = 3, m = 12, d = 20
//! y = 1, m = 12, d = 20
//! y = 0, m = 12, d = 20
//! y = 0, m = 6, d = 20
//! y = 0, m = 9, d = 20
//! y = 0, m = 11, d = 20
//! y = 0, m = 10, d = 20
//! y = 0, m = 10, d = 10
//! y = 0, m = 10, d = 5
//! y = 0, m = 10, d = 3
//! y = 0, m = 10, d = 2
//! y = 0, m = 10, d = 1
//! ```
//!
//! The test failure message said there were two successful cases; we see these
//! at the very top, `2497-08-27` and `9641-08-18`. The next case,
//! `7360-12-20`, failed. There's nothing immediately obviously special about
//! this date. Fortunately, proptest reduced it to a much simpler case. First,
//! it rapidly reduced the `y` input to `0` at the beginning, and similarly
//! reduced the `d` input to the minimum allowable value of `1` at the end.
//! Between those two, though, we see something different: it tried to shrink
//! `12` to `6`, but then ended up raising it back up to `10`. This is because
//! the `0000-06-20` and `0000-09-20` test cases _passed_.
//!
//! In the end, we get the date `0000-10-01`, which apparently gets parsed as
//! `0000-00-01`. Again, this failing case was added to the failure persistence
//! file, and we should add this as its own unit test:
//!
//! ```text
//! $ git add proptest-regressions
//! ```
//!
//! ```rust,ignore
//! #[test]
//! fn test_october_first() {
//!     assert_eq!(Some(0, 10, 1), parse_date("0000-10-01"));
//! }
//! ```
//!
//! Now to figure out what's broken in the code. Even without the intermediate
//! input, we can say with reasonable confidence that the year and day parts
//! don't come into the picture since both were reduced to the minimum
//! allowable input. The month input was _not_, but was reduced to `10`. This
//! means we can infer that there's something special about `10` that doesn't
//! hold for `9`. In this case, that "special something" is being two digits
//! wide. In our code:
//!
//! ```rust,ignore
//!     let month = &s[6..7];
//! ```
//!
//! We were off by one, and need to use the range `5..7`. After fixing this,
//! the test passes.
//!
//! The `proptest!` macro has some additional syntax, including for setting
//! configuration for things like the number of test cases to generate. See its
//! [documentation](macro.proptest.html) <!-- NOREADME
//! [documentation](https://docs.rs/proptest/*/proptest/macro.proptest.html)
//! NOREADME -->
//! for more details.
//!
//! There is a more in-depth tutorial
//! [further down](#in-depth-tutorial). <!-- NOREADME
//! [in the crate documentation](https://docs.rs/proptest/#in-depth-tutorial).
//! NOREADME -->
//!
//! ## Differences between QuickCheck and Proptest
//!
//! QuickCheck and Proptest are similar in many ways: both generate random
//! inputs for a function to check certain properties, and automatically shrink
//! inputs to minimal failing cases.
//!
//! The one big difference is that QuickCheck generates and shrinks values
//! based on type alone, whereas Proptest uses explicit `Strategy` objects. The
//! QuickCheck approach has a lot of disadvantages in comparison:
//!
//! - QuickCheck can only define one generator and shrinker per type. If you
//! need a custom generation strategy, you need to wrap it in a newtype and
//! implement traits on that by hand. In Proptest, you can define arbitrarily
//! many different strategies for the same type, and there are plenty built-in.
//!
//! - For the same reason, QuickCheck has a single "size" configuration that
//! tries to define the range of values generated. If you need an integer
//! between 0 and 100 and another between 0 and 1000, you probably need to do
//! another newtype. In Proptest, you can directly just express that you want a
//! `0..100` integer and a `0..1000` integer.
//!
//! - Types in QuickCheck are not easily composable. Defining `Arbitrary` and
//! `Shrink` for a new struct which is simply produced by the composition of
//! its fields requires implementing both by hand, including a bidirectional
//! mapping between the struct and a tuple of its fields. In Proptest, you can
//! make a tuple of the desired components and then `prop_map` it into the
//! desired form. Shrinking happens automatically in terms of the input types.
//!
//! - Because constraints on values cannot be expressed in QuickCheck,
//! generation and shrinking may lead to a lot of input rejections. Strategies
//! in Proptest are aware of simple constraints and do not generate or shrink
//! to values that violate them.
//!
//! The author of Hypothesis also has an [article on this
//! topic](http://hypothesis.works/articles/integrated-shrinking/).
//!
//! Of course, there's also some relative downsides that fall out of what
//! Proptest does differently:
//!
//! - Generating complex values in Proptest can be up to an order of magnitude
//! slower than in QuickCheck. This is because QuickCheck performs stateless
//! shrinking based on the output value, whereas Proptest must hold on to all
//! the intermediate states and relationships in order for its richer shrinking
//! model to work.
//!
//! - In cases where one usually does have a single canonical way to generate
//! values per type, Proptest will be more verbose than QuickCheck since one
//! needs to name the strategy every time rather than getting them implicitly
//! based on types.
//!
//! ## Limitations of Property Testing
//!
//! Given infinite time, property testing will eventually explore the whole
//! input space to a test. However, time is not infinite, so only a randomly
//! sampled portion of the input space can be explored. This means that
//! property testing is extremely unlikely to find single-value edge cases in a
//! large space. For example, the following test will virtually always pass:
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! proptest! {
//!     # /* NOREADME
//!     #[test]
//!     # NOREADME */
//!     fn i64_abs_is_never_negative(a in prop::num::i64::ANY) {
//!         assert!(a.abs() >= 0);
//!     }
//! }
//! # // NOREADME
//! # fn main() { i64_abs_is_never_negative(); } // NOREADME
//! ```
//!
//! Because of this, traditional unit testing with intelligently selected cases
//! is still necessary for many kinds of problems.
//!
//! Similarly, in some cases it can be hard or impossible to define a strategy
//! which actually produces useful inputs. A strategy of `.{1,4096}` may be
//! great to fuzz a C parser, but is highly unlikely to produce anything that
//! makes it to a code generator.
//!
//! ## Failure Persistence
//!
//! By default, when Proptest finds a failing test case, it _persists_ that
//! failing case in a file named after the source containing the failing test,
//! but in a separate directory tree rooted at `proptest-regressions`† . Later
//! runs of tests will replay those test cases before generating novel cases.
//! This ensures that the test will not fail on one run and then spuriously
//! pass on the next, and also exposes similar tests to the same
//! known-problematic input.
//!
//! (†  If you do not have an obvious source directory, you may instead find
//! files next to the source files, with a different extension.)
//!
//! It is recommended to check these files in to your source control so that
//! other test runners (e.g., collaborators or a CI system) also replay these
//! cases.
//!
//! Note that, by default, all tests in the same crate will share that one
//! persistence file. If you have a very large number of tests, it may be
//! desirable to separate them into smaller groups so the number of extra test
//! cases that get run is reduced. This can be done by adjusting the
//! `failure_persistence` flag on `Config`.
//!
//! There are two ways this persistence could theoretically be done.
//!
//! The immediately obvious option is to persist a representation of the value
//! itself, for example by using Serde. While this has some advantages,
//! particularly being resistant to changes like tweaking the input strategy,
//! it also has a lot of problems. Most importantly, there is no way to
//! determine whether any given value is actually within the domain of the
//! strategy that produces it. Thus, some (likely extremely fragile) mechanism
//! to ensure that the strategy that produced the value exactly matches the one
//! in use in a test case would be required.
//!
//! The other option is to store the _seed_ that was used to produce the
//! failing test case. This approach requires no support from the strategy or
//! the produced value. If the strategy in use differs from the one used to
//! produce failing case that was persisted, the seed may or may not produce
//! the problematic value, but nonetheless produces a valid value. Due to these
//! advantages, this is the approach Proptest uses.
//!
//! <!-- ENDREADME -->
//!
//! ## In-Depth Tutorial
//!
//! This tutorial will introduce proptest from the bottom up, starting from the
//! basic building blocks, in the hopes of making the model as a whole clear.
//! In particular, we'll start off without using the macros so that the macros
//! can later be understood in terms of what they expand into rather than
//! magic. But as a result, the first part is _not_ representative of how
//! proptest is normally used. If bottom-up isn't your style, you may wish to
//! skim the first few sections.
//!
//! Also note that the examples here focus on the usage of proptest itself, and
//! as such generally have trivial test bodies. In real code, you would
//! obviously have assertions and so forth in the test bodies.
//!
//! ### Strategy Basics
//!
//! The [_Strategy_](strategy/trait.Strategy.html) is the most fundamental
//! concept in proptest. A strategy defines two things:
//!
//! - How to generate random values of a particular type from a random number
//! generator.
//!
//! - How to "shrink" such values into "simpler" forms.
//!
//! Proptest ships with a substantial library of strategies. Some of these are
//! defined in terms of built-in types; for example, `0..100i32` is a strategy
//! to generate `i32`s between 0, inclusive, and 100, exclusive. As we've
//! already seen, strings are themselves strategies for generating strings
//! which match the former as a regular expression.
//!
//! Generating a value is a two-step process. First, a `TestRunner` is passed
//! to the `new_value()` method of the `Strategy`; this returns a `ValueTree`,
//! which we'll look at in more detail momentarily. Calling the `current()`
//! method on the `ValueTree` produces the actual value. Knowing that, we can
//! put the pieces together and generate values. The below is the
//! `tutoral-strategy-play.rs` example:
//!
//! ```rust
//! extern crate proptest;
//!
//! use proptest::test_runner::TestRunner;
//! use proptest::strategy::{Strategy, ValueTree};
//!
//! fn main() {
//!     let mut runner = TestRunner::default();
//!     let int_val = (0..100i32).new_value(&mut runner).unwrap();
//!     let str_val = "[a-z]{1,4}\\p{Cyrillic}{1,4}\\p{Greek}{1,4}"
//!         .new_value(&mut runner).unwrap();
//!     println!("int_val = {}, str_val = {}",
//!              int_val.current(), str_val.current());
//! }
//! ```
//!
//! If you run this a few times, you'll get output similar to the following:
//!
//! ```text
//! $ target/debug/examples/tutorial-strategy-play
//! int_val = 99, str_val = vѨͿἕΌ
//! $ target/debug/examples/tutorial-strategy-play
//! int_val = 25, str_val = cwᵸійΉ
//! $ target/debug/examples/tutorial-strategy-play
//! int_val = 5, str_val = oegiᴫᵸӈᵸὛΉ
//! ```
//!
//! This knowledge is sufficient to build an extremely primitive fuzzing test.
//!
//! ```rust,no_run
//! extern crate proptest;
//!
//! use proptest::test_runner::TestRunner;
//! use proptest::strategy::{Strategy, ValueTree};
//!
//! fn some_function(v: i32) {
//!     // Do a bunch of stuff, but crash if v > 500
//!     assert!(v <= 500);
//! }
//!
//! # /*
//! #[test]
//! # */
//! fn some_function_doesnt_crash() {
//!     let mut runner = TestRunner::default();
//!     for _ in 0..256 {
//!         let val = (0..10000i32).new_value(&mut runner).unwrap();
//!         some_function(val.current());
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! This _works_, but when the test fails, we don't get much context, and even
//! if we recover the input, we see some arbitrary-looking value like 1771
//! rather than the boundary condition of 501. For a function taking just an
//! integer, this is probably still good enough, but as inputs get more
//! complex, interpreting completely random values becomes increasingly
//! difficult.
//!
//! ### Shrinking Basics
//!
//! Finding the "simplest" input that causes a test failure is referred to as
//! _shrinking_. This is where the intermediate `ValueTree` type comes in.
//! Besides `current()`, it provides two methods — `simplify()` and
//! `complicate()` — which together allow binary searching over the input
//! space. The `tutorial-simplify-play.rs` example shows how repeated calls to
//! `simplify()` produce incrementally "simpler" outputs, both in terms of size
//! and in characters used.
//!
//! ```rust
//! extern crate proptest;
//!
//! use proptest::test_runner::TestRunner;
//! use proptest::strategy::{Strategy, ValueTree};
//!
//! fn main() {
//!     let mut runner = TestRunner::default();
//!     let mut str_val = "[a-z]{1,4}\\p{Cyrillic}{1,4}\\p{Greek}{1,4}"
//!         .new_value(&mut runner).unwrap();
//!     println!("str_val = {}", str_val.current());
//!     while str_val.simplify() {
//!         println!("        = {}", str_val.current());
//!     }
//! }
//! ```
//!
//! A couple runs:
//!
//! ```text
//! $ target/debug/examples/tutorial-simplify-play
//! str_val = vy꙲ꙈᴫѱΆῨῨ
//!         = y꙲ꙈᴫѱΆῨῨ
//!         = y꙲ꙈᴫѱΆῨῨ
//!         = m꙲ꙈᴫѱΆῨῨ
//!         = g꙲ꙈᴫѱΆῨῨ
//!         = d꙲ꙈᴫѱΆῨῨ
//!         = b꙲ꙈᴫѱΆῨῨ
//!         = a꙲ꙈᴫѱΆῨῨ
//!         = aꙈᴫѱΆῨῨ
//!         = aᴫѱΆῨῨ
//!         = aѱΆῨῨ
//!         = aѱΆῨῨ
//!         = aѱΆῨῨ
//!         = aиΆῨῨ
//!         = aМΆῨῨ
//!         = aЎΆῨῨ
//!         = aЇΆῨῨ
//!         = aЃΆῨῨ
//!         = aЁΆῨῨ
//!         = aЀΆῨῨ
//!         = aЀῨῨ
//!         = aЀῨ
//!         = aЀῨ
//!         = aЀῢ
//!         = aЀ῟
//!         = aЀ῞
//!         = aЀ῝
//! $ target/debug/examples/tutorial-simplify-play
//! str_val = dyiꙭᾪῇΊ
//!         = yiꙭᾪῇΊ
//!         = iꙭᾪῇΊ
//!         = iꙭᾪῇΊ
//!         = iꙭᾪῇΊ
//!         = eꙭᾪῇΊ
//!         = cꙭᾪῇΊ
//!         = bꙭᾪῇΊ
//!         = aꙭᾪῇΊ
//!         = aꙖᾪῇΊ
//!         = aꙋᾪῇΊ
//!         = aꙅᾪῇΊ
//!         = aꙂᾪῇΊ
//!         = aꙁᾪῇΊ
//!         = aꙀᾪῇΊ
//!         = aꙀῇΊ
//!         = aꙀΊ
//!         = aꙀΊ
//!         = aꙀΊ
//!         = aꙀΉ
//!         = aꙀΈ
//! ```
//!
//! Note that shrinking never shrinks a value to something outside the range
//! the strategy describes. Notice the strings in the above example still match
//! the regular expression even in the end. An integer drawn from
//! `100..1000i32` will shrink towards zero, but will stop at 100 since that is
//! the minimum value.
//!
//! `simplify()` and `complicate()` can be used to adapt our primitive fuzz
//! test to actually find the boundary condition.
//!
//! ```rust
//! extern crate proptest;
//!
//! use proptest::test_runner::TestRunner;
//! use proptest::strategy::{Strategy, ValueTree};
//!
//! fn some_function(v: i32) -> bool {
//!     // Do a bunch of stuff, but crash if v > 500
//!     // assert!(v <= 500);
//!     // But return a boolean instead of panicking for simplicity
//!     v <= 500
//! }
//!
//! // We know the function is broken, so use a purpose-built main function to
//! // find the breaking point.
//! fn main() {
//!     let mut runner = TestRunner::default();
//!     for _ in 0..256 {
//!         let mut val = (0..10000i32).new_value(&mut runner).unwrap();
//!         if some_function(val.current()) {
//!             // Test case passed
//!             continue;
//!         }
//!
//!         // We found our failing test case, simplify it as much as possible.
//!         loop {
//!             if !some_function(val.current()) {
//!                 // Still failing, find a simpler case
//!                 if !val.simplify() {
//!                     // No more simplification possible; we're done
//!                     break;
//!                 }
//!             } else {
//!                 // Passed this input, back up a bit
//!                 if !val.complicate() {
//!                     break;
//!                 }
//!             }
//!         }
//!
//!         println!("The minimal failing case is {}", val.current());
//!         assert_eq!(501, val.current());
//!         return;
//!     }
//!     panic!("Didn't find a failing test case");
//! }
//! ```
//!
//! This code reliably finds the boundary of the failure, 501.
//!
//! ### Using the Test Runner
//!
//! The above is quite a bit of code though, and it can't handle things like
//! panics. Fortunately, proptest's
//! [`TestRunner`](test_runner/struct.TestRunner.html) provides this
//! functionality for us. The method we're interested in is `run`. We simply
//! give it the strategy and a function to test inputs and it takes care of the
//! rest.
//!
//! ```rust
//! extern crate proptest;
//!
//! use proptest::test_runner::{Config, FailurePersistence,
//!                             TestError, TestRunner};
//!
//! fn some_function(v: i32) {
//!     // Do a bunch of stuff, but crash if v > 500.
//!     // We return to normal `assert!` here since `TestRunner` catches
//!     // panics.
//!     assert!(v <= 500);
//! }
//!
//! // We know the function is broken, so use a purpose-built main function to
//! // find the breaking point.
//! fn main() {
//!     let mut runner = TestRunner::new(Config {
//!         // Turn failure persistence off for demonstration
//!         failure_persistence: FailurePersistence::Off,
//!         .. Config::default()
//!     });
//!     let result = runner.run(&(0..10000i32), |&v| {
//!         some_function(v);
//!         Ok(())
//!     });
//!     match result {
//!         Err(TestError::Fail(_, value)) => {
//!             println!("Found minimal failing case: {}", value);
//!             assert_eq!(501, value);
//!         },
//!         result => panic!("Unexpected result: {:?}", result),
//!     }
//! }
//! ```
//!
//! That's a lot better! Still a bit boilerplatey; the `proptest!` will help
//! with that, but it does some other stuff we haven't covered yet, so for the
//! moment we'll keep using `TestRunner` directly.
//!
//! ### Compound Strategies
//!
//! Testing functions that take single arguments of primitive types is nice and
//! all, but is kind of underwhelming. Back when we were writing the whole
//! stack by hand, extending the technique to, say, _two_ integers was clear,
//! if verbose. But `TestRunner` only takes a single `Strategy`; how can we
//! test a function that needs inputs from more than one?
//!
//! ```rust,ignore
//! use proptest::test_runner::TestRunner;
//!
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! # /*
//! #[test]
//! # */
//! fn test_add() {
//!     let mut runner = TestRunner::default();
//!     runner.run(/* uhhm... */).unwrap();
//! }
//! #
//! # fn main() { test_add(); }
//! ```
//!
//! The key is that strategies are _composable_. The simplest form of
//! composition is "compound strategies", where we take multiple strategies and
//! combine their values into one value that holds each input separately. There
//! are several of these. The simplest is a tuple; a tuple of strategies is
//! itself a strategy for tuples of the values those strategies produce. For
//! example, `(0..10i32,100..1000i32)` is a strategy for pairs of integers
//! where the first value is between 0 and 100 and the second is between 100
//! and 1000.
//!
//! So for our two-argument function, our strategy is simply a tuple of ranges.
//!
//! ```rust
//! use proptest::test_runner::TestRunner;
//!
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! # /*
//! #[test]
//! # */
//! fn test_add() {
//!     let mut runner = TestRunner::default();
//!     // Combine our two inputs into a strategy for one tuple. Our test
//!     // function then destructures the generated tuples back into separate
//!     // `a` and `b` variables to be passed in to `add()`.
//!     runner.run(&(0..1000i32, 0..1000i32), |&(a, b)| {
//!         let sum = add(a, b);
//!         assert!(sum >= a);
//!         assert!(sum >= b);
//!         Ok(())
//!     }).unwrap();
//! }
//! #
//! # fn main() { test_add(); }
//! ```
//!
//! Other compound strategies include fixed-sizes arrays of strategies, as well
//! as the various strategies provided in the [collection](collection/index.html)
//! module.
//!
//! ### Syntax Sugar: `proptest!`
//!
//! Now that we know about compound strategies, we can understand how the
//! [`proptest!`](macro.proptest.html) macro works. Our example from the prior
//! section can be rewritten using that macro like so:
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//!
//! fn add(a: i32, b: i32) -> i32 {
//!     a + b
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_add(a in 0..1000i32, b in 0..1000i32) {
//!         let sum = add(a, b);
//!         assert!(sum >= a);
//!         assert!(sum >= b);
//!     }
//! }
//! #
//! # fn main() { test_add(); }
//! ```
//!
//! Conceptually, the desugaring process is fairly simple. At the start of the
//! test function, a new `TestRunner` is constructed. The input strategies
//! (after the `in` keyword) are grouped into a tuple. That tuple is passed in
//! to the `TestRunner` as the input strategy. The test body has `Ok(())` added
//! to the end, then is put into a lambda that destructures the generated input
//! tuple back into the named parameters and then runs the body. The end result
//! is extremely similar to what we wrote by hand in the prior section.
//!
//! `proptest!` actually does a few other things in order to make failure
//! output easier to read and to overcome the 10-tuple limit.
//!
//! ### Transforming Strategies
//!
//! Suppose you have a function that takes a string which needs to be the
//! `Display` format of an arbitrary `u32`. A first attempt to providing this
//! argument might be to use a regular expression, like so:
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//!
//! fn do_stuff(v: &str) {
//!     let i: u32 = v.parse().unwrap();
//!     let s = i.to_string();
//!     assert_eq!(&s, v);
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_do_stuff(ref v in "[1-9][0-9]{0,8}") {
//!         do_stuff(v);
//!     }
//! }
//! # fn main() { test_do_stuff(); }
//! ```
//!
//! This kind of works, but it has problems. For one, it does not explore the
//! whole `u32` space. It is possible to write a regular expression that does,
//! but such an expression is rather long, and also results in a pretty odd
//! distribution of values. The input also doesn't shrink correctly, since
//! proptest tries to shrink it in terms of a string rather than an integer.
//!
//! What you really want to do is generate a `u32` and then pass in its string
//! representation. One way to do this is to just take `u32` as an input to the
//! test and then transform it to a string within the test code. This approach
//! works fine, but isn't reusable or composable. Ideally, we could get a
//! _strategy_ that does this.
//!
//! The thing we're looking for is the first strategy _combinator_, `prop_map`.
//! We need to ensure `Strategy` is in scope to use it.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! // Grab `Strategy` and a shorter namespace prefix
//! use proptest::prelude::*;
//!
//! fn do_stuff(v: &str) {
//!     let i: u32 = v.parse().unwrap();
//!     let s = i.to_string();
//!     assert_eq!(&s, v);
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_do_stuff(ref v in prop::num::u32::ANY.prop_map(
//!                          |v| v.to_string())) {
//!         do_stuff(v);
//!     }
//! }
//! # fn main() { test_do_stuff(); }
//! ```
//!
//! Calling `prop_map` on a `Strategy` creates a new strategy which transforms
//! every generated value using the provided function. Proptest retains the
//! relationship between the original `Strategy` and the transformed one; as a
//! result, shrinking occurs in terms of `u32`, even though we're generating a
//! `String`.
//!
//! `prop_map` is also the principal way to define strategies for new types,
//! since most types are simply composed of other, simpler values.
//!
//! Let's update our code so it takes a more interesting structure.
//!
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! #[derive(Clone, Debug)]
//! struct Order {
//!   id: String,
//!   // Some other fields, though the test doesn't do anything with them
//!   item: String,
//!   quantity: u32,
//! }
//!
//! fn do_stuff(order: &Order) {
//!     let i: u32 = order.id.parse().unwrap();
//!     let s = i.to_string();
//!     assert_eq!(s, order.id);
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_do_stuff(
//!         ref order in
//!         (prop::num::u32::ANY.prop_map(|v| v.to_string()),
//!          "[a-z]*", 1..1000u32).prop_map(
//!              |(id, item, quantity)| Order { id, item, quantity })
//!     ) {
//!         do_stuff(order);
//!     }
//! }
//! # fn main() { test_do_stuff(); }
//! ```
//!
//! Notice how we were able to take the output from `prop_map` and put it in a
//! tuple, then call `prop_map` on _that_ tuple to produce yet another value.
//!
//! But that's quite a mouthful in the argument list. Fortunately, strategies
//! are normal values, so we can extract it to a function.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! // snip
//! #
//! # #[derive(Clone, Debug)]
//! # struct Order {
//! #   id: String,
//! #   // Some other fields, though the test doesn't do anything with them
//! #   item: String,
//! #   quantity: u32,
//! # }
//! #
//! # fn do_stuff(order: &Order) {
//! #     let i: u32 = order.id.parse().unwrap();
//! #     let s = i.to_string();
//! #     assert_eq!(s, order.id);
//! # }
//!
//! fn arb_order(max_quantity: u32) -> BoxedStrategy<Order> {
//!     (prop::num::u32::ANY.prop_map(|v| v.to_string()),
//!      "[a-z]*", 1..max_quantity)
//!     .prop_map(|(id, item, quantity)| Order { id, item, quantity })
//!     .boxed()
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_do_stuff(ref order in arb_order(1000)) {
//!         do_stuff(order);
//!     }
//! }
//! # fn main() { test_do_stuff(); }
//! ```
//!
//! We `boxed()` the strategy in the function since otherwise the type would
//! not be nameable, and even if it were, it would be very hard to read or
//! write. Boxing a `Strategy` turns both it and its `ValueTree`s into trait
//! objects, which both makes the types simpler and can be used to mix
//! heterogeneous `Strategy` types as long as they produce the same value
//! types.
//!
//! The `arb_order()` function is also _parameterised_, which is another
//! advantage of extracting strategies to separate functions. In this case, if
//! we have a test that needs an `Order` with no more than a dozen items, we
//! can simply call `arb_order(12)` rather than needing to write out a whole
//! new strategy.
//!
//! ### Syntax Sugar: `prop_compose!`
//!
//! Defining strategy-returning functions like this is extremely useful, but
//! the code above is a bit verbose, as well as hard to read for similar
//! reasons to writing test functions by hand.
//!
//! To simplify this task, proptest includes the
//! [`prop_compose!`](macro.prop_compose.html) macro. Before going into
//! details, here's our code from above rewritten to use it.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! // snip
//! #
//! # #[derive(Clone, Debug)]
//! # struct Order {
//! #   id: String,
//! #   // Some other fields, though the test doesn't do anything with them
//! #   item: String,
//! #   quantity: u32,
//! # }
//! #
//! # fn do_stuff(order: &Order) {
//! #     let i: u32 = order.id.parse().unwrap();
//! #     let s = i.to_string();
//! #     assert_eq!(s, order.id);
//! # }
//!
//! prop_compose! {
//!     fn arb_order_id()(id in prop::num::u32::ANY) -> String {
//!         id.to_string()
//!     }
//! }
//! prop_compose! {
//!     fn arb_order(max_quantity: u32)
//!                 (id in arb_order_id(), item in "[a-z]*",
//!                  quantity in 1..max_quantity)
//!                 -> Order {
//!         Order { id, item, quantity }
//!     }
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_do_stuff(ref order in arb_order(1000)) {
//!         do_stuff(order);
//!     }
//! }
//! # fn main() { test_do_stuff(); }
//! ```
//!
//! We had to extract `arb_order_id()` out into its own function, but otherwise
//! this desugars to almost exactly what we wrote in the previous section. The
//! generated function takes the first parameter list as arguments. These
//! arguments are used to select the strategies in the second argument list.
//! Values are then drawn from those strategies and transformed by the function
//! body. The actual function as a return type of `BoxedStrategy<T>` where `T`
//! is the declared return type.
//!
//! ### Filtering
//!
//! Sometimes, you have a case where your input values have some sort of
//! "irregular" constraint on them. For example, an integer needing to be even,
//! or two values needing to be non-equal.
//!
//! In general, the ideal solution is to find a way to take a seed value and
//! then use `prop_map` to transform it into the desired, irregular domain. For
//! example, to generate even integers, use something like
//!
//! ```rust,no_run
//! # #[macro_use] extern crate proptest;
//! prop_compose! {
//!     // Generate arbitrary integers up to half the maximum desired value,
//!     // then multiply them by 2, thus producing only even integers in the
//!     // desired range.
//!     fn even_integer(max: i32)(base in 0..max/2) -> i32 { base * 2 }
//! }
//! # fn main() { }
//! ```
//!
//! For the cases where this is not viable, it is possible to filter
//! strategies. Proptest actually divides filters into two categories:
//!
//! - "Local" filters apply to a single strategy. If a value is rejected,
//!   a new value is drawn from that strategy only.
//!
//! - "Global" filters apply to the whole test case. If the test case is
//!   rejected, the whole thing is regenerated.
//!
//! The distinction is somewhat arbitrary, since something like a "global
//! filter" could be created by just putting a "local filter" around the whole
//! input strategy. In practise, the distinction is as to what code performs
//! the rejection.
//!
//! A local filter is created with the `prop_filter` combinator. Besides a
//! function indicating whether to accept the value, it also takes an _owned_
//! `String` which it uses to record where/why the rejection happened.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn some_test(
//!       v in (0..1000u32)
//!         .prop_filter("Values must not divisible by 7 xor 11".to_owned(),
//!                      |v| !((0 == v % 7) ^ (0 == v % 11)))
//!     ) {
//!         assert_eq!(0 == v % 7, 0 == v % 11);
//!     }
//! }
//! # fn main() { some_test(); }
//! ```
//!
//! Global filtering results when a test itself returns
//! `Err(TestCaseError::Reject)`. The [`prop_assume!`](macro.prop_assume.html)
//! macro provides an easy way to do this.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//!
//! fn frob(a: i32, b: i32) -> (i32, i32) {
//!     let d = (a - b).abs();
//!     (a / d, b / d)
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_frob(a in -1000..1000, b in -1000..1000) {
//!         // Input illegal if a==b.
//!         // Equivalent to
//!         // if (a == b) { return Err(TestCaseError::Reject(...)); }
//!         prop_assume!(a != b);
//!
//!         let (a2, b2) = frob(a, b);
//!         assert!(a2.abs() <= a.abs());
//!         assert!(b2.abs() <= b.abs());
//!     }
//! }
//! # fn main() { test_frob(); }
//! ```
//!
//! While useful, filtering has a lot of disadvantages:
//!
//! - Since it is simply rejection sampling, it will slow down generation of
//! test cases since values need to be generated additional times to satisfy
//! the filter. In the case where a filter always returns false, a test could
//! theoretically never generate a result.
//!
//! - Proptest tracks how many local and global rejections have happened, and
//! aborts if they exceed a certain number. This prevents a test taking an
//! extremely long time due to rejections, but means not all filters are viable
//! in the default configuration. The limits for local and global rejections
//! are different; by default, proptest allows a large number of local
//! rejections but a fairly small number of global rejections, on the premise
//! that the former are cheap but potentially common (having been built into
//! the strategy) but the latter are expensive but rare (being an edge case in
//! the particular test).
//!
//! - Shrinking and filtering do not play well together. When shrinking, if a
//! value winds up being rejected, there is no pass/fail information to
//! continue shrinking properly. Instead, proptest treats such a rejection the
//! same way it handles a shrink that results in a passing test: by backing
//! away from simplification with a call to `complicate()`. Thus encountering a
//! filter rejection during shrinking prevents shrinking from continuing to any
//! simpler values, even if there are some that would be accepted by the
//! filter.
//!
//! ### Generating Recursive Data
//!
//! Randomly generating recursive data structures is trickier than it sounds.
//! For example, the below is a naïve attempt at generating a JSON AST by using
//! recursion. This also uses the [`prop_oneof!`](macro.prop_oneof.html), which
//! we haven't seen yet but should be self-explanatory.
//!
//! ```rust,no_run
//! #[macro_use] extern crate proptest;
//!
//! use std::collections::HashMap;
//! use proptest::prelude::*;
//!
//! #[derive(Clone, Debug)]
//! enum Json {
//!     Null,
//!     Bool(bool),
//!     Number(f64),
//!     String(String),
//!     Array(Vec<Json>),
//!     Map(HashMap<String, Json>),
//! }
//!
//! fn arb_json() -> BoxedStrategy<Json> {
//!     prop_oneof![
//!         Just(Json::Null),
//!         prop::bool::ANY.prop_map(Json::Bool),
//!         prop::num::f64::ANY.prop_map(Json::Number),
//!         ".*".prop_map(Json::String),
//!         prop::collection::vec(arb_json(), 0..10).prop_map(Json::Array),
//!         prop::collection::hash_map(
//!           ".*", arb_json(), 0..10).prop_map(Json::Map),
//!     ].boxed()
//! }
//! # fn main() { }
//! ```
//!
//! Upon closer consideration, this obviously can't work because `arb_json()`
//! recurses unconditionally.
//!
//! A more sophisticated attempt is to define one strategy for each level of
//! nesting up to some maximum. This doesn't overflow the stack, but as defined
//! here, even four levels of nesting will produce trees with _thousands_ of
//! nodes; by eight levels, we get to tens of _millions_.
//!
//! Proptest provides a more reliable solution in the form of the
//! `prop_recursive` combinator. To use this, we create a strategy for the
//! non-recursive case, then give the combinator that strategy, some size
//! parameters, and a function to transform a nested strategy into a recursive
//! strategy.
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//!
//! use std::collections::HashMap;
//! use proptest::prelude::*;
//!
//! #[derive(Clone, Debug)]
//! enum Json {
//!     Null,
//!     Bool(bool),
//!     Number(f64),
//!     String(String),
//!     Array(Vec<Json>),
//!     Map(HashMap<String, Json>),
//! }
//!
//! fn arb_json() -> BoxedStrategy<Json> {
//!     let leaf = prop_oneof![
//!         Just(Json::Null),
//!         prop::bool::ANY.prop_map(Json::Bool),
//!         prop::num::f64::ANY.prop_map(Json::Number),
//!         ".*".prop_map(Json::String),
//!     ];
//!     leaf.prop_recursive(
//!       8, // 8 levels deep
//!       256, // Shoot for maximum size of 256 nodes
//!       10, // We put up to 10 items per collection
//!       |inner| prop_oneof![
//!           // Take the inner strategy and make the two recursive cases.
//!           prop::collection::vec(inner.clone(), 0..10)
//!               .prop_map(Json::Array),
//!           prop::collection::hash_map(".*", inner, 0..10)
//!               .prop_map(Json::Map),
//!       ].boxed()).boxed()
//! }
//! # fn main() { }
//! ```
//!
//! ### Higher-Order Strategies
//!
//! A _higher-order strategy_ is a strategy which is generated by another
//! strategy. That sounds kind of scary, so let's consider an example first.
//!
//! Say you have a function you want to test that takes a slice and an index
//! into that slice. If we use a fixed size for the slice, it's easy, but maybe
//! we need to test with different slice sizes. We could try something with a
//! filter:
//!
//! ```rust,ignore
//! fn some_function(stuff: &[String], index: usize) { /* do stuff */ }
//!
//! proptest! {
//!     #[test]
//!     fn test_some_function(
//!         ref stuff in prop::collection::vec(".*", 1..100),
//!         index in 0..100usize
//!     ) {
//!         prop_assume!(index < stuff.len());
//!         some_function(stuff, index);
//!     }
//! }
//! ```
//!
//! This doesn't work very well. First off, you get a lot of global rejections
//! since `index` will be outside of `stuff` 50% of the time. But secondly, it
//! will be rare to actually get a small `stuff` vector, since it would have to
//! randomly choose a small `index` at the same time.
//!
//! The solution is the `prop_flat_map` combinator. This is sort of like
//! `prop_map`, except that the transform returns a _strategy_ instead of a
//! value. This is more easily understood by implementing our example:
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::prelude::*;
//!
//! fn some_function(stuff: &[String], index: usize) {
//!     let _ = &stuff[index];
//!     // Do stuff
//! }
//!
//! fn vec_and_index() -> BoxedStrategy<(Vec<String>, usize)> {
//!     prop::collection::vec(".*", 1..100)
//!         .prop_flat_map(|vec| {
//!             let len = vec.len();
//!             (Just(vec), 0..len)
//!         }).boxed()
//! }
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn test_some_function((ref vec, index) in vec_and_index()) {
//!         some_function(vec, index);
//!     }
//! }
//! # fn main() { test_some_function(); }
//! ```
//!
//! In `vec_and_index()`, we make a strategy to produce an arbitrary vector.
//! But then we derive a new strategy based on _values_ produced by the first
//! one. The new strategy produces the generated vector unchanged, but also
//! adds a valid index into that vector, which we can do by picking the
//! strategy for that index based on the size of the vector.
//!
//! Even though the new strategy specifies the singleton `Just(vec)` strategy
//! for the vector, proptest still understands the connection to the original
//! strategy and will shrink `vec` as well. All the while, `index` continues to
//! be a valid index into `vec`.
//!
//! `prop_compose!` actually allows making second-order strategies like this by
//! simply providing three argument lists instead of two. The below desugars to
//! something much like what we wrote by hand above, except that the index and
//! vector's positions are internally reversed due to borrowing limitations.
//!
//! ```rust,no_run
//! # #[macro_use] extern crate proptest;
//! # use proptest::prelude::*;
//! prop_compose! {
//!     fn vec_and_index()(vec in prop::collection::vec(".*", 1..100))
//!                     (index in 0..vec.len(), vec in Just(vec))
//!                     -> (Vec<String>, usize) {
//!        (vec, index)
//!    }
//! }
//! # fn main() { }
//! ```
//!
//! ### Configuring the number of tests cases requried
//!
//! The default number of successful test cases that must execute for a test
//! as a whole to pass is currently 256. If you are not satisfied with this
//! and want to run more or fewer, there are a few ways to do this.
//!
//! The first way is to set the environment-variable `PROPTEST_CASES` to a
//! value that can be successfully parsed as a `u32`. The value you set to this
//! variable is now the new default.
//!
//! Another way is to use `#![proptest_config(expr)]` inside `proptest!` where
//! `expr : Config`. To only change the number of test cases, you can simply
//! write:
//!
//! ```rust
//! #[macro_use] extern crate proptest;
//! use proptest::test_runner::Config;
//!
//! fn add(a: i32, b: i32) -> i32 { a + b }
//!
//! proptest! {
//!     // The next line modifies the number of tests.
//!     #![proptest_config(Config::with_cases(1000))]
//!     # /*
//!     #[test]
//!     # */
//!     fn test_add(a in 0..1000i32, b in 0..1000i32) {
//!         let sum = add(a, b);
//!         assert!(sum >= a);
//!         assert!(sum >= b);
//!     }
//! }
//! #
//! # fn main() { test_add(); }
//! ```
//!
//! Through the same `proptest_config` mechanism you may fine-tune your
//! configuration through the `Config` type. See its documentation for more
//! information.
//!
//! ### Conclusion
//!
//! That's it for the tutorial, at least for now. There are more details for
//! the features discussed above on their individual documentation pages, and
//! you can find out about all the strategies provided out-of-the-box by
//! perusing the module tree below.

#![deny(missing_docs)]

#![cfg_attr(feature = "unstable", feature(i128_type))]

extern crate bit_set;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate quick_error;
extern crate rand;
extern crate regex_syntax;

#[cfg(test)] extern crate regex;

// Pervasive internal sugar
macro_rules! mapfn {
    ($(#[$meta:meta])* [$($vis:tt)*]
     fn $name:ident[$($gen:tt)*]($parm:ident: $input:ty) -> $output:ty {
         $($body:tt)*
     }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug)]
        $($vis)* struct $name;
        impl $($gen)* ::strategy::statics::MapFn<$input> for $name {
            type Output = $output;
            fn apply(&self, $parm: $input) -> $output {
                $($body)*
            }
        }
    }
}

macro_rules! delegate_vt_0 {
    () => {
        fn current(&self) -> Self::Value {
            self.0.current()
        }

        fn simplify(&mut self) -> bool {
            self.0.simplify()
        }

        fn complicate(&mut self) -> bool {
            self.0.complicate()
        }
    }
}

macro_rules! opaque_strategy_wrapper {
    ($(#[$smeta:meta])* pub struct $stratname:ident
     [$($sgen:tt)*][$($swhere:tt)*]
     ($innerstrat:ty) -> $stratvtty:ty;

     $(#[$vmeta:meta])* pub struct $vtname:ident
     [$($vgen:tt)*][$($vwhere:tt)*]
     ($innervt:ty) -> $actualty:ty;
    ) => {
        $(#[$smeta])* pub struct $stratname $($sgen)* ($innerstrat)
            $($swhere)*;

        $(#[$vmeta])* pub struct $vtname $($vgen)* ($innervt) $($vwhere)*;

        impl $($sgen)* Strategy for $stratname $($sgen)* $($swhere)* {
            type Value = $stratvtty;
            fn new_value(&self, runner: &mut TestRunner) -> NewTree<Self> {
                self.0.new_value(runner).map($vtname)
            }
        }

        impl $($vgen)* ValueTree for $vtname $($vgen)* $($vwhere)* {
            type Value = $actualty;

            delegate_vt_0!();
        }
    }
}

pub mod test_runner;
pub mod strategy;
pub mod bool;
pub mod num;
pub mod bits;
pub mod tuple;
pub mod array;
pub mod collection;
pub mod char;
pub mod string;
pub mod option;
pub mod result;
pub mod sample;

#[doc(hidden)]
#[macro_use] pub mod sugar;

pub mod prelude;
