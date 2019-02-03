# Configuring the number of tests cases requried

The default number of successful test cases that must execute for a test
as a whole to pass is currently 256. If you are not satisfied with this
and want to run more or fewer, there are a few ways to do this.

The first way is to set the environment-variable `PROPTEST_CASES` to a
value that can be successfully parsed as a `u32`. The value you set to this
variable is now the new default.

Another way is to use `#![proptest_config(expr)]` inside `proptest!` where
`expr : Config`. To only change the number of test cases, you can simply
write:

```rust
use proptest::prelude::*;

fn add(a: i32, b: i32) -> i32 { a + b }

proptest! {
    // The next line modifies the number of tests.
    #![proptest_config(ProptestConfig::with_cases(1000))]
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

Through the same `proptest_config` mechanism you may fine-tune your
configuration through the `Config` type. See its documentation for more
information.
