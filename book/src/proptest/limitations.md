# Limitations of Property Testing

Given infinite time, property testing will eventually explore the whole
input space to a test. However, time is not infinite, so only a randomly
sampled portion of the input space can be explored. This means that
property testing is extremely unlikely to find single-value edge cases in a
large space. For example, the following test will virtually always pass:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn i64_abs_is_never_negative(a: i64) {
        // This actually fails if a == i64::MIN, but randomly picking one
        // specific value out of 2⁶⁴ is overwhelmingly unlikely.
        assert!(a.abs() >= 0);
    }
}
# // NOREADME
# fn main() { } // NOREADME
```

Because of this, traditional unit testing with intelligently selected cases
is still necessary for many kinds of problems.

Similarly, in some cases it can be hard or impossible to define a strategy
which actually produces useful inputs. A strategy of `.{1,4096}` may be
great to fuzz a C parser, but is highly unlikely to produce anything that
makes it to a code generator.
