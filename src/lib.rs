//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Proptest is a property testing framework (i.e., the QuickCheck family)
//! inspired by the [Hypothesis](http://hypothesis.works/) framework for
//! Python.
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
//! ```no_run
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
//! ```ignore
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
//! proptest = "0.1.0"
//! ```
//!
//! and at the top of `main.rs` or `lib.rs`:
//!
//! ```ignore
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
//! ```ignore
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
//! thread 'main' panicked at 'Test failed: byte index 4 is not a char boundary; it is inside 'ௗ' (bytes 2..5) of `aAௗ0㌀0`; minimal failing input: "aAௗ0㌀0"
//! 	successes: 102
//! 	local rejects: 0
//! 	global rejects: 0
//! '
//! ```
//!
//! The first thing we should do is copy the failing case to a traditional unit
//! test since it has exposed a bug.
//!
//! ```ignore
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
//! ```no_run
//! use std::ascii::AsciiExt;
//!
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
//! ```ignore
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
//! ```ignore
//! proptest! {
//!     // snip...
//!
//!     #[test]
//!     fn parses_date_back_to_original(y in 0u32..10000,
//!                                     m in 1u32..13, d in 1u32..32) {
//!         let (y2, m2, d2) = parse_date(
//!             &format!("{:04}-{:02}-{:02}", y, m, d)).unwrap();
//!         assert_eq!((y, m, d), (y2, m2, d2));
//!     }
//! }
//! ```
//!
//! Here, we see that besides regexes, we can use any expression which is a
//! `proptest::Strategy`, in this case, integer ranges.
//!
//! The test fails when we run it, again with a bunch of output, though the
//! full output is actually rather interesting this time:
//!
//! ```text
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(1358, 11, 28)`, right: `(1358, 1, 28)`)', examples/dateparser_v2.rs:46
//! note: Run with `RUST_BACKTRACE=1` for a backtrace.
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(679, 11, 28)`, right: `(679, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(339, 11, 28)`, right: `(339, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(169, 11, 28)`, right: `(169, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(84, 11, 28)`, right: `(84, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(42, 11, 28)`, right: `(42, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(21, 11, 28)`, right: `(21, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(10, 11, 28)`, right: `(10, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(5, 11, 28)`, right: `(5, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(2, 11, 28)`, right: `(2, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(1, 11, 28)`, right: `(1, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 11, 28)`, right: `(0, 1, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 28)`, right: `(0, 0, 28)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 14)`, right: `(0, 0, 14)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 7)`, right: `(0, 0, 7)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 4)`, right: `(0, 0, 4)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 2)`, right: `(0, 0, 2)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'assertion failed: `(left == right)` (left: `(0, 10, 1)`, right: `(0, 0, 1)`)', examples/dateparser_v2.rs:46
//! thread 'main' panicked at 'Test failed: assertion failed: `(left == right)` (left: `(0, 10, 1)`, right: `(0, 0, 1)`); minimal failing input: (0, 10, 1)
//! 	successes: 0
//! 	local rejects: 0
//! 	global rejects: 0
//! ', examples/dateparser_v2.rs:33
//! ```
//!
//! Notice how we started with a completely random date — 1358-11-28 — but it
//! was then quickly reduced to the minimal case, 0000-10-01, which gets parsed
//! as if it were 0000-00-01.
//!
//! Again, let's add this as its own unit test:
//!
//! ```ignore
//! #[test]
//! fn test_october_first() {
//!   assert_eq!(Some(0, 10, 1), parse_date("0000-10-01"));
//! }
//! ```
//!
//! What's special about this case? The tens digit of the month! In our code:
//!
//! ```ignore
//!     let month = &s[6..7];
//! ```
//!
//! We were off by one, and need to use the range `5..7`. After fixing this,
//! the test passes.
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
//! ## Limitations of Property Testing
//!
//! Given infinite time, property testing will eventually explore the whole
//! input space to a test. However, time is not infinite, so only a randomly
//! sampled portion of the input space can be explored. This means that
//! property testing is extremely unlikely to find single-value edge cases in a
//! large space. For example, the following test will virtually always pass:
//!
//! ```
//! #[macro_use] extern crate proptest;
//!
//! proptest! {
//!     # /*
//!     #[test]
//!     # */
//!     fn i64_abs_is_never_negative(a in proptest::num::i64::ANY) {
//!         assert!(a.abs() >= 0);
//!     }
//! }
//! #
//! # fn main() { i64_abs_is_never_negative(); }
//! ```
//!
//! Because of this, traditional unit testing with intelligently selected cases
//! is still necessary for many kinds of problems.
//!
//! Similarly, in some cases it can be hard or impossible to define a strategy
//! which actually produces useful inputs. A strategy of `.{1,4096}` may be
//! great to fuzz a C parser, but is highly unlikely to produce anything that
//! makes it to a code generator.

#![deny(missing_docs)]

extern crate bit_set;
#[macro_use] extern crate quick_error;
extern crate rand;
extern crate regex_syntax;

#[cfg(test)] extern crate regex;

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

#[macro_use] mod sugar;
