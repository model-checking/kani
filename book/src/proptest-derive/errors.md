# Error Index

[issue tracker]: https://github.com/altsysrq/proptest

## E0001

[lifetime parameters]: https://doc.rust-lang.org/stable/book/second-edition/ch10-03-lifetime-syntax.html#lifetime-annotations-in-struct-definitions

This error occurs when `#[derive(Arbitrary)]` is used on a type which has any
[lifetime parameters]. For example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo<'a> {
    bar: &'a str,
}
```

[gats]: https://github.com/rust-lang/rust/issues/44265
[issue#9]: https://github.com/AltSysrq/proptest/issues/9

Due to the lack of *[generic associated types (GATs)][gats]* on stable Rust,
it is currently impossible to define a `Strategy` which generates a type
that is lifetime-generic (e.g. `&'a T`). Thus, proptest cannot implement
`Arbitrary` for such types either and therefore you cannot `#[derive(Arbitrary)]`
for such types. Once GATs are available, we will try to lift this restriction.
To follow the progress, consult the [tracking issue][issue#9] on the matter.

## E0002

This error occurs when `#[derive(Arbitrary)]` is used on a `union` type.
An example:

```rust
#[derive(Debug, Arbitrary)]
union IU32 {
    signed: i32,
    unsigned: u32,
}
```

There are two main reasons for the error.

1. It is not possible to `#[derive(Debug)]` on `union` types and manual
   implementations cannot know which variant is valid so there are not
   many valid implementations which are possible.

2. Second, we cannot mechanically tell which variant out of `signed` and
   `unsigned` to generate. While we could allow you to tell the macro,
   with an attribute such as `#[proptest(select)]` on the variant,
   we have opted for a more conservative approach for the time being.
   If you have a use case for `#[derive(Arbitrary)]` on `union` types,
   please reach out on the [issue tracker].

## E0003

This error occurs when `#[derive(Arbitrary)]` is used on a struct which
contains known [uninhabited
types](https://doc.rust-lang.org/nomicon/exotic-sizes.html#empty-types). This
in turn means the struct itself is uninhabited and so it there is no sensible
`Arbitrary` implementation since values of the struct cannot be produced.

A trivial example:

```rust
#[derive(Debug, Arbitrary)]
struct Uninhabited {
    inhabited: u32,
    never: !,
}
```

Because there exist no values assignable to field `never`, it is also
impossible to construct an instance of struct `Uninhabited`.

Proptest's ability to identify uninhabited types is limited. If it does not
recognise a particular type as uninhabited, the type will instead be assumed to
be inhabited and you will instead get an error about the type not implementing
`Arbitrary` trait.

## E0004

This error occurs when `#[derive(Arbitrary)]` is used on an enum with no
variants at all. For example:

```rust
#[derive(Debug, Arbitrary)]
enum Uninhabited {}
```

Such an enum has no values at all, so it does not make sense to provide an
`Arbitrary` implementation for it since no values can be generated.

## E0005

This error occurs if `#[derive(Arbitrary)]` is used on an enum whose variants
are all uninhabited, using the same logic as described for [`E0003`](#e0003).
As a result, the enum itself is totally uninhabited.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Uninhabited {
    Never(!),
    NeverEver(!, !),
}
```

## E0006

This error occurs if `#[derive(Arbitrary)]` is used on an enum where all
inhabited variants are marked with [`#[proptest(skip)]`]. In other words,
proptest is forbidden from generating any of the enum's variants, and thus the
enum itself cannot be generated.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum MyEnum {
    // Ordinarily, proptest would be able to generate either of these variants,
    // but both are forbidden, so in the end proptest isn't allowed to generate
    // anything at all.
    #[proptest(skip)]
    UnitVariant,
    #[proptest(skip)]
    SimpleVariant(u32),
    // This variant is implicitly skipped because proptest knows it is
    // uninhabited.
    Uninhabited(!),
}
```

## E0007

This error happens if an attribute [`#[proptest(strategy = "expr")]`] or
[`#[proptest(value = "expr")]`] is applied to the same item that has
`#[derive(Arbitrary)]`.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(value = "MyStruct(42)")]
struct MyStruct(u32);
```

This is rejected since nothing is being "derived" _per se_. A written out
implementation of `Arbitrary` should be used instead.

## E0008

This error happens if [`#[proptest(skip)]`] is applied to an unskippable item.
For example, struct fields cannot be skipped because Rust requires every field
of a struct to have a value.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct WidgetContainer {
    desired_widget_count: usize,
    #[proptest(skip)]
    widgets: Vec<Widget>,
}
```

In general, the appropriate way to request proptest to not generate a field
value is to use [`#[proptest(value = "expr")]`] to provide a fixed value
yourself. For example, the above code could be properly written as follows:

```rust
#[derive(Debug, Arbitrary)]
struct WidgetContainer {
    desired_widget_count: usize,
    #[proptest(value = "vec![]")] // Always generate an empty widget vec
    widgets: Vec<Widget>,
}
```

## E0009

This error happens if [`#[proptest(weight = <integer>)]`] is applied to an item
where this does not make sense, such as a struct field. For example:

```rust
#[derive(Debug, Arbitrary)]
struct Point {
    x: u32,
    #[proptest(weight = 42)]
    y: u32,
}
```

The `weight` attribute only is sensible where proptest has a choice between
multiple items, i.e., enum variants. In contrast, with struct fields proptest
must provide a value for _every_ field so there is no "this-or-that" choice.

## E0010

This error occurs if [`#[proptest(params = "type")]`] and/or
[`#[proptest(no_params)]`] are set on both an item and its parent.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(params = "String")]
struct Foo {
    #[proptest(no_params)]
    bar: String,
}
```

If the parent item has any explicit parameter configuration, it totally defines
the parameters for the whole `Arbitrary` implementation and the child items
must work with that and cannot specify their own parameters.

## E0011

This error occurs if [`#[proptest(params = "type")]`] is set on a field but no
explicit strategy is configured with [`#[proptest(strategy = "expr")]`] or
another such modifier. For example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(param = "u8")]
    some_string: String,
}
```

This example illustrates why both must be specified: `String`'s arbitrary
implementation takes a `proptest::string::StringParam`, but here we try to pass
it a `u8`.

While the generated code could work if the type given by `param` is the same as
that for the default strategy, there would be no purpose in specifying the
parameter type by hand; therefore specifying only `param` is in all cases
forbidden.

## E0012

This error occurs if [`#[proptest(filter = "expr")]`] is set on an item, but the
item containing it specifies a direct way to generate the whole value, which
would thus occur without consulting the filter.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Foo {
    #[proptest(value = "Foo::Bar(42)")]
    Bar {
        #[proptest(filter = "is_even")]
        even_number: u32,
    },
    // ...
}
```

In this example, the entire `Bar` variant specifies how to generate itself
wholesale. As a result, the `filter` clause on `even_number` has no opportunity
to run.

## E0013

This error would occur if an outer attribute of the form `#![proptest(..)]`
were applied to something underneath a `#[derive(Arbitrary)]`.

