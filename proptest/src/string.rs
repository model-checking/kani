//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strategies for generating strings and byte strings from regular
//! expressions.

use crate::std_facade::{Box, Cow, String, ToOwned, Vec};
use core::fmt;
use core::mem;
use core::ops::RangeInclusive;
use core::u32;

use regex_syntax::hir::{
    self, Hir,
    HirKind::*,
    Literal::*,
    RepetitionKind::{self, *},
    RepetitionRange::*,
};
use regex_syntax::{Error as ParseError, Parser};

use crate::bool;
use crate::char;
use crate::collection::{size_range, vec, SizeRange};
use crate::strategy::*;
use crate::test_runner::*;

/// Wraps the regex that forms the `Strategy` for `String` so that a sensible
/// `Default` can be given. The default is a string of non-control characters.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringParam(&'static str);

impl From<StringParam> for &'static str {
    fn from(x: StringParam) -> Self {
        x.0
    }
}

impl From<&'static str> for StringParam {
    fn from(x: &'static str) -> Self {
        StringParam(x)
    }
}

impl Default for StringParam {
    fn default() -> Self {
        StringParam("\\PC*")
    }
}

// quick_error! uses bare trait objects, so we enclose its invocation here in a
// module so the lint can be disabled just for it.
#[allow(bare_trait_objects)]
mod error_container {
    use super::*;

    quick_error! {
        /// Errors which may occur when preparing a regular expression for use with
        /// string generation.
        #[derive(Debug)]
        pub enum Error {
            /// The string passed as the regex was not syntactically valid.
            RegexSyntax(err: ParseError) {
                from()
                    source(err)
                    display("{}", err)
            }
            /// The regex was syntactically valid, but contains elements not
            /// supported by proptest.
            UnsupportedRegex(message: &'static str) {
                display("{}", message)
            }
        }
    }
}

pub use self::error_container::Error;

opaque_strategy_wrapper! {
    /// Strategy which generates values (i.e., `String` or `Vec<u8>`) matching
    /// a regular expression.
    ///
    /// Created by various functions in this module.
    #[derive(Debug)]
    pub struct RegexGeneratorStrategy[<T>][where T : fmt::Debug]
        (SBoxedStrategy<T>) -> RegexGeneratorValueTree<T>;
    /// `ValueTree` corresponding to `RegexGeneratorStrategy`.
    pub struct RegexGeneratorValueTree[<T>][where T : fmt::Debug]
        (Box<dyn ValueTree<Value = T>>) -> T;
}

impl Strategy for str {
    type Tree = RegexGeneratorValueTree<String>;
    type Value = String;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        string_regex(self).unwrap().new_tree(runner)
    }
}

type ParseResult<T> = Result<RegexGeneratorStrategy<T>, Error>;

#[doc(hidden)]
/// A type which knows how to produce a `Strategy` from a regular expression
/// generating the type.
///
/// This trait exists for the benefit of `#[proptest(regex = "...")]`.
/// It is semver exempt, so use at your own risk.
/// If you found a use for the trait beyond `Vec<u8>` and `String`,
/// please file an issue at https://github.com/AltSysrq/proptest.
pub trait StrategyFromRegex: Sized + fmt::Debug {
    type Strategy: Strategy<Value = Self>;

    /// Produce a strategy for `Self` from the `regex`.
    fn from_regex(regex: &str) -> Self::Strategy;
}

impl StrategyFromRegex for String {
    type Strategy = RegexGeneratorStrategy<Self>;

    fn from_regex(regex: &str) -> Self::Strategy {
        string_regex(regex).unwrap()
    }
}

impl StrategyFromRegex for Vec<u8> {
    type Strategy = RegexGeneratorStrategy<Self>;

    fn from_regex(regex: &str) -> Self::Strategy {
        bytes_regex(regex).unwrap()
    }
}

