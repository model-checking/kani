# Syntax Sugar: `prop_compose!`

Defining strategy-returning functions like this is extremely useful, but
the code above is a bit verbose, as well as hard to read for similar
reasons to writing test functions by hand.

To simplify this task, proptest includes the
[`prop_compose!`](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.prop_compose.html)
macro. Before going into details, here's our code from above rewritten to use
it.

```rust
use proptest::prelude::*;

// snip
#
# #[derive(Clone, Debug)]
# struct Order {
#   id: String,
#   // Some other fields, though the test doesn't do anything with them
#   item: String,
#   quantity: u32,
# }
#
# fn do_stuff(order: Order) {
#     let i: u32 = order.id.parse().unwrap();
#     let s = i.to_string();
#     assert_eq!(s, order.id);
# }

prop_compose! {
    fn arb_order_id()(id in any::<u32>()) -> String {
        id.to_string()
    }
}
prop_compose! {
    fn arb_order(max_quantity: u32)
                (id in arb_order_id(), item in "[a-z]*",
                 quantity in 1..max_quantity)
                -> Order {
        Order { id, item, quantity }
    }
}

proptest! {
    # /*
    #[test]
    # */
    fn test_do_stuff(order in arb_order(1000)) {
        do_stuff(order);
    }
}
# fn main() { test_do_stuff(); }
```

We had to extract `arb_order_id()` out into its own function, but otherwise
this desugars to almost exactly what we wrote in the previous section. The
generated function takes the first parameter list as arguments. These
arguments are used to select the strategies in the second argument list.
Values are then drawn from those strategies and transformed by the function
body. The actual function has a return type of `impl Strategy<Value = T>`
where `T` is the declared return type.