As of Rust 1.30.0, there are no known ways to produce this error since the Rust
compiler will reject the attribute first.

## E0014

This error occurs if a bare `#[proptest]` attribute is applied to anything,
since it has no meaningful content.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest]
    field: u8,
}
```

The only legal use of the attribute is the form `#[proptest(..)]`.

## E0015

This error occurs if an attribute of the form `#[proptest = value]` is
encountered in any context.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest = 1234]
    field: u8,
}
```

## E0016

This error occurs if a literal (as opposed to `key = value`) is passed inside
`#[proptest(..)]` in any context.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(1234)]
    field: u8,
}
```

## E0017

This error occurs if any modifier of `#[proptest(..)]` is set more than once on
the same item.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(no_params, no_params)]
struct Foo(u32);
```

## E0018

This error occurs if an unknown modifier is passed in `#[proptest(..)]`.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(frobnicate = "true")]
struct Foo(u32);
```

Please see the [modifiers reference](modifiers.md) to see what modifiers are
available.

## E0019

This error happens if anything extra is passed to [`#[proptest(no_params)]`].

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(no_params = "true")]
struct Foo(u32);
```

`no_params` takes no configuration. The correct form is simply
`#[proptest(no_params)]`.

## E0020

This error happens if anything extra is passed to [`#[proptest(skip)]`].

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Foo {
    Small,
    #[proptest(skip = "yes")]
    Huge(ExpensiveType),
}
```

`skip` takes no configuration. The correct form is simply `#[proptest(skip)]`.

