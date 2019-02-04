# Modifier Reference

All modifiers interpreted by `#[derive(Arbitrary)]` are of the form
`#[proptest(..)]`, where the content between the parentheses follows the normal
Rust attribute syntax.

Each modifier within the parentheses is independent, in that putting two
modifiers in the same attribute is equivalent to having two `#[proptest(..)]`
attributes with one modifier each.

For brevity, modifiers are sometimes referenced by name alone; e.g., "the
`weight` modifier" refers to `#[proptest(weight = nn)]` and not some
freestanding `#[weight]` attribute.

## `filter`

Form: `#[proptest(filter = F)]` or `#[proptest(filter(F))]` where `F` is either
a bare identifier (i.e., naming a function) or a Rust expression in a string.
In either case, the parameter must evaluate to something which is `Fn (&T) ->
bool`, where `T` is the type of the item being filtered.

Usable on: structs, enums, enum variants, fields

The `filter` modifier allows filtering values generated for a field via
rejection sampling. Since rejection sampling is inefficient and interferes with
shrinking, it should only be used for conditions that are very rare or are
unfeasible to express otherwise. In many cases, [`strategy`](#strategy) can be
used to more directly express the desired behaviour without rejection sampling.
See the documentation for [`prop_filter`] for more details.

The argument to the modifier must be a valid argument for the second parameter
of [`prop_filter`].

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(filter = "|segment| segment.start != segment.end")]
struct NonEmptySegment {
    start: i32,
    end: i32,
}

// Equivalent to the above
fn is_nonempty(segment: &NonEmptySegment) -> bool {
    segment.start != segment.end
}

#[derive(Debug, Arbitrary)]
#[proptest(filter = is_nomempty)]
struct NonEmptySegment {
    start: i32,
    end: i32,
}
```

As mentioned above, filtering should be avoided when it is reasonably possible
to express a non-filtering strategy that achieves the same effect. For example:

```rust
#[derive(Debug, Arbitrary)]
struct BadExample {
    // Don't do this! Your tests will run more slowly and shrinking won't work
    // properly.
    #[proptest(filter = "|x| x % 2 == 0")]
    even_number: u32,
}

#[derive(Debug, Arbitrary)]
struct GoodExample {
    // Directly generate even numbers only by transforming the set of all
    // `u32`s and then mapping it to the set of even `u32`s.
    #[proptest(strategy = "any::<u32>().prop_map(|x| x / 2 * 2)")]
    even_number: u32,
}
```

[`prop_filter`]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/strategy/trait.Strategy.html#method.prop_filter

## `no_bound`

Form: `#[proptest(no_bound)]`

Usable on: generic type definitions and type parameters

Normally, when `#[derive(Arbitrary)]` is applied to an item with generic type
parameter, every type parameter which is "used" (see below) is required to
`impl Arbitrary`. For example, given a declaration like the following:

```rust
#[derive(Arbitrary)]
struct MyStruct<T> { /* ... */ }
```

Something like this will be generated:

```rust
impl<T> Arbitrary for MyStruct<T> where T: Arbitrary { /* ... */ }
```

Placing `#[proptest(no_bound)]` on a generic type parameter suppresses this. For
example, the following removes the extra `where T: Arbitrary`:

```rust
#[derive(Arbitrary)]
struct MyStruct<#[proptest(no_bound)] T> { /* ... */ }
```

Placing `#[proptest(no_bound)]` on a generic type definition is equivalent to
placing the same attribute on every type parameter.

```rust
#[derive(Arbitrary)]
#[proptest(no_bound)]
struct MyStruct<A, B, C> { /* ... */ }

// Equivalent to
#[derive[Arbitrary)]
struct MyStruct<
  #[proptest(no_bound)] A,
  #[proptest(no_bound)] B,
  #[proptest(no_bound)] C,
> { /* ... */ }
```

A type parameter is "used" if the following hold:

- The enum or struct definition references it at least once, and that reference
  is not inside the type argument of a `PhantomData`.

