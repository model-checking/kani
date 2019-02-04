# Filtering

Sometimes, you have a case where your input values have some sort of
"irregular" constraint on them. For example, an integer needing to be even,
or two values needing to be non-equal.

In general, the ideal solution is to find a way to take a seed value and
then use `prop_map` to transform it into the desired, irregular domain. For
example, to generate even integers, use something like

```rust,no_run
use proptest::prelude::*;
prop_compose! {
    // Generate arbitrary integers up to half the maximum desired value,
    // then multiply them by 2, thus producing only even integers in the
    // desired range.
    fn even_integer(max: i32)(base in 0..max/2) -> i32 { base * 2 }
}
# fn main() { }
```

For the cases where this is not viable, it is possible to filter
strategies. Proptest actually divides filters into two categories:

- "Local" filters apply to a single strategy. If a value is rejected,
  a new value is drawn from that strategy only.

- "Global" filters apply to the whole test case. If the test case is
  rejected, the whole thing is regenerated.

The distinction is somewhat arbitrary, since something like a "global
filter" could be created by just putting a "local filter" around the whole
input strategy. In practise, the distinction is as to what code performs
the rejection.

A local filter is created with the `prop_filter` combinator. Besides a
function indicating whether to accept the value, it also takes a value of
type `&'static str`, `String`, .., which it uses to record where/why the
rejection happened.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn some_test(
      v in (0..1000u32)
        .prop_filter("Values must not divisible by 7 xor 11",
                     |v| !((0 == v % 7) ^ (0 == v % 11)))
    ) {
        assert_eq!(0 == v % 7, 0 == v % 11);
    }
}
# fn main() { some_test(); }
```

Global filtering results when a test itself returns
`Err(TestCaseError::Reject)`. The
[`prop_assume!`](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.prop_assume.html)
macro provides an easy way to do this.

```rust
use proptest::prelude::*;

fn frob(a: i32, b: i32) -> (i32, i32) {
    let d = (a - b).abs();
    (a / d, b / d)
}

proptest! {
    #[test]
    fn test_frob(a in -1000..1000, b in -1000..1000) {
        // Input illegal if a==b.
        // Equivalent to
        // if (a == b) { return Err(TestCaseError::Reject(...)); }
        prop_assume!(a != b);

        let (a2, b2) = frob(a, b);
        assert!(a2.abs() <= a.abs());
        assert!(b2.abs() <= b.abs());
    }
}
# fn main() { test_frob(); }
```

While useful, filtering has a lot of disadvantages:

- Since it is simply rejection sampling, it will slow down generation of test
  cases since values need to be generated additional times to satisfy the
  filter. In the case where a filter always returns false, a test could
  theoretically never generate a result.

- Proptest tracks how many local and global rejections have happened, and
  aborts if they exceed a certain number. This prevents a test taking an
  extremely long time due to rejections, but means not all filters are viable
  in the default configuration. The limits for local and global rejections are
  different; by default, proptest allows a large number of local rejections but
  a fairly small number of global rejections, on the premise that the former
  are cheap but potentially common (having been built into the strategy) but
  the latter are expensive but rare (being an edge case in the particular
  test).

- Shrinking and filtering do not play well together. When shrinking, if a value
  winds up being rejected, there is no pass/fail information to continue
  shrinking properly. Instead, proptest treats such a rejection the same way it
  handles a shrink that results in a passing test: by backing away from
  simplification with a call to `complicate()`. Thus encountering a filter
  rejection during shrinking prevents shrinking from continuing to any simpler
  values, even if there are some that would be accepted by the filter.