/// Creates a strategy which generates strings matching the given regular
/// expression.
///
/// If you don't need error handling and aren't limited by setup time, it is
/// also possible to directly use a `&str` as a strategy with the same effect.
pub fn string_regex(regex: &str) -> ParseResult<String> {
    string_regex_parsed(&regex_to_hir(regex)?)
}

/// Like `string_regex()`, but allows providing a pre-parsed expression.
pub fn string_regex_parsed(expr: &Hir) -> ParseResult<String> {
    bytes_regex_parsed(expr)
        .map(|v| {
            v.prop_map(|bytes| {
                String::from_utf8(bytes).expect("non-utf8 string")
            })
            .sboxed()
        })
        .map(RegexGeneratorStrategy)
}

/// Creates a strategy which generates byte strings matching the given regular
/// expression.
pub fn bytes_regex(regex: &str) -> ParseResult<Vec<u8>> {
    bytes_regex_parsed(&regex_to_hir(regex)?)
}

/// Like `bytes_regex()`, but allows providing a pre-parsed expression.
pub fn bytes_regex_parsed(expr: &Hir) -> ParseResult<Vec<u8>> {
    match expr.kind() {
        Empty => Ok(Just(vec![]).sboxed()),

        Literal(lit) => Ok(Just(match lit {
            Unicode(scalar) => to_bytes(*scalar),
            Byte(byte) => vec![*byte],
        })
        .sboxed()),

        Class(class) => Ok(match class {
            hir::Class::Unicode(class) => {
                unicode_class_strategy(class).prop_map(to_bytes).sboxed()
            }
            hir::Class::Bytes(class) => {
                let subs = class.iter().map(|r| r.start()..=r.end());
                Union::new(subs).prop_map(|b| vec![b]).sboxed()
            }
        }),

        Repetition(rep) => Ok(vec(
            bytes_regex_parsed(&rep.hir)?,
            to_range(rep.kind.clone())?,
        )
        .prop_map(|parts| {
            parts.into_iter().fold(vec![], |mut acc, child| {
                acc.extend(child);
                acc
            })
        })
        .sboxed()),

        Group(group) => bytes_regex_parsed(&group.hir).map(|v| v.0),

        Concat(subs) => {
            let subs = ConcatIter {
                iter: subs.iter(),
                buf: vec![],
                next: None,
            };
            let ext = |(mut lhs, rhs): (Vec<_>, _)| {
                lhs.extend(rhs);
                lhs
            };
            Ok(subs
                .fold(Ok(None), |accum: Result<_, Error>, rhs| {
                    Ok(match accum? {
                        None => Some(rhs?.sboxed()),
                        Some(accum) => {
                            Some((accum, rhs?).prop_map(ext).sboxed())
                        }
                    })
                })?
                .unwrap_or_else(|| Just(vec![]).sboxed()))
        }

        Alternation(subs) => {
            Ok(Union::try_new(subs.iter().map(bytes_regex_parsed))?.sboxed())
        }

        Anchor(_) => {
            unsupported("line/text anchors not supported for string generation")
        }

        WordBoundary(_) => unsupported(
            "word boundary tests not supported for string generation",
        ),
    }
    .map(RegexGeneratorStrategy)
}

fn unicode_class_strategy(
    class: &hir::ClassUnicode,
) -> char::CharStrategy<'static> {
    static NONL_RANGES: &[RangeInclusive<char>] = &[
        '\x00'..='\x09',
        // Multiple instances of the latter range to partially make up
        // for the bias of having such a tiny range in the control
        // characters.
        '\x0B'..=::core::char::MAX,
        '\x0B'..=::core::char::MAX,
        '\x0B'..=::core::char::MAX,
        '\x0B'..=::core::char::MAX,
        '\x0B'..=::core::char::MAX,
    ];

    let dotnnl = |x: &hir::ClassUnicodeRange, y: &hir::ClassUnicodeRange| {
        x.start() == '\0'
            && x.end() == '\x09'
            && y.start() == '\x0B'
            && y.end() == '\u{10FFFF}'
    };

    char::ranges(match class.ranges() {
        [x, y] if dotnnl(x, y) || dotnnl(y, x) => Cow::Borrowed(NONL_RANGES),
        _ => Cow::Owned(class.iter().map(|r| r.start()..=r.end()).collect()),
    })
}

