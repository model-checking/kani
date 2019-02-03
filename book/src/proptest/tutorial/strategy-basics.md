# Strategy Basics

Please make sure to read the [introduction to this tutorial](index.md) before
starting this section.

The [_Strategy_](strategy/trait.Strategy.html) is the most fundamental
concept in proptest. A strategy defines two things:

- How to generate random values of a particular type from a random number
generator.

- How to "shrink" such values into "simpler" forms.

Proptest ships with a substantial library of strategies. Some of these are
defined in terms of built-in types; for example, `0..100i32` is a strategy
to generate `i32`s between 0, inclusive, and 100, exclusive. As we've
already seen, strings are themselves strategies for generating strings
which match the former as a regular expression.

Generating a value is a two-step process. First, a `TestRunner` is passed
to the `new_tree()` method of the `Strategy`; this returns a `ValueTree`,
which we'll look at in more detail momentarily. Calling the `current()`
method on the `ValueTree` produces the actual value. Knowing that, we can
put the pieces together and generate values. The below is the
`tutoral-strategy-play.rs` example:

```rust
use proptest::test_runner::TestRunner;
use proptest::strategy::{Strategy, ValueTree};

fn main() {
    let mut runner = TestRunner::default();
    let int_val = (0..100i32).new_tree(&mut runner).unwrap();
    let str_val = "[a-z]{1,4}\\p{Cyrillic}{1,4}\\p{Greek}{1,4}"
        .new_tree(&mut runner).unwrap();
    println!("int_val = {}, str_val = {}",
             int_val.current(), str_val.current());
}
```

If you run this a few times, you'll get output similar to the following:

```text
$ target/debug/examples/tutorial-strategy-play
int_val = 99, str_val = vѨͿἕΌ
$ target/debug/examples/tutorial-strategy-play
int_val = 25, str_val = cwᵸійΉ
$ target/debug/examples/tutorial-strategy-play
int_val = 5, str_val = oegiᴫᵸӈᵸὛΉ
```

This knowledge is sufficient to build an extremely primitive fuzzing test.

```rust,no_run
use proptest::test_runner::TestRunner;
use proptest::strategy::{Strategy, ValueTree};

fn some_function(v: i32) {
    // Do a bunch of stuff, but crash if v > 500
    assert!(v <= 500);
}

#[test]
fn some_function_doesnt_crash() {
    let mut runner = TestRunner::default();
    for _ in 0..256 {
        let val = (0..10000i32).new_tree(&mut runner).unwrap();
        some_function(val.current());
    }
}
# fn main() { }
```

This _works_, but when the test fails, we don't get much context, and even
if we recover the input, we see some arbitrary-looking value like 1771
rather than the boundary condition of 501. For a function taking just an
integer, this is probably still good enough, but as inputs get more
complex, interpreting completely random values becomes increasingly
difficult.
