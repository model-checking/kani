# Generating Recursive Data

Randomly generating recursive data structures is trickier than it sounds. For
example, the below is a naïve attempt at generating a JSON AST by using
recursion. This also uses the
[`prop_oneof!`](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.prop_oneof.html),
which we haven't seen yet but should be self-explanatory.

```rust,no_run
use std::collections::HashMap;
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Map(HashMap<String, Json>),
}

fn arb_json() -> impl Strategy<Value = Json> {
    prop_oneof![
        Just(Json::Null),
        any::<bool>().prop_map(Json::Bool),
        any::<f64>().prop_map(Json::Number),
        ".*".prop_map(Json::String),
        prop::collection::vec(arb_json(), 0..10).prop_map(Json::Array),
        prop::collection::hash_map(
          ".*", arb_json(), 0..10).prop_map(Json::Map),
    ].boxed()
}
# fn main() { }
```

Upon closer consideration, this obviously can't work because `arb_json()`
recurses unconditionally.

A more sophisticated attempt is to define one strategy for each level of
nesting up to some maximum. This doesn't overflow the stack, but as defined
here, even four levels of nesting will produce trees with _thousands_ of
nodes; by eight levels, we get to tens of _millions_.

Proptest provides a more reliable solution in the form of the
`prop_recursive` combinator. To use this, we create a strategy for the
non-recursive case, then give the combinator that strategy, some size
parameters, and a function to transform a nested strategy into a recursive
strategy.

```rust
use std::collections::HashMap;
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Map(HashMap<String, Json>),
}

fn arb_json() -> impl Strategy<Value = Json> {
    let leaf = prop_oneof![
        Just(Json::Null),
        any::<bool>().prop_map(Json::Bool),
        any::<f64>().prop_map(Json::Number),
        ".*".prop_map(Json::String),
    ];
    leaf.prop_recursive(
      8, // 8 levels deep
      256, // Shoot for maximum size of 256 nodes
      10, // We put up to 10 items per collection
      |inner| prop_oneof![
          // Take the inner strategy and make the two recursive cases.
          prop::collection::vec(inner.clone(), 0..10)
              .prop_map(Json::Array),
          prop::collection::hash_map(".*", inner, 0..10)
              .prop_map(Json::Map),
      ])
}
# fn main() { }
```