struct ConcatIter<'a, I> {
    buf: Vec<u8>,
    iter: I,
    next: Option<&'a Hir>,
}

fn flush_lit_buf<I>(
    it: &mut ConcatIter<'_, I>,
) -> Option<ParseResult<Vec<u8>>> {
    Some(Ok(RegexGeneratorStrategy(
        Just(mem::replace(&mut it.buf, vec![])).sboxed(),
    )))
}

impl<'a, I: Iterator<Item = &'a Hir>> Iterator for ConcatIter<'a, I> {
    type Item = ParseResult<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        // A left-over node, process it first:
        if let Some(next) = self.next.take() {
            return Some(bytes_regex_parsed(next));
        }

        // Accumulate a literal sequence as long as we can:
        while let Some(next) = self.iter.next() {
            match next.kind() {
                // A literal. Accumulate:
                Literal(Unicode(scalar)) => self.buf.extend(to_bytes(*scalar)),
                Literal(Byte(byte)) => self.buf.push(*byte),
                // Encountered a non-literal.
                _ => {
                    return if !self.buf.is_empty() {
                        // We've accumulated a literal from before, flush it out.
                        // Store this node so we deal with it the next call.
                        self.next = Some(next);
                        flush_lit_buf(self)
                    } else {
                        // We didn't; just yield this node.
                        Some(bytes_regex_parsed(next))
                    };
                }
            }
        }

        // Flush out any accumulated literal from before.
        if !self.buf.is_empty() {
            flush_lit_buf(self)
        } else {
            self.next.take().map(bytes_regex_parsed)
        }
    }
}

fn to_range(kind: RepetitionKind) -> Result<SizeRange, Error> {
    Ok(match kind {
        ZeroOrOne => size_range(0..=1),
        ZeroOrMore => size_range(0..=32),
        OneOrMore => size_range(1..=32),
        Range(range) => match range {
            Exactly(count) if u32::MAX == count => {
                return unsupported(
                    "Cannot have repetition of exactly u32::MAX",
                )
            }
            Exactly(count) => size_range(count as usize),
            AtLeast(min) => {
                let max = if min < u32::MAX as u32 / 2 {
                    min as usize * 2
                } else {
                    u32::MAX as usize
                };
                size_range((min as usize)..max)
            }
            Bounded(_, max) if u32::MAX == max => {
                return unsupported("Cannot have repetition max of u32::MAX")
            }
            Bounded(min, max) => size_range((min as usize)..(max as usize + 1)),
        },
    })
}

fn to_bytes(khar: char) -> Vec<u8> {
    let mut buf = [0u8; 4];
    khar.encode_utf8(&mut buf).as_bytes().to_owned()
}

fn regex_to_hir(pattern: &str) -> Result<Hir, Error> {
    Ok(Parser::new().parse(pattern)?)
}

