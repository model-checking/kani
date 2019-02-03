# Transforming Strategies

Suppose you have a function that takes a string which needs to be the
`Display` format of an arbitrary `u32`. A first attempt to providing this
argument might be to use a regular expression, like so:

```rust
use proptest::prelude::*;

fn do_stuff(v: String) {
    let i: u32 = v.parse().unwrap();
    let s = i.to_string();
    assert_eq!(s, v);
}

proptest! {
    #[test]
    fn test_do_stuff(v in "[1-9][0-9]{0,8}") {
        do_stuff(v);
    }
}
# fn main() { test_do_stuff(); }
```

This kind of works, but it has problems. For one, it does not explore the
whole `u32` space. It is possible to write a regular expression that does,
but such an expression is rather long, and also results in a pretty odd
distribution of values. The input also doesn't shrink correctly, since
proptest tries to shrink it in terms of a string rather than an integer.

What you really want to do is generate a `u32` and then pass in its string
representation. One way to do this is to just take `u32` as an input to the
test and then transform it to a string within the test code. This approach
works fine, but isn't reusable or composable. Ideally, we could get a
_strategy_ that does this.

The thing we're looking for is the first strategy _combinator_, `prop_map`.
We need to ensure `Strategy` is in scope to use it.

```rust
// Grab `Strategy`, shorter namespace prefix, and the macros
use proptest::prelude::*;

fn do_stuff(v: String) {
    let i: u32 = v.parse().unwrap();
    let s = i.to_string();
    assert_eq!(s, v);
}

proptest! {
    #[test]
    fn test_do_stuff(v in any::<u32>().prop_map(|v| v.to_string())) {
        do_stuff(v);
    }
}
# fn main() { test_do_stuff(); }
```

Calling `prop_map` on a `Strategy` creates a new strategy which transforms
every generated value using the provided function. Proptest retains the
relationship between the original `Strategy` and the transformed one; as a
result, shrinking occurs in terms of `u32`, even though we're generating a
`String`.

`prop_map` is also the principal way to define strategies for new types,
since most types are simply composed of other, simpler values.

Let's update our code so it takes a more interesting structure.


```rust
use proptest::prelude::*;

#[derive(Clone, Debug)]
struct Order {
  id: String,
  // Some other fields, though the test doesn't do anything with them
  item: String,
  quantity: u32,
}

fn do_stuff(order: Order) {
    let i: u32 = order.id.parse().unwrap();
    let s = i.to_string();
    assert_eq!(s, order.id);
}

proptest! {
    #[test]
    fn test_do_stuff(
        order in
        (any::<u32>().prop_map(|v| v.to_string()),
         "[a-z]*", 1..1000u32).prop_map(
             |(id, item, quantity)| Order { id, item, quantity })
    ) {
        do_stuff(order);
    }
}
# fn main() { test_do_stuff(); }
```

Notice how we were able to take the output from `prop_map` and put it in a
tuple, then call `prop_map` on _that_ tuple to produce yet another value.

But that's quite a mouthful in the argument list. Fortunately, strategies
are normal values, so we can extract it to a function.

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

fn arb_order(max_quantity: u32) -> BoxedStrategy<Order> {
    (any::<u32>().prop_map(|v| v.to_string()),
     "[a-z]*", 1..max_quantity)
    .prop_map(|(id, item, quantity)| Order { id, item, quantity })
    .boxed()
}

proptest! {
    #[test]
    fn test_do_stuff(order in arb_order(1000)) {
        do_stuff(order);
    }
}
# fn main() { test_do_stuff(); }
```

We `boxed()` the strategy in the function since otherwise the type would
not be nameable, and even if it were, it would be very hard to read or
write. Boxing a `Strategy` turns both it and its `ValueTree`s into trait
objects, which both makes the types simpler and can be used to mix
heterogeneous `Strategy` types as long as they produce the same value
types.

The `arb_order()` function is also _parameterised_, which is another
advantage of extracting strategies to separate functions. In this case, if
we have a test that needs an `Order` with no more than a dozen items, we
can simply call `arb_order(12)` rather than needing to write out a whole
new strategy.

We can also use `-> impl Strategy<Value = Order>` instead to avoid the
overhead as in the following example. You should use `-> impl Strategy<..>`
unless you need the dynamic dispatch.

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

fn arb_order(max_quantity: u32) -> impl Strategy<Value = Order> {
    (any::<u32>().prop_map(|v| v.to_string()),
     "[a-z]*", 1..max_quantity)
    .prop_map(|(id, item, quantity)| Order { id, item, quantity })
}

proptest! {
    #[test]
    fn test_do_stuff(order in arb_order(1000)) {
        do_stuff(order);
    }
}

# fn main() { test_do_stuff(); }
```