- The item referencing the type parameter does not have any proptest modifiers
  which replace the usual use of `Arbitrary`, such as [`skip`](#skip) or
  [`value`](#value).

Due to the above, `#[proptest(no_bound)]` is generally only needed when the
type parameter is used in another type which does not itself have an
`Arbitrary` mound on the type.

## `no_params`

Form: `#[proptest(no_params)]`

Usable on: structs, enums, enum variants, fields

On a struct or enum, `no_params` causes the `Arbitrary` parameter type to be
`()`. All automatic delegations to `Arbitrary` on members of the item use
`Default::default()` for their parameters.

On an enum variant or field, suppresses the addition of any parameter for the
variant or field to the parameters for the whole struct. If the variant or
field automatically delegates to `Arbitrary` for its value, that `Arbitrary`
call uses `Default::default()` for its own parameter.

See the [`param` modifier](#param) for more information on how parameters work.

## `params`

Form: `#[proptest(params = T)]` or `#[proptest(params(T))]`, where `T` is
either a bare identifier or Rust code inside a string. In either case, the
value must name a concrete Rust type which implements `Default`.

Usable on: structs, enums, enum variants, fields

The [`Arbitrary` trait] specifies a `Parameters` type which is used to control
generation. By default, the `Parameters` type is a tuple of the parameters
which are automatically passed to other `Arbitrary` implementations.

If applied to a struct or enum, `params` completely replaces the `Parameters`
type. Any automatic delegations to other `Arbitrary` implementations then use
`Default::default()` as there is no automatic way to locate an appropriate
value (if there even is any) within the `params` type.

If applied to an enum variant or field, `params` specifies the parameters type
for just that item, as if its type had an `Arbitrary` implementation taking
that type. In this case, either [`value`](#value) or [`strategy`](#strategy)
_must_ be specified since the parameter type will not generally be compatible
with the normal `Arbitrary` invocation (and in cases where it is, `params`
would be useless if not used).

Any expressions (such as in the [`value`](#value) and [`strategy`](#strategy)
modifiers) underneath an item with the `params` modifier has access to a
variable named `params` which is of the type passed in
`#[proptest(params = ..)]`.

Examples:

```rust
#[derive(Debug)]
struct WidgetRange(usize, usize);

impl Default for WidgetRange {
    fn default() -> Self { Self(0, 100) }
}

#[derive(Debug, Arbitrary)]
#[proptest(params(WidgetRange))]
struct WidgetCollection {
    #[proptest(strategy = "params.0 ..= params.1")]
    desired_widget_count: usize,
    // ...
}

// ...

proptest! {
    #[test]
    fn test_something(wc in any_with::<WidgetCollection>(WidgetRange(10, 20))) {
        assert!(wc.desired_widget_count >= 10 && wc.desired_widget_count <= 20);
    }
}
```

[`Arbitrary` trait]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/arbitrary/trait.Arbitrary.html

## `regex`

Form: `#[proptest(regex = "string")]` or `#[proptest(regex("string"))]`, where
`string` is a regular expression. May also be invoked as
`#[proptest(regex(function_name))]`, where `function_name` is a no-argument
function that returns an `&'static str`.

Usable on: fields

This modifier specifies to generate character or byte strings for a field which
match a particular regular expression.

The `regex` modifier is equivalent to using the [`strategy`](#strategy) modifier and
enclosing the string in [`string_regex`] or [`bytes_regex`]. It can only be
applied to fields of type `String` or `Vec<u8>`.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct FileContent {
    #[proptest(regex = "[a-z0-9.]+")]
    name: String,
    #[proptest(regex = "([0-9]+\n)*")]
    content: Vec<u8>,
}
```

[`string_regex`]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/string/fn.string_regex.html
[`bytes_regex`]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/string/fn.bytes_regex.html

## `skip`

Form: `#[proptest(skip)]`

Usable on: enum variants

Annotating an enum variant with `#[proptest(skip)]` prevents proptest from
generating that particular variant. This is useful when there is no sensible
way to generate the variant or when you want to temporarily stop generating
some variant during development.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum DataSource {
    Memory(Vec<u8>),

    // There's no way to produce an "arbitrary" file handle, so we skip
    // generating this case.
    #[proptest(skip)]
    File(std::fs::File),
}
```

It is an error to annotate all inhabited variants of an enum with
`#[proptest(skip)]` as this leaves proptest with no options to generate the
enum.

## `strategy`

Form: `#[proptest(strategy = S)]` or `#[proptest(strategy = S)]`, where `S` is
either a string containing a Rust expression which evaluates to an appropriate
`Strategy`, or a bare identifier naming a function which, when called with no
arguments, returns such a `Strategy`.

Usable on: enum variants, fields

By default, enum variants are generated by recursing into their definition as
is done for struct declarations, and fields are generated by invoking
`Arbitrary` on the field type to produce a `Strategy`. The `strategy` modifier
allows to manually provide a custom strategy directly.

In the case of fields, the strategy must produce values of the same type as
that field. For enum variants, it must produce values of the enum type itself
and these values ought to be of the variant in question.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Token {
    Delimitation {
        // This field is still generated via Arbitrary
        delimiter: Delimiter,

        // But for this field we use a custom strategy
        #[proptest(strategy = "1..10")]
        count: u32,

        // Here we also use a custom strategy, generated by the function
        // `offset_strategy`.
        #[proptest(strategy = offset_strategy)]
        offset: u32,
    },

    // Specify how to generate the whole enum variant
    #[proptest(strategy = "\"[a-zA-Z]+\".prop_map(Token::Word)")]
    Word(String),
}

#[derive(Debug, Arbitrary)]
enum Delimiter { /* ... */ }

fn offset_strategy() -> impl Strategy<Value = u32> {
  0..100
}
```

## `value`

Form: `#[proptest(value = V)]` or `#[proptest(value(V))]`, where V can be: (a)
a Rust expression enclosed in a string; (b) another literal, or (c) a bare
identifier naming a no-argument function.

Usable on: enum variants, fields

The `value` modifier indicates that proptest should use the given expression or
function to produce a value for the field, instead of going through the usual
value generation machinery.

The argument to `value` is directly used as an expression for the field value
or enum variant to be generated, except that in the third form where it is a
bare identifier, it is called as a no-argument function to produce the value.

Using `value` is equivalent to using [`strategy`](#strategy) and enclosing the
value in `LazyJust`.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct EventCounter {
    // We always start with the first two fields set to 0/None
    #[proptest(value = 0)]
    number_seen: u64,

    #[proptest(value = "None")]
    last_seen_time: Option<Instant>,

    // This field is generated normally
    max_events: u64,
}
```

## `weight`

Form: `#[proptest(weight = W)]` or `#[proptest(weight(W))]`, where `W` is an
expression evaluating to a `u32`. `weight` may also be abbreviated to `w`, as
in `#[proptest(w = W)]`.

Usable on: enum variants

The `weight` modifier determines how likely proptest is to generate a
particular enum variant. Weights are relative to each other; for example, a
`weight = 3` variant is 50% more likely to be generated than a `weight = 2`
variant and three times as likely to be generated as a `weight = 1` variant.

Variants with no `weight` modifier are equivalent to being annotated
`#[proptest(weight = 1)]`.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum FilterOption {
    KeepAll,
    DiscardAll,

    // This option is presumably harder for the code to handle correctly,
    // so we generate it more frequently than the other options.
    #[proptest(weight = 3)]
    OnlyMatching(String),
}
```