fn unsupported<T>(error: &'static str) -> Result<T, Error> {
    Err(Error::UnsupportedRegex(error))
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use regex::Regex;

    use super::*;

    fn do_test(
        pattern: &str,
        min_distinct: usize,
        max_distinct: usize,
        iterations: usize,
    ) {
        let generated = generate_values_matching_regex(pattern, iterations);
        assert!(
            generated.len() >= min_distinct,
            "Expected to generate at least {} strings, but only \
             generated {}",
            min_distinct,
            generated.len()
        );
        assert!(
            generated.len() <= max_distinct,
            "Expected to generate at most {} strings, but \
             generated {}",
            max_distinct,
            generated.len()
        );
    }

    fn generate_values_matching_regex(
        pattern: &str,
        iterations: usize,
    ) -> HashSet<String> {
        let rx = Regex::new(pattern).unwrap();
        let mut generated = HashSet::new();

        let strategy = string_regex(pattern).unwrap();
        let mut runner = TestRunner::deterministic();
        for _ in 0..iterations {
            let mut value = strategy.new_tree(&mut runner).unwrap();

            loop {
                let s = value.current();
                let ok = if let Some(matsch) = rx.find(&s) {
                    0 == matsch.start() && s.len() == matsch.end()
                } else {
                    false
                };
                if !ok {
                    panic!(
                        "Generated string {:?} which does not match {:?}",
                        s, pattern
                    );
                }

                generated.insert(s);

                if !value.simplify() {
                    break;
                }
            }
        }
        generated
    }

    #[test]
    fn test_case_insensitive_produces_all_available_values() {
        let mut expected: HashSet<String> = HashSet::new();
        expected.insert("a".into());
        expected.insert("b".into());
        expected.insert("A".into());
        expected.insert("B".into());
        assert_eq!(generate_values_matching_regex("(?i:a|B)", 64), expected);
    }

    #[test]
    fn test_literal() {
        do_test("foo", 1, 1, 8);
    }

    #[test]
    fn test_casei_literal() {
        do_test("(?i:fOo)", 8, 8, 64);
    }

    #[test]
    fn test_alternation() {
        do_test("foo|bar|baz", 3, 3, 16);
    }

    #[test]
    fn test_repitition() {
        do_test("a{0,8}", 9, 9, 64);
    }

    #[test]
    fn test_question() {
        do_test("a?", 2, 2, 16);
    }

    #[test]
    fn test_star() {
        do_test("a*", 33, 33, 256);
    }

    #[test]
    fn test_plus() {
        do_test("a+", 32, 32, 256);
    }

    #[test]
    fn test_n_to_range() {
        do_test("a{4,}", 4, 4, 64);
    }

    #[test]
    fn test_concatenation() {
        do_test("(foo|bar)(xyzzy|plugh)", 4, 4, 32);
    }

    #[test]
    fn test_ascii_class() {
        do_test("[[:digit:]]", 10, 10, 256);
    }

    #[test]
    fn test_unicode_class() {
        do_test("\\p{Greek}", 24, 512, 256);
    }

    #[test]
    fn test_dot() {
        do_test(".", 200, 65536, 256);
    }

    #[test]
    fn test_dot_s() {
        do_test("(?s).", 200, 65536, 256);
    }

    #[test]
    fn test_backslash_d_plus() {
        do_test("\\d+", 1, 65536, 256);
    }

    fn assert_send_and_sync<T: Send + Sync>(_: T) {}

    #[test]
    fn regex_strategy_is_send_and_sync() {
        assert_send_and_sync(string_regex(".").unwrap());
    }

    macro_rules! consistent {
        ($name:ident, $value:expr) => {
            #[test]
            fn $name() {
                test_generates_matching_strings($value);
            }
        };
    }

    fn test_generates_matching_strings(pattern: &str) {
        use std::time;

        let mut runner = TestRunner::default();
        let start = time::Instant::now();

        // If we don't support this regex, just move on quietly
        if let Ok(strategy) = string_regex(pattern) {
            let rx = Regex::new(pattern).unwrap();

            for _ in 0..1000 {
                let mut val = strategy.new_tree(&mut runner).unwrap();
                // No more than 1000 simplify steps to keep test time down
                for _ in 0..1000 {
                    let s = val.current();
                    assert!(
                        rx.is_match(&s),
                        "Produced string {:?}, which does not match {:?}",
                        s,
                        pattern
                    );

                    if !val.simplify() {
                        break;
                    }
                }

                // Quietly stop testing if we've run for >10 s
                if start.elapsed().as_secs() > 10 {
                    break;
                }
            }
        }
    }

    include!("regex-contrib/crates_regex.rs");
}