## E0021

This error happens if [`#[proptest(weight = <integer>)]`] is passed an invalid
integer or passed nothing at all.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Foo {
    #[proptest(weight)]
    V1,
    #[proptest(weight = heavy)]
    V2,
}
```

The only acceptable form is `#[proptest(weight = <integer>)]`, where
`<integer>` is either an integer literal which fits in a `u32` or the same but
enclosed in quotation marks.

## E0022

This error occurs if more than one of [`#[proptest(no_params)]`] and
[`#[proptest(params = "type")]`] are applied to the same item.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(no_params, params = "u8")]
struct Foo(u32);
```

One attribute or the other must be picked depending on desired effect.

## E0023

This error happens if an invalid [`#[proptest(params = "type")]`] attribute is
applied to an item.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(params = "Vec<u8")] // Note missing '>'
struct Foo(u32);
```

There are a few different ways to get this error:

- Pass nothing at all. E.g., `#[proptest(params)]`.

- Pass something other than a string as the value. E.g.,
  `#[proptest(params = 42)]`.

- Pass a malformed type in the string, as in the example above. (See also
  [caveat on syntax](#valid-rust-syntax).)

## E0024

This error happens if an invalid `#[proptest ..]` attribute is applied using a
syntax the `proptest-derive` crate is not prepared to handle.

Exactly what conditions can produce this error vary by Rust version.

## E0025

This error happens if more than one of [`#[proptest(strategy = "expr")]`],
[`#[proptest(value = "expr")]`], or [`#[proptest(regex = "string")]`] are applied
to the same item.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(value = "42", strategy = "Just(56)")]
    bar: u32,
}
```

Each of these modifiers completely describe how to generate the value, so they
cannot both be applied to the same thing. One or the other must be chosen
depending on the desired effect.

## E0026

This error happens if an invalid form of [`#[proptest(strategy = "expr")]`] or
[`#[proptest(value = "expr")]`] is used.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(value = "3↑↑↑↑3")] // String content is not valid Rust syntax
    g1: u128,
}
```

There are a few different ways to get this error:

- Pass nothing at all. E.g., `#[proptest(value)]`.

- Use another illegal form. E.g., `#[proptest(value("a", "b"))]`.

- Pass a string expression which is not valid Rust syntax, as in the above
  example. (See also [caveat on syntax](#valid-rust-syntax).)

## E0027

This error happens if an invalid form of [`#[proptest(filter = "expr")]`] is
used.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(filter = "> 3")] // String content is not an expression
    big_number: u128,
}
```

There are a few different ways to get this error:

- Pass nothing at all. E.g., `#[proptest(filter)]`.

- Use another illegal form. E.g., `#[proptest(filter("a", "b"))]`.

