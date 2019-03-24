# Getting Started

Let's say we want to make a function that parses dates of the form
`YYYY-MM-DD`. We're not going to worry about _validating_ the date, any
triple of integers is fine. So let's bang something out real quick.

```rust,no_run
fn parse_date(s: &str) -> Option<(u32, u32, u32)> {
    if 10 != s.len() { return None; }
    if "-" != &s[4..5] || "-" != &s[7..8] { return None; }

    let year = &s[0..4];
    let month = &s[6..7];
    let day = &s[8..10];

    year.parse::<u32>().ok().and_then(
        |y| month.parse::<u32>().ok().and_then(
            |m| day.parse::<u32>().ok().map(
                |d| (y, m, d))))
}
```

It compiles, that means it works, right? Maybe not, let's add some tests.

```rust,ignore
#[test]
fn test_parse_date() {
    assert_eq!(None, parse_date("2017-06-1"));
    assert_eq!(None, parse_date("2017-06-170"));
    assert_eq!(None, parse_date("2017006-17"));
    assert_eq!(None, parse_date("2017-06017"));
    assert_eq!(Some((2017, 06, 17)), parse_date("2017-06-17"));
}
```

Tests pass, deploy to production! But now your application starts crashing,
and people are upset that you moved Christmas to February. Maybe we need to
be a bit more thorough.

In `Cargo.toml`, add

```toml
[dev-dependencies]
proptest = "0.9.2"
```

Now we can add some property tests to our date parser. But how do we test
the date parser for arbitrary inputs, without making another date parser in
the test to validate it? We won't need to as long as we choose our inputs
and properties correctly. But before correctness, there's actually an even
simpler property to test: _The function should not crash._ Let's start
there.

```rust,ignore
// Bring the macros and other important things into scope.
use proptest::prelude::*;

proptest! {
    #[test]
    fn doesnt_crash(s in "\\PC*") {
        parse_date(&s);
    }
}
```

What this does is take a literally random `&String` (ignore `\\PC*` for the
moment, we'll get back to that — if you've already figured it out, contain
your excitement for a bit) and give it to `parse_date()` and then throw the
output away.

When we run this, we get a bunch of scary-looking output, eventually ending
with

```text
thread 'main' panicked at 'Test failed: byte index 4 is not a char boundary; it is inside 'ௗ' (bytes 2..5) of `aAௗ0㌀0`; minimal failing input: s = "aAௗ0㌀0"
	successes: 102
	local rejects: 0
	global rejects: 0
'
```

