# Syntax Sugar: `proptest!`

Now that we know about compound strategies, we can understand how the
[`proptest!`](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.proptest.html)
macro works. Our example from the prior section can be rewritten using that
macro like so:

```rust
use proptest::prelude::*;

fn add(a: i32, b: i32) -> i32 {
    a + b
}

proptest! {
    #[test]
    fn test_add(a in 0..1000i32, b in 0..1000i32) {
        let sum = add(a, b);
        assert!(sum >= a);
        assert!(sum >= b);
    }
}
#
# fn main() { test_add(); }
```

Conceptually, the desugaring process is fairly simple. At the start of the
test function, a new `TestRunner` is constructed. The input strategies
(after the `in` keyword) are grouped into a tuple. That tuple is passed in
to the `TestRunner` as the input strategy. The test body has `Ok(())` added
to the end, then is put into a lambda that destructures the generated input
tuple back into the named parameters and then runs the body. The end result
is extremely similar to what we wrote by hand in the prior section.

`proptest!` actually does a few other things in order to make failure
output easier to read and to overcome the 10-tuple limit.
