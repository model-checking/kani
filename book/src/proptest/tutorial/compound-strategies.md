# Compound Strategies

Testing functions that take single arguments of primitive types is nice and
all, but is kind of underwhelming. Back when we were writing the whole
stack by hand, extending the technique to, say, _two_ integers was clear,
if verbose. But `TestRunner` only takes a single `Strategy`; how can we
test a function that needs inputs from more than one?

```rust,ignore
use proptest::test_runner::TestRunner;

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    let mut runner = TestRunner::default();
    runner.run(/* uhhm... */).unwrap();
}
#
# fn main() { test_add(); }
```

The key is that strategies are _composable_. The simplest form of
composition is "compound strategies", where we take multiple strategies and
combine their values into one value that holds each input separately. There
are several of these. The simplest is a tuple; a tuple of strategies is
itself a strategy for tuples of the values those strategies produce. For
example, `(0..100i32,100..1000i32)` is a strategy for pairs of integers
where the first value is between 0 and 100 and the second is between 100
and 1000.

So for our two-argument function, our strategy is simply a tuple of ranges.

```rust
use proptest::test_runner::TestRunner;

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    let mut runner = TestRunner::default();
    // Combine our two inputs into a strategy for one tuple. Our test
    // function then destructures the generated tuples back into separate
    // `a` and `b` variables to be passed in to `add()`.
    runner.run(&(0..1000i32, 0..1000i32), |(a, b)| {
        let sum = add(a, b);
        assert!(sum >= a);
        assert!(sum >= b);
        Ok(())
    }).unwrap();
}
#
# fn main() { test_add(); }
```

Other compound strategies include fixed-sizes arrays of strategies and
`Vec`s of strategies (which produce arrays or `Vec`s of values parallel to
the strategy collection), as well as the various strategies provided in the
[collection](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/collection/index.html) module.