If we look at the top directory after the test fails, we'll see a new
`proptest-regressions` directory, which contains some files corresponding
to source files containing failing test cases. These are [_failure
persistence_](#failure-persistence) files. The first thing we should do is
add these to source control.

```text
$ git add proptest-regressions
```

The next thing we should do is copy the failing case to a traditional unit
test since it has exposed a bug not similar to what we've tested in the
past.

```rust,ignore
#[test]
fn test_unicode_gibberish() {
    assert_eq!(None, parse_date("aAௗ0㌀0"));
}
```

Now, let's see what happened... we forgot about UTF-8! You can't just
blindly slice strings since you could split a character, in this case that
Tamil diacritic placed atop other characters in the string.

In the interest of making the code changes as small as possible, we'll just
check that the string is ASCII and reject anything that isn't.

```rust,no_run
# use std::ascii::AsciiExt; //NOREADME
# // NOREADME
fn parse_date(s: &str) -> Option<(u32, u32, u32)> {
    if 10 != s.len() { return None; }

    // NEW: Ignore non-ASCII strings so we don't need to deal with Unicode.
    if !s.is_ascii() { return None; }

    if "-" != &s[4..5] || "-" != &s[7..8] { return None; }

    let year = &s[0..4];
    let month = &s[6..7];
    let day = &s[8..10];

    year.parse::<u32>().ok().and_then(
        |y| month.parse::<u32>().ok().and_then(
            |m| day.parse::<u32>().ok().map(
                |d| (y, m, d))))
}
```

The tests pass now! But we know there are still more problems, so let's
test more properties.

Another property we want from our code is that it parses every valid date.
We can add another test to the `proptest!` section:

```rust,ignore
proptest! {
    // snip...

    #[test]
    fn parses_all_valid_dates(s in "[0-9]{4}-[0-9]{2}-[0-9]{2}") {
        parse_date(&s).unwrap();
    }
}
```

The thing to the right-hand side of `in` is actually a *regular
expression*, and `s` is chosen from strings which match it. So in our
previous test, `"\\PC*"` was generating arbitrary strings composed of
arbitrary non-control characters. Now, we generate things in the YYYY-MM-DD
format.

The new test passes, so let's move on to something else.

The final property we want to check is that the dates are actually parsed
_correctly_. Now, we can't do this by generating strings — we'd end up just
reimplementing the date parser in the test! Instead, we start from the
expected output, generate the string, and check that it gets parsed back.

```rust,ignore
proptest! {
    // snip...

    #[test]
    fn parses_date_back_to_original(y in 0u32..10000,
                                    m in 1u32..13, d in 1u32..32) {
        let (y2, m2, d2) = parse_date(
            &format!("{:04}-{:02}-{:02}", y, m, d)).unwrap();
        // prop_assert_eq! is basically the same as assert_eq!, but doesn't
        // cause a bunch of panic messages to be printed on intermediate
        // test failures. Which one to use is largely a matter of taste.
        prop_assert_eq!((y, m, d), (y2, m2, d2));
    }
}
```

Here, we see that besides regexes, we can use any expression which is a
`proptest::strategy::Strategy`, in this case, integer ranges.

The test fails when we run it. Though there's not much output this time.

```text
thread 'main' panicked at 'Test failed: assertion failed: `(left == right)` (left: `(0, 10, 1)`, right: `(0, 0, 1)`) at examples/dateparser_v2.rs:46; minimal failing input: y = 0, m = 10, d = 1
	successes: 2
	local rejects: 0
	global rejects: 0
', examples/dateparser_v2.rs:33
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

The failing input is `(y, m, d) = (0, 10, 1)`, which is a rather specific
output. Before thinking about why this breaks the code, let's look at what
proptest did to arrive at this value. At the start of our test function,
insert

```rust,ignore
    println!("y = {}, m = {}, d = {}", y, m, d);
```

Running the test again, we get something like this:

```text
y = 2497, m = 8, d = 27
y = 9641, m = 8, d = 18
y = 7360, m = 12, d = 20
y = 3680, m = 12, d = 20
y = 1840, m = 12, d = 20
y = 920, m = 12, d = 20
y = 460, m = 12, d = 20
y = 230, m = 12, d = 20
y = 115, m = 12, d = 20
y = 57, m = 12, d = 20
y = 28, m = 12, d = 20
y = 14, m = 12, d = 20
y = 7, m = 12, d = 20
y = 3, m = 12, d = 20
y = 1, m = 12, d = 20
y = 0, m = 12, d = 20
y = 0, m = 6, d = 20
y = 0, m = 9, d = 20
y = 0, m = 11, d = 20
y = 0, m = 10, d = 20
y = 0, m = 10, d = 10
y = 0, m = 10, d = 5
y = 0, m = 10, d = 3
y = 0, m = 10, d = 2
y = 0, m = 10, d = 1
```

The test failure message said there were two successful cases; we see these
at the very top, `2497-08-27` and `9641-08-18`. The next case,
`7360-12-20`, failed. There's nothing immediately obviously special about
this date. Fortunately, proptest reduced it to a much simpler case. First,
it rapidly reduced the `y` input to `0` at the beginning, and similarly
reduced the `d` input to the minimum allowable value of `1` at the end.
Between those two, though, we see something different: it tried to shrink
`12` to `6`, but then ended up raising it back up to `10`. This is because
the `0000-06-20` and `0000-09-20` test cases _passed_.

In the end, we get the date `0000-10-01`, which apparently gets parsed as
`0000-00-01`. Again, this failing case was added to the failure persistence
file, and we should add this as its own unit test:

```text
$ git add proptest-regressions
```

```rust,ignore
#[test]
fn test_october_first() {
    assert_eq!(Some((0, 10, 1)), parse_date("0000-10-01"));
}
```

Now to figure out what's broken in the code. Even without the intermediate
input, we can say with reasonable confidence that the year and day parts
don't come into the picture since both were reduced to the minimum
allowable input. The month input was _not_, but was reduced to `10`. This
means we can infer that there's something special about `10` that doesn't
hold for `9`. In this case, that "special something" is being two digits
wide. In our code:

```rust,ignore
    let month = &s[6..7];
```

We were off by one, and need to use the range `5..7`. After fixing this,
the test passes.

The `proptest!` macro has some additional syntax, including for setting
configuration for things like the number of test cases to generate. See its
[documentation](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.proptest.html)
for more details.