- Pass a string expression which is not valid Rust syntax, as in the above
  example. (See also [caveat on syntax](#valid-rust-syntax).)

## E0028

This error occurs if a modifier which implies a value is to be generated is
applied to an enum variant which is also marked [`#[proptest(skip)]`].

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Enum {
    V1(u32),
    #[proptest(skip, value = "Enum::V2(42)")]
    V2(u32),
}
```

Here, the [`#[proptest(value = "expr")]`] modifier suggests the user intends
some value to be generated for the enum variant, but at the same time
[`#[proptest(skip)]`] indicates not to generate that variant.

## E0029

This error happens if a modifier which would constrain or control how the value
of an enum variant is to be generated is applied to a unit variant.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Foo {
    #[proptest(value = "Foo::V1")]
    UnitVariant,
    // ...
}
```

Unit variants only have one possible value, so there is only one possible
strategy. As a result, it is pointless to try to specify an alternate strategy
or to filter such variants.

## E0030

This error happens if a modifier which would constrain or control how the value
of a struct is to be generated is applied to a unit struct.

Example:

```rust
#[derive(Debug, Arbitrary)]
#[proptest(params = "u8")]
struct UnitStruct;
```

Unit structs only have one possible value, so there is only one possible
strategy. As a result, it is pointless to try to specify an alternate strategy
or to filter such structs.

## E0031

This error occurs if [`#[proptest(no_bound)]`] is applied to something that is
not a type variable.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo {
    #[proptest(no_bound)]
    bar: u32,
}
```

The `no_bound` modifier only makes sense on generic type variables, as in

```rust
#[derive(Debug, Arbitrary)]
struct Foo<#[proptest(no_bound)] T> {
    #[proptest(value = "None")]
    bar: Option<T>,
}
```

## E0032

This error happens if [`#[proptest(no_bound)]`] is passed anything.

Example:

```rust
#[derive(Debug, Arbitrary)]
struct Foo<#[proptest(no_bound = "yes")] T> {
    _bar: PhantomData<T>,
}
```

The only valid form for the modifier is `#[proptest(no_bound)]`.

## E0033

This error occurs if the sum of the weights on the variants of an enum overflow
a `u32`.

Example:

```rust
#[derive(Debug, Arbitrary)]
enum Foo {
    #[proptest(weight = 3_000_000_000)]
    ThreeFifths,
    #[proptest(weight = 2_000_000_000)]
    TwoFifths,
}
```

The only solution is to reduce the magnitude of the weights so that their sum
fits in a `u32`. Keep in mind that variants without a `weight` modifier still
effectively have `#[proptest(weight = 1)]`.

## E0034

This error occurs if [`#[proptest(regex = "string")]`] is used with invalid
syntax.

The most common forms are `#[proptest(regex = "string-regex")]` and
`#[proptest(regex("string-regex"))]`.

## E0035

This error occurs if both [`#[proptest(regex = "string")]`] and
[`#[proptest(params = "type")]`] are applied to the same item.

Values generated via regular expression take no parameters so the `params`
modifier would be meaningless.

## "Valid Rust syntax"

The definition of "valid Rust syntax" in various string modifiers is determined
by the `syn` crate. If valid syntax is rejected, you can work around it in a
couple ways depending on what the syntax is describing:

For types, simply define a type alias for the type in question. For example,

```rust
type RetroBox = ~str; // N.B. "~str" is not valid Rust 1.30 syntax

//...
#[derive(Debug, Arbitrary)]
#[proptest(params = "RetroBox")]
struct MyStruct { /* ... */ }
```

For values, you can generally factor the code into a constant or function. For
example,

```rust
// N.B. Rust 1.30 does not have an exponentiation operator.
const PI_SQUARED: f64 = PI ** 2.0;

//...
#[derive(Debug, Arbitrary)]
struct MyStruct {
    #[proptest(value = "PI_SQUARED")]
    factor: f64,
}
```

If you need to implement such a work around, consider also [filing an
issue](https://github.com/altsysrq/proptest/issues).

[`#[proptest(filter = "expr")]`]: modifiers.md#filter
[`#[proptest(no_bound)]`]: modifiers.md#no_bound
[`#[proptest(no_params)]`]: modifiers.md#no_params
[`#[proptest(params = "type")]`]: modifiers.md#params
[`#[proptest(regex = "string")]`]: modifiers.md#regex
[`#[proptest(skip)]`]: modifiers.md#skip
[`#[proptest(strategy = "expr")]`]: modifiers.md#strategy
[`#[proptest(value = "expr")]`]: modifiers.md#value
[`#[proptest(weight = <integer>)]`]: modifiers.md#weight
