# Generating Enums

The syntax sugar for defining strategies for `enum`s is currently somewhat
limited. Creating such strategies with `prop_compose!` is possible but
generally is not very readable, so in most cases defining the function by
hand is preferable.

The core building block is the [`prop_oneof!`](macro.prop_oneof.html)
macro, in which you list one case for each case in your `enum`. For `enum`s
which have no data, the strategy for each case is
`Just(YourEnum::TheCase)`. Enum cases with data generally require putting
the data in a tuple and then using `prop_map` to map it into the enum case.

Here is a simple example:

```rust,no_run
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum MyEnum {
    SimpleCase,
    CaseWithSingleDatum(u32),
    CaseWithMultipleData(u32, String),
}

fn my_enum_strategy() -> impl Strategy<Value = MyEnum> {
  prop_oneof![
    // For cases without data, `Just` is all you need
    Just(MyEnum::SimpleCase),

    // For cases with data, write a strategy for the interior data, then
    // map into the actual enum case.
    any::<u32>().prop_map(MyEnum::CaseWithSingleDatum),

    (any::<u32>(), ".*").prop_map(
      |(a, b)| MyEnum::CaseWithMultipleData(a, b)),
  ]
}
#
# fn main() { }
```

In general, it is best to list the enum cases in order from "simplest" to
"most complex", since shrinking will shrink down toward items earlier in
the list.

For particularly complex enum cases, it can be helpful to extract the strategy
for that case to a separate strategy. Here,
[`prop_compose!`](https://altsysrq.github.io/rustdoc/proptest/latest/proptest/macro.prop_compose.html)
can be of use.

```rust,no_run
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum MyComplexEnum {
    SimpleCase,
    AnotherSimpleCase,
    ComplexCase {
        product_code: String,
        id: u64,
        chapter: String,
    },
}

prop_compose! {
  fn my_complex_enum_complex_case()(
      product_code in "[0-9A-Z]{10,20}",
      id in 1u64..10000u64,
      chapter in "X{0,2}(V?I{1,3}|IV|IX)",
  ) -> MyComplexEnum {
      MyComplexEnum::ComplexCase { product_code, id, chapter }
  }
}

fn my_enum_strategy() -> BoxedStrategy<MyComplexEnum> {
  prop_oneof![
    Just(MyComplexEnum::SimpleCase),
    Just(MyComplexEnum::AnotherSimpleCase),
    my_complex_enum_complex_case(),
  ].boxed()
}
#
# fn main() { }
```
